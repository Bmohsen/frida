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
    pub(crate) mount_point: String,
    pub total_space_gb: f64,
    pub available_space_gb: f64,
    pub(crate) is_removable: bool,
    pub is_system: bool,
    pub file_system: String,
    pub disk_type: String,
}

#[derive(Serialize, Debug)]
pub struct SysInfo<T> {
    current_user: String,
    cpu: String,
    cpu_cores: u8,
    memmory: Option<T>,
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
    disks.iter().map(|disk: &Disk| {
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
    let current_user =  current_user.get(0).unwrap().name().to_string();

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