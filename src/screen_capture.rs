//! Cross-platform screen capture module for Project FRIDA
//!
//! Uses the `screenshots` crate for Windows, macOS, and Linux support.

use screenshots::{Screen};
use std::path::Path;
use std::fs;
use chrono::Local;

#[derive(Debug)]
pub enum ScreenCaptureError {
    NoScreens,
    CaptureFailed(String),
    SaveFailed(String),
}

pub struct ScreenCapture;

impl ScreenCapture {
    /// Capture the primary screen and save to the specified directory.
    /// Returns the path to the saved image on success.
    pub fn capture_and_save<P: AsRef<Path>>(output_dir: P) -> Result<String, ScreenCaptureError> {
        let screens = Screen::all().map_err(|e| ScreenCaptureError::CaptureFailed(e.to_string()))?;
        let screen = screens.get(0).ok_or(ScreenCaptureError::NoScreens)?;
        let image = screen.capture().map_err(|e| ScreenCaptureError::CaptureFailed(e.to_string()))?;
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("screenshot_{}.png", timestamp);
        let output_path = output_dir.as_ref().join(filename);
        image.save(&output_path).map_err(|e| ScreenCaptureError::SaveFailed(e.to_string()))?;
        Ok(output_path.to_string_lossy().to_string())
    }

    /// Capture all screens and save to the specified directory.
    /// Returns a vector of saved image paths.
    pub fn capture_all_and_save<P: AsRef<Path>>(output_dir: P) -> Result<Vec<String>, ScreenCaptureError> {
        let screens = Screen::all().map_err(|e| ScreenCaptureError::CaptureFailed(e.to_string()))?;
        if screens.is_empty() {
            return Err(ScreenCaptureError::NoScreens);
        }
        fs::create_dir_all(&output_dir).map_err(|e| ScreenCaptureError::SaveFailed(e.to_string()))?;
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let mut paths = Vec::new();
        for (i, screen) in screens.iter().enumerate() {
            let image = screen.capture().map_err(|e| ScreenCaptureError::CaptureFailed(e.to_string()))?;
            let filename = format!("screenshot_{}_{}.png", i, timestamp);
            let output_path = output_dir.as_ref().join(filename);
            image.save(&output_path).map_err(|e| ScreenCaptureError::SaveFailed(e.to_string()))?;
            paths.push(output_path.to_string_lossy().to_string());
        }
        Ok(paths)
    }
}
