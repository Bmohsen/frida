//! Cross-platform screen capture module for Project FRIDA
//!
//! Uses the `screenshots` crate for Windows, macOS, and Linux support.
//! Supports timed captures with a specified interval in seconds.

use chrono::Local;
use screenshots::Screen;
use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;

#[derive(Debug)]
pub enum ScreenCaptureError {
    NoScreens,
    CaptureFailed(String),
    SaveFailed(String),
}

pub struct ScreenCapture;

impl ScreenCapture {
    /// Capture the primary screen and save to the specified directory.
    /// If time_seconds is provided, captures will be taken at that interval.
    /// Returns the path to the saved image on success, or a vector of paths if time_seconds is provided.
    pub fn capture_and_save<P: AsRef<Path>>(
        output_dir: P,
        time_seconds: Option<u64>,
    ) -> Result<Vec<String>, ScreenCaptureError> {
        let mut paths = Vec::new();

        // For immediate capture
        let capture_once = || -> Result<String, ScreenCaptureError> {
            let screens =
                Screen::all().map_err(|e| ScreenCaptureError::CaptureFailed(e.to_string()))?;
            let screen = screens.get(0).ok_or(ScreenCaptureError::NoScreens)?;
            let image = screen
                .capture()
                .map_err(|e| ScreenCaptureError::CaptureFailed(e.to_string()))?;
            let timestamp = Local::now().format("%Y%m%d_%H%M%S");
            let filename = format!("screenshot_{}.png", timestamp);
            let output_path = output_dir.as_ref().join(&filename);
            image
                .save(&output_path)
                .map_err(|e| ScreenCaptureError::SaveFailed(e.to_string()))?;
            Ok(output_path.to_string_lossy().to_string())
        };

        // Take first capture
        paths.push(capture_once()?);

        // If time_seconds is specified, continue capturing at interval
        if let Some(interval) = time_seconds {
            let duration = Duration::from_secs(interval);
            while let Ok(path) = capture_once() {
                paths.push(path);
                thread::sleep(duration);
            }
        }

        Ok(paths)
    }

    /// Capture all screens and save to the specified directory.
    /// If time_seconds is provided, captures will be taken at that interval.
    /// Returns a vector of saved image paths.
    pub fn capture_all_and_save<P: AsRef<Path>>(
        output_dir: P,
        time_seconds: Option<u64>,
    ) -> Result<Vec<String>, ScreenCaptureError> {
        let mut all_paths = Vec::new();

        // For capturing all screens at once
        let capture_all_once = || -> Result<Vec<String>, ScreenCaptureError> {
            let screens =
                Screen::all().map_err(|e| ScreenCaptureError::CaptureFailed(e.to_string()))?;
            if screens.is_empty() {
                return Err(ScreenCaptureError::NoScreens);
            }
            fs::create_dir_all(&output_dir)
                .map_err(|e| ScreenCaptureError::SaveFailed(e.to_string()))?;
            let timestamp = Local::now().format("%Y%m%d_%H%M%S");
            let mut capture_paths = Vec::new();

            for (i, screen) in screens.iter().enumerate() {
                let image = screen
                    .capture()
                    .map_err(|e| ScreenCaptureError::CaptureFailed(e.to_string()))?;
                let filename = format!("screenshot_{}_{}.png", i, timestamp);
                let output_path = output_dir.as_ref().join(filename);
                image
                    .save(&output_path)
                    .map_err(|e| ScreenCaptureError::SaveFailed(e.to_string()))?;
                capture_paths.push(output_path.to_string_lossy().to_string());
            }
            Ok(capture_paths)
        };

        // Take first capture
        all_paths.extend(capture_all_once()?);

        // If time_seconds is specified, continue capturing at interval
        if let Some(interval) = time_seconds {
            let duration = Duration::from_secs(interval);
            while let Ok(paths) = capture_all_once() {
                all_paths.extend(paths);
                thread::sleep(duration);
            }
        }

        Ok(all_paths)
    }
}
