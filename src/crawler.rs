//! Filesystem crawling module for Project FRIDA
//!
//! This module is designed to walk the entire filesystem of all connected drives,
//! creating a tree structure of all accessible files and directories. It operates
//! with low CPU overhead to avoid detection and system performance impact.

use serde::{Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use async_recursion::async_recursion;
use std::time::{Duration, UNIX_EPOCH};
use tokio::time::sleep;

use crate::drives::DriveInfo;
use crate::log::Log;

/// Represents a file in the filesystem tree.
#[derive(Serialize, Debug)]
pub struct FileInfo {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub modified: u64, // Unix timestamp
}

/// Represents a directory in the filesystem tree.
#[derive(Serialize, Debug)]
pub struct DirectoryInfo {
    pub name: String,
    pub path: PathBuf,
    pub children: Vec<DirectoryInfo>,
    pub files: Vec<FileInfo>,
}

/// Recursively walks a directory to build a tree structure.
#[async_recursion]
async fn walk_directory(path: &Path) -> Result<DirectoryInfo, std::io::Error> {
    let mut dir_info = DirectoryInfo {
        name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
        path: path.to_path_buf(),
        children: Vec::new(),
        files: Vec::new(),
    };

    let mut read_dir = match tokio::fs::read_dir(path).await {
        Ok(entries) => entries,
        Err(e) => {
            // Log error but don't stop crawling other directories
            return Err(e);
        }
    };

    while let Some(entry) = read_dir.next_entry().await? {
        let entry_path = entry.path();
        if entry_path.is_dir() {
            if let Ok(child_dir) = walk_directory(&entry_path).await {
                dir_info.children.push(child_dir);
            }
        } else if let Ok(metadata) = entry.metadata().await {
            let modified_timestamp = metadata.modified()?
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            dir_info.files.push(FileInfo {
                name: entry.file_name().to_string_lossy().to_string(),
                path: entry_path.clone(),
                size: metadata.len(),
                modified: modified_timestamp,
            });
        }
    }

    Ok(dir_info)
}

/// Crawls all provided drives and saves the filesystem tree to a JSON file.
///
/// # Arguments
/// * `drives` - A slice of `DriveInfo` from the `drives` module.
/// * `output_path` - The path to save the resulting JSON file.
pub async fn crawl_drives(drives: &[DriveInfo], output_path: &str) -> Result<(), std::io::Error> {
    let mut all_drives_tree = Vec::new();

    for drive in drives {
        Log::info(format!("Crawling drive: {}", drive.mount_point));
        let drive_path = Path::new(&drive.mount_point);
        if let Ok(drive_tree) = walk_directory(drive_path).await {
            all_drives_tree.push(drive_tree);
        }
    }

    let json_output = serde_json::to_string_pretty(&all_drives_tree)?;
    fs::write(output_path, json_output).await?;

    Log::info(format!("Filesystem crawl complete. Output saved to {}", output_path));
    Ok(())
}
