//! Filesystem crawling module for Project FRIDA
//!
//! This module is designed to walk the entire filesystem of all connected drives,
//! creating a tree structure of all accessible files and directories. It operates
//! with low CPU overhead to avoid detection and system performance impact.

use serde::{Serialize};
use std::path::{PathBuf};
use std::time::{Duration, UNIX_EPOCH};
use tokio::fs::{self, File};
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::mpsc;
use tokio::time::sleep;
use async_recursion::async_recursion;

use crate::constants;
use crate::drives::DriveInfo;
use crate::log::Log;

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

/// Recursively walks a directory, sending discovered nodes through a channel.
#[async_recursion]
async fn walk_directory(
    path: PathBuf,
    sender: mpsc::Sender<FileSystemNode>,
) -> Result<(), std::io::Error> {
    let mut read_dir = match fs::read_dir(&path).await {
        Ok(entries) => entries,
        Err(e) => {
            Log::error(format!("Could not read directory {}: {}", path.display(), e));
            return Err(e);
        }
    };

    while let Some(entry) = read_dir.next_entry().await? {
        // Throttle CPU usage to keep a low profile.
                sleep(Duration::from_millis(constants::CRAWLER_CPU_THROTTLE_MS)).await;
        let entry_path = entry.path();
        if entry_path.is_dir() {
            let dir_info = DirectoryInfo {
                name: entry.file_name().to_string_lossy().to_string(),
                path: entry_path.clone(),
            };
            if sender.send(FileSystemNode::Directory(dir_info)).await.is_err() {
                // Receiver has been dropped, stop crawling.
                return Ok(());
            }
            // Recursively walk the subdirectory.
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
pub async fn crawl_drives(drives: &[DriveInfo], output_path: &str) -> Result<(), std::io::Error> {
    let (sender, mut receiver) = mpsc::channel(100);
    let output_file = File::create(output_path).await?;
    let mut writer = BufWriter::new(output_file);

    // Writer task: receives nodes and writes them to the file.
    let writer_task = tokio::spawn(async move {
        while let Some(node) = receiver.recv().await {
            if let Ok(json) = serde_json::to_string(&node) {
                if writer.write_all(json.as_bytes()).await.is_err() {
                    break;
                }
                if writer.write_all(b"\n").await.is_err() {
                    break;
                }
            }
        }
        let _ = writer.flush().await;
    });

    // Crawler tasks: walk each drive and send data to the writer.
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

    // Drop the original sender to allow the receiver to close when all crawlers are done.
    drop(sender);

    // Wait for all crawlers to finish.
    for handle in crawl_handles {
        let _ = handle.await;
    }

    // Wait for the writer to finish processing all messages.
    let _ = writer_task.await;

    Log::info(format!("Filesystem crawl complete. Output saved to {}", output_path));
    Ok(())
}
