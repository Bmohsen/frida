//! Common test utilities for Project FRIDA
//!
//! This module contains shared utilities, fixtures, and helper functions
//! for testing across the project.

use std::path::PathBuf;
use tempfile::TempDir;

/// Creates a temporary directory for test files
pub fn temp_test_dir() -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("Failed to create temporary directory");
    let path = dir.path().to_path_buf();
    (dir, path)
}

/// Test fixture that provides a mock system configuration
pub struct MockSystemConfig {
    pub username: String,
    pub hostname: String,
    pub os_type: String,
    pub drives: Vec<MockDrive>,
}

impl Default for MockSystemConfig {
    fn default() -> Self {
        Self {
            username: "test_user".to_string(),
            hostname: "test-host".to_string(),
            os_type: "TestOS".to_string(),
            drives: vec![
                MockDrive {
                    name: "C:".to_string(),
                    total_space_gb: 500.0,
                    available_space_gb: 250.0,
                    is_system: true,
                },
                MockDrive {
                    name: "D:".to_string(),
                    total_space_gb: 1000.0,
                    available_space_gb: 750.0,
                    is_system: false,
                },
            ],
        }
    }
}

/// Mock drive for testing
pub struct MockDrive {
    pub name: String,
    pub total_space_gb: f64,
    pub available_space_gb: f64,
    pub is_system: bool,
}

/// Mock device for testing
pub struct MockDevice {
    pub vendor_id: u16,
    pub product_id: u16,
    pub manufacturer: String,
    pub product: String,
    pub serial: String,
}

impl Default for MockDevice {
    fn default() -> Self {
        Self {
            vendor_id: 0x1234,
            product_id: 0x5678,
            manufacturer: "Test Manufacturer".to_string(),
            product: "Test Device".to_string(),
            serial: "TEST123456789".to_string(),
        }
    }
}
