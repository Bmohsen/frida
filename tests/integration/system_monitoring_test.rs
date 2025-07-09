//! Integration tests for system monitoring functionality
//!
//! These tests verify that multiple components work together correctly
//! but use mocks to avoid accessing actual system resources.

use rstest::*;
use serial_test::serial;

// Import the trait implementations from our mocks
use crate::mocks::device_monitor_mock::{DeviceMonitor, TestDeviceMonitor};
use crate::mocks::drives_mock::{DriveEnumerator, TestDriveEnumerator};

#[fixture]
fn drive_enumerator() -> TestDriveEnumerator {
    TestDriveEnumerator::default()
}

#[fixture]
fn device_monitor() -> TestDeviceMonitor {
    TestDeviceMonitor::default()
}

#[rstest]
#[serial]
fn test_system_data_collection(
    drive_enumerator: TestDriveEnumerator,
    device_monitor: TestDeviceMonitor,
) {
    // Test that we can collect system data from multiple sources
    let drives = drive_enumerator.list_drives();
    let system_info = drive_enumerator.sys_info();
    let devices = device_monitor.get_connected_devices();

    // Verify we got data from all sources
    assert!(!drives.is_empty(), "Should have at least one drive");
    assert!(
        !system_info.current_user.is_empty(),
        "Should have a username"
    );
    assert!(!devices.is_empty(), "Should have at least one device");

    // Verify the data makes sense together
    if let Some(system_drive) = drives.iter().find(|d| d.is_system) {
        println!(
            "System drive: {} with {}GB total space",
            system_drive.name, system_drive.total_space_gb
        );
    } else {
        println!("No system drive found in test data");
    }

    println!(
        "Username: {}, CPU: {}",
        system_info.current_user, system_info.cpu
    );

    for device in devices {
        println!(
            "Device: {:04x}:{:04x} - {}",
            device.vendor_id,
            device.product_id,
            device.product.unwrap_or_else(|| "Unknown".to_string())
        );
    }
}

// Test table-driven approach for different system configurations
#[rstest]
#[case("Windows", "C:", true)]
#[case("Linux", "/", true)]
#[case("MacOS", "/System", true)]
#[case("Windows", "E:", false)]
fn test_system_drive_detection(
    #[case] os_name: &str,
    #[case] drive_path: &str,
    #[case] expected_is_system: bool,
) {
    // This test shows how to use parameterized tests for multiple test cases
    println!(
        "Testing if {} drive on {} is a system drive",
        drive_path, os_name
    );

    let is_system_drive = match os_name {
        "Windows" => drive_path == "C:",
        "Linux" => drive_path == "/",
        "MacOS" => drive_path.contains("/System"),
        _ => false,
    };

    assert_eq!(
        is_system_drive,
        expected_is_system,
        "Drive {} on {} should{} be detected as a system drive",
        drive_path,
        os_name,
        if expected_is_system { "" } else { " not" }
    );
}
