//! Logging and event tracking module for Project FRIDA.
//!
//! This module provides a centralized, thread-safe, and process-safe logging
//! facility. It ensures that all log messages, whether from the main process or
//! an injected replica, are written to a single log file (`frida.log`) located
//! in a `logs` directory next to the main executable.

use chrono::Local;
use once_cell::sync::Lazy;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::sync::Mutex;

use crate::paths;

// A global, thread-safe logger instance.
// `Lazy` ensures that the logger is initialized only once, the first time it's accessed.
static LOGGER: Lazy<Mutex<Logger>> = Lazy::new(|| Mutex::new(Logger::new()));

/// Represents the global logger.
struct Logger {
    file: Option<File>,
}

impl Logger {
    /// Creates and initializes a new Logger instance.
    fn new() -> Self {
        let log_file = Self::initialize_log_file();
        Logger { file: log_file }
    }
    /// Initializes the log file path and creates the file and directories.
    /// Returns an `Option<File>` handle for writing.
    fn initialize_log_file() -> Option<File> {
        // Use the centralized paths module to get the correct data directory.
        let data_dir = &paths::get().data_dir;
        let log_dir = data_dir.join("logs");

        if fs::create_dir_all(&log_dir).is_err() {
            // Cannot create log directory, so we can't log to a file.
            // We can still log to stdout, but file logging will be disabled.
            return None;
        }
        let log_path = log_dir.join("frida.log");
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .ok()
    }
    /// Writes a log message to the file and prints it to the console.
    fn log(&mut self, level: &str, message: &str) {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
        let log_entry = format!("{} [{}] - {}", timestamp, level, message);
        // Print to console for real-time feedback during development/debugging.
        println!("{}", log_entry);
        // Write to the log file if it's available.
        if let Some(file) = self.file.as_mut() {
            // We use writeln! to add a newline character.
            let _ = writeln!(file, "{}", log_entry);
        }
    }
}

/// Public logging interface.
pub struct Log;
impl Log {
    /// Logs an informational message.
    pub fn info(msg: String) {
        if let Ok(mut logger) = LOGGER.lock() {
            logger.log("INFO", &msg);
        }
    }
    /// Logs an error message.
    pub fn error(msg: String) {
        if let Ok(mut logger) = LOGGER.lock() {
            logger.log("ERROR", &msg);
        }
    }
    /// Logs a warning message.
    pub fn warn(msg: String) {
        if let Ok(mut logger) = LOGGER.lock() {
            logger.log("WARN", &msg);
        }
    }
}
