// Import from the actual library crate, not from tests
use frida_rust::drives::{DriveInfo, SysInfo};

// Define the MockDrive directly in this file instead of importing from common
pub struct MockDrive {
    pub name: String,
    pub total_space_gb: f64,
    pub available_space_gb: f64,
    pub is_system: bool,
}

// First, we create a mockable trait for the drives functionality
// This allows unit testing without touching real system resources

// Instead of using automock which has issues, define the trait manually
pub trait DriveEnumerator {
    fn list_drives(&self) -> Vec<DriveInfo>;
    fn sys_info(&self) -> SysInfo<String>;
}

// Implement a mock drive enumerator for testing
pub struct TestDriveEnumerator {
    mock_drives: Vec<MockDrive>,
    mock_username: String,
    mock_cpu: String,
    mock_cores: u8,
}

impl Default for TestDriveEnumerator {
    fn default() -> Self {
        Self {
            mock_drives: vec![
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
            mock_username: "test_user".to_string(),
            mock_cpu: "Test CPU @ 3.0GHz".to_string(),
            mock_cores: 4,
        }
    }
}

impl DriveEnumerator for TestDriveEnumerator {
    fn list_drives(&self) -> Vec<DriveInfo> {
        self.mock_drives
            .iter()
            .map(|mock_drive| DriveInfo {
                name: mock_drive.name.clone(),
                mount_point: format!("{}\\", mock_drive.name),
                total_space_gb: mock_drive.total_space_gb,
                available_space_gb: mock_drive.available_space_gb,
                is_removable: !mock_drive.is_system,
                is_system: mock_drive.is_system,
                file_system: "NTFS".to_string(),
                disk_type: if mock_drive.is_system { "SSD".to_string() } else { "HDD".to_string() },
            })
            .collect()
    }

    fn sys_info(&self) -> SysInfo<String> {
        SysInfo {
            current_user: self.mock_username.clone(),
            cpu: self.mock_cpu.clone(),
            cpu_cores: self.mock_cores,
            memmory: Some("16.00 GB".to_string()),
        }
    }
}
