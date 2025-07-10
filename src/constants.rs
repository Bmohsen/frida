//! Global constants for Project FRIDA.

// --- Crawler Configuration ---
pub const CRAWLER_OUTPUT_FILENAME: &str = "logs/filesystem_tree.jsonl";
pub const CRAWLER_CPU_THROTTLE_MS: u64 = 1;

// --- Screen Capture ---
pub const SCREENSHOT_OUTPUT_DIR: &str = "logs/screenshots";

// --- Network Stealth ---
pub const STEALTH_CHUNK_SIZE: usize = 4096; // 1KB
