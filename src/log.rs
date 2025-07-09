//! Logging and event tracking module
//!
//! This module provides unified logging capabilities across the application.
//! It supports different log levels (info, error, warn) with consistent formatting
//! and can be extended to write logs to files or remote endpoints.
//! All application events are captured for auditing and debugging purposes.

pub struct Log {
    pub log_type: String,
    pub message: (),
}

impl Log {
    pub fn info(msg: String) -> Log {
        Log {
            log_type: "INFO".to_string(),
            message: println!("[INFO]: {}", msg),
        }
    }

    pub fn error(msg: String) -> Log {
        Log {
            log_type: "ERROR".to_string(),
            message: println!("[ERROR]: {}", msg),
        }
    }

    pub fn warn(msg: String) -> Log {
        Log {
            log_type: "WARN".to_string(),
            message: println!("[WARN]: {}", msg),
        }
    }
}
