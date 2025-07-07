//! Hardware device monitoring and surveillance module
//!
//! This module provides continuous monitoring of all USB and peripheral devices
//! connected to the system. It detects new device connections in real-time,
//! captures detailed device metadata (vendor/product IDs, serial numbers),
//! and logs the information for later analysis.

use crate::log::Log;
use crate::writer;
use rusb::{Context, DeviceHandle, UsbContext};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
const DEVICE_LOG_FILE: &str = "logs/devices.log";
const CHECK_INTERVAL_SECONDS: u64 = 5;
struct DeviceInfo {
    vendor_id: u16,
    product_id: u16,
    manufacturer: String,
    product: String,
    serial_number: String,
}
/// Starts a device monitor in a separate thread
pub fn start_device_monitor() {
    Log::info("Device monitor starting...".to_string());
    // Track currently connected devices to detect new ones
    let connected_devices = Arc::new(Mutex::new(Vec::new()));
    thread::spawn(move || {
        loop {
            match detect_usb_devices() {
                Ok(current_devices) => {
                    let mut connected = connected_devices.lock().unwrap();
                    // Find new devices that weren't connected before
                    let new_devices: Vec<_> = current_devices
                        .iter()
                        .filter(|d| {
                            !connected.contains(&(
                                d.vendor_id,
                                d.product_id,
                                d.serial_number.clone(),
                            ))
                        })
                        .collect();
                    if !new_devices.is_empty() {
                        Log::info(format!("Detected {} new device(s)", new_devices.len()));
                        // Log device information
                        for device in &new_devices {
                            let device_info = format!(
                                "New device connected:\n  - Vendor ID: {:04x}\n  - Product ID: {:04x}\n  - Manufacturer: {}\n  - Product: {}\n  - Serial: {}",
                                device.vendor_id,
                                device.product_id,
                                device.manufacturer,
                                device.product,
                                device.serial_number
                            );
                            Log::info(device_info.clone());

                            // Save to file
                            if let Err(e) = writer::save_output(&device_info, DEVICE_LOG_FILE, true)
                            {
                                Log::error(format!("Failed to write device info: {}", e));
                            }
                        }
                    }
                    // Update the list of connected devices
                    *connected = current_devices
                        .iter()
                        .map(|d| (d.vendor_id, d.product_id, d.serial_number.clone()))
                        .collect();
                }
                Err(e) => {
                    Log::error(format!("Error detecting USB devices: {}", e));
                }
            }
            // Sleep before checking again
            thread::sleep(Duration::from_secs(CHECK_INTERVAL_SECONDS));
        }
    });
}

/// Detect all currently connected USB devices
fn detect_usb_devices() -> Result<Vec<DeviceInfo>, String> {
    let context = Context::new().map_err(|e| e.to_string())?;
    let devices = context.devices().map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for device in devices.iter() {
        if let Ok(device_desc) = device.device_descriptor() {
            // Try to get device information
            let handle = match device.open() {
                Ok(h) => h,
                Err(_) => continue, // Skip if we can't open the device
            };

            let manufacturer =
                get_string_descriptor(&handle, device_desc.manufacturer_string_index())
                    .unwrap_or_else(|_| "Unknown".to_string());

            let product = get_string_descriptor(&handle, device_desc.product_string_index())
                .unwrap_or_else(|_| "Unknown".to_string());

            let serial_number =
                get_string_descriptor(&handle, device_desc.serial_number_string_index())
                    .unwrap_or_else(|_| "Unknown".to_string());

            result.push(DeviceInfo {
                vendor_id: device_desc.vendor_id(),
                product_id: device_desc.product_id(),
                manufacturer,
                product,
                serial_number,
            });
        }
    }

    Ok(result)
}

/// Helper function to get string descriptors from a USB device
fn get_string_descriptor(
    handle: &DeviceHandle<Context>,
    index: Option<u8>,
) -> Result<String, String> {
    match index {
        Some(idx) => {
            // Use 0x0409 for English (United States)
            handle
                .read_string_descriptor_ascii(idx)
                .map_err(|e| e.to_string())
        }
        None => Err("No string descriptor index".to_string()),
    }
}
