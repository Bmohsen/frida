//! Global constants and path configurations for Project FRIDA.

use crate::paths;
use std::path::PathBuf;

// --- Crawler Configuration ---
pub fn crawler_output_file() -> PathBuf {
    paths::get().data_dir.join("filesystem_tree.jsonl")
}
pub const CRAWLER_CPU_THROTTLE_MS: u64 = 1;
pub const CRAWLER_MAX_ENTRIES_PER_FILE: usize = 10000;

// --- Screen Capture ---
pub fn screenshot_output_dir() -> PathBuf {
    let dir = paths::get().data_dir.join("screenshots");
    // Ensure the directory exists before returning it.
    if !dir.exists() {
        let _ = std::fs::create_dir_all(&dir);
    }
    dir
}

// --- Network Stealth ---
pub const STEALTH_CHUNK_SIZE: usize = 4096; // 4KB
