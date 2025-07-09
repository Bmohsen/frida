//! Project FRIDA Library
//!
//! Re-exports all modules so they can be shared between the binary (`main.rs`)
//! and external integration tests.

pub mod drives;
pub mod device_monitor;
pub mod keylogger;
pub mod network;
pub mod writer;
pub mod injector;
pub mod file_scanner;
pub mod log;
pub mod screen_capture;
