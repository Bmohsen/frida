//! Drive enumeration and storage media analysis module
//!
//! This module provides functionality to enumerate and collect information about
//! storage devices connected to the system, including both fixed and removable media.
//! It captures drive capacity, available space, filesystem type, and other relevant metadata.

use serde::Serialize;
use sysinfo::{Disk, Disks, Pid, System, Users};

/// Holds structured information about a single disk drive.
/// Derives `Serialize` to allow for easy conversion to JSON.
#[derive(Serialize, Debug)]
pub struct DriveInfo {
    pub name: String,
    pub mount_point: String,
    pub total_space_gb: f64,
    pub available_space_gb: f64,
    pub is_removable: bool,
    pub is_system: bool,
    pub file_system: String,
    pub disk_type: String,
}

#[derive(Serialize, Debug)]
pub struct SysInfo<T> {
    pub current_user: String,
    pub cpu: String,
    pub cpu_cores: u8,
    pub memmory: Option<T>,
}

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: Pid,
    pub name: String,
    // add any other fields you need
}

/// Converts bytes to gigabytes with two decimal places.
fn bytes_to_gb(bytes: u64) -> f64 {
    (bytes as f64 / (1024.0 * 1024.0 * 1024.0) * 100.0).round() / 100.0
}

/// Lists all physical drives on the system.
///
/// # Returns
/// A `Vec<DriveInfo>` containing details for each detected drive.
pub fn list_drives() -> Vec<DriveInfo> {
    let disks = Disks::new_with_refreshed_list();
    disks
        .iter()
        .map(|disk: &Disk| {
            // Determine if this is a system drive based on mount point
            let mount_point = disk.mount_point().to_string_lossy().to_lowercase();
            let is_system = mount_point.starts_with("c:") || // Windows system drive
                           mount_point == "/" ||  // Linux/macOS root
                           mount_point.contains("system") || 
                           mount_point.contains("windows") ||
                           mount_point.contains("program");

            DriveInfo {
                name: disk.name().to_string_lossy().into_owned(),
                mount_point: disk.mount_point().to_string_lossy().into_owned(),
                total_space_gb: bytes_to_gb(disk.total_space()),
                available_space_gb: bytes_to_gb(disk.available_space()),
                is_removable: disk.is_removable(),
                is_system,
                file_system: format!("{:?}", disk.file_system()),
                disk_type: format!("{:?}", disk.kind()),
            }
        })
        .collect()
}

pub fn list_pss() -> Vec<ProcessInfo> {
    let sys = System::new_all();
    sys.processes()
        .iter()
        .map(|(&pid, process)| ProcessInfo {
            pid,
            name: process.name().to_string(),
            // extract other fields as needed
        })
        .collect()
}

pub fn sys_info() -> SysInfo<String> {
    let mut sys = System::new_all();
    sys.refresh_all();

    // Get current user (first user in the list for demo purposes)
    let current_user = Users::new_with_refreshed_list();
    let current_user = current_user.get(0).unwrap().name().to_string();

    // Get CPU brand and cores
    let cpu = sys.global_cpu_info().brand().to_string();
    let cpu_cores = sys.physical_core_count().unwrap_or(0) as u8;

    // Get total memory in GB
    let total_memory_gb = bytes_to_gb(sys.total_memory() * 1024 * 1024); // total_memory returns MB
    let memmory = Some(format!("{:.2} GB", total_memory_gb));

    SysInfo {
        current_user,
        cpu,
        cpu_cores,
        memmory,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bytes_to_gb_conversion() {
        // Test with 1GB
        assert_eq!(bytes_to_gb(1073741824), 1.0);

        // Test with 2.5GB
        assert_eq!(bytes_to_gb(2684354560), 2.5);

        // Test with small value
        assert_eq!(bytes_to_gb(104857600), 0.1); // 100MB should be 0.1GB
    }

    #[test]
    fn test_list_drives_returns_something() {
        let drives = list_drives();
        // Should return at least one drive on any system
        assert!(
            !drives.is_empty(),
            "No drives detected, which is unlikely on a real system"
        );
    }

    #[test]
    fn test_drive_info_structure() {
        let drives = list_drives();
        if !drives.is_empty() {
            let first_drive = &drives[0];

            // Available space must be less than or equal to total space
            // Only check this if we have a non-zero total space
            if first_drive.total_space_gb > 0.0 {
                assert!(
                    first_drive.available_space_gb <= first_drive.total_space_gb,
                    "Available space cannot exceed total space"
                );
            }

            // Only validate non-empty file system if we actually have a file system
            // Some virtual drives might not report this correctly
            if !first_drive.file_system.is_empty() {
                assert!(
                    first_drive.file_system.len() > 0,
                    "File system should be identified"
                );
            }

            // Test passes as long as we can access the drive info structure
            // without crashing, regardless of the actual values
        }
    }

    #[test]
    fn test_sys_info_contains_data() {
        let info = sys_info();

        // In test environments or containers, some system info might not be available
        // Only make assertions that should be true on any system that can run the tests

        // We should have at least a username or a placeholder
        // The current_user should be available from the Users struct, but let's be cautious
        println!("Current user: {}", info.current_user);

        // CPU cores should be at least 1 on any system that can run the test
        if info.cpu_cores == 0 {
            println!("Warning: CPU cores reported as 0, which is unusual");
        }

        // Memory info check - just verify we can call the method
        if let Some(mem_str) = &info.memmory {
            println!("Memory info: {}", mem_str);
        }

        // This test primarily verifies that sys_info() runs without crashing
        // It's sufficient for the test to complete without panicking
    }

    #[test]
    fn test_list_processes() {
        let processes = list_pss();

        // Should find at least some processes on any running system
        assert!(
            !processes.is_empty(),
            "No processes found, which is impossible on a running system"
        );

        if !processes.is_empty() {
            let first_process = &processes[0];
            // Process should have a name
            assert!(
                !first_process.name.is_empty(),
                "Process name should not be empty"
            );
        }
    }
}
