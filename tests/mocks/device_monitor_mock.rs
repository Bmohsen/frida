//! Mock implementations for the device_monitor module
use std::sync::{Arc, Mutex};

// Define a trait for the device monitor functionality - don't use automock
pub trait DeviceMonitor {
    fn get_connected_devices(&self) -> Vec<DeviceInfo>;
    fn register_device_callback(&self, callback: Box<dyn Fn(&DeviceInfo) + Send + 'static>);
}

// DeviceInfo struct that represents USB devices
#[derive(Clone, Debug)]
pub struct DeviceInfo {
    pub vendor_id: u16,
    pub product_id: u16,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial_number: Option<String>,
}

// Test implementation for the DeviceMonitor trait
pub struct TestDeviceMonitor {
    mock_devices: Arc<Mutex<Vec<DeviceInfo>>>,
}

impl Default for TestDeviceMonitor {
    fn default() -> Self {
        let mock_devices = vec![
            DeviceInfo {
                vendor_id: 0x046d, // Logitech
                product_id: 0xc52b,
                manufacturer: Some("Logitech".to_string()),
                product: Some("USB Keyboard".to_string()),
                serial_number: Some("LGKBD123456".to_string()),
            },
            DeviceInfo {
                vendor_id: 0x8564,
                product_id: 0x1000,
                manufacturer: Some("Kingston".to_string()),
                product: Some("DataTraveler".to_string()),
                serial_number: Some("KS0001234567".to_string()),
            },
        ];

        Self {
            mock_devices: Arc::new(Mutex::new(mock_devices)),
        }
    }
}

impl DeviceMonitor for TestDeviceMonitor {
    fn get_connected_devices(&self) -> Vec<DeviceInfo> {
        let devices = self.mock_devices.lock().unwrap();
        devices.clone()
    }

    fn register_device_callback(&self, callback: Box<dyn Fn(&DeviceInfo) + Send + 'static>) {
        // In a test implementation, we could manually trigger the callback
        // with mock devices to test the callback functionality
        let devices = self.mock_devices.lock().unwrap();

        // Just call the callback once with the first device for demonstration
        if !devices.is_empty() {
            let device = &devices[0];
            callback(device);
        }
    }
}
