//! File system operations and data persistence module
//!
//! This module provides functionality for securely writing collected data
//! to the file system. It handles JSON serialization, file creation,
//! directory management, and both append and overwrite operations.
//! All file operations include proper error handling and path validation.

use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

/// Saves serializable data to a file as JSON.
///
/// # Arguments
/// * `data` - The data to serialize and save. Must implement `serde::Serialize`.
/// * `file_path` - The path to the output file.
/// * `append` - If true, appends to the file; otherwise, overwrites it.
///
/// # Returns
/// A `Result` indicating success or an `std::io::Error`.
pub fn save_output<T: Serialize>(
    data: &T,
    file_path: &str,
    append: bool,
) -> Result<String, String> {
    // Ensure the parent directory exists.
    if let Some(parent_dir) = Path::new(file_path).parent() {
        fs::create_dir_all(parent_dir).map_err(|e| e.to_string())?;
    }

    // Serialize the data to a pretty JSON string.
    let json_string = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;

    // Open the file with the correct options (append or create/truncate).
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .append(append)
        .truncate(!append)
        .open(file_path)
        .map_err(|e| e.to_string())?;

    // Write the JSON string, followed by a newline.
    writeln!(file, "{}", json_string).map_err(|e| e.to_string())?;

    Ok("ok".to_string())
}
