//! Cross-platform screen capture module for Project FRIDA
//!
//! Uses the `screenshots` crate for Windows, macOS, and Linux support.
//! Supports timed captures with a specified interval in seconds.
//! Provides efficient compression options (WebP, JPEG, PNG) to minimize file size.

use chrono::Local;
use image::{DynamicImage, ImageFormat};
use screenshots::Screen;
use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;
use webp::{Encoder, WebPMemory};

#[derive(Debug)]
pub enum ScreenCaptureError {
    NoScreens,
    CaptureFailed(String),
    SaveFailed(String),
    CompressionFailed(String),
}

/// Format options for saving screenshots
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompressionFormat {
    /// PNG format - lossless, larger file size
    PNG,
    /// JPEG format - lossy, medium file size, medium quality
    JPEG(u8), // Quality: 1-100
    /// WebP format - lossy, smallest file size, good quality
    WebP(f32), // Quality: 0.0-100.0
}

pub struct ScreenCapture;

impl ScreenCapture {
    /// Capture the primary screen and save to the specified directory.
    /// If time_seconds is provided, captures will be taken at that interval.
    /// Returns the path to the saved image on success, or a vector of paths if time_seconds is provided.
    /// Uses PNG format by default.
    pub fn capture_and_save<P: AsRef<Path>>(
        output_dir: P,
        time_seconds: Option<u64>,
    ) -> Result<Vec<String>, ScreenCaptureError> {
        Self::capture_and_save_with_compression(output_dir, time_seconds, CompressionFormat::PNG)
    }
    
    /// Capture the primary screen and save to the specified directory with compression.
    /// Allows specifying the compression format and quality.
    /// If time_seconds is provided, captures will be taken at that interval.
    /// Returns the path to the saved image on success, or a vector of paths if time_seconds is provided.
    pub fn capture_and_save_with_compression<P: AsRef<Path>>(
        output_dir: P,
        time_seconds: Option<u64>,
        format: CompressionFormat,
    ) -> Result<Vec<String>, ScreenCaptureError> {
        let mut paths = Vec::new();

        // Create output directory if it doesn't exist
        fs::create_dir_all(&output_dir)
            .map_err(|e| ScreenCaptureError::SaveFailed(e.to_string()))?;
            
        // For immediate capture
        let capture_once = || -> Result<String, ScreenCaptureError> {
            let screens =
                Screen::all().map_err(|e| ScreenCaptureError::CaptureFailed(e.to_string()))?;
            let screen = screens.get(0).ok_or(ScreenCaptureError::NoScreens)?;
            let image = screen
                .capture()
                .map_err(|e| ScreenCaptureError::CaptureFailed(e.to_string()))?;
                
            // Convert to DynamicImage for processing
            let img_buffer = image::ImageBuffer::from_raw(
                image.width() as u32,
                image.height() as u32,
                image.to_vec(),
            )
            .ok_or(ScreenCaptureError::CompressionFailed("Failed to create image buffer".to_string()))?;
            
            let dynamic_image = DynamicImage::ImageRgba8(img_buffer);
            
            // Compress and save with the specified format
            let timestamp = Local::now().format("%Y%m%d_%H%M%S");
            let (_filename, output_path) = Self::get_filename_and_path(&output_dir, timestamp.to_string(), &format);
            
            Self::save_with_compression(dynamic_image, &output_path, format)?;
            
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
    /// Uses PNG format by default.
    pub fn capture_all_and_save<P: AsRef<Path>>(
        output_dir: P,
        time_seconds: Option<u64>,
    ) -> Result<Vec<String>, ScreenCaptureError> {
        Self::capture_all_and_save_with_compression(output_dir, time_seconds, CompressionFormat::PNG)
    }
    
    /// Capture all screens and save to the specified directory with compression.
    /// Allows specifying the compression format and quality.
    /// If time_seconds is provided, captures will be taken at that interval.
    /// Returns a vector of saved image paths.
    pub fn capture_all_and_save_with_compression<P: AsRef<Path>>(
        output_dir: P,
        time_seconds: Option<u64>,
        format: CompressionFormat,
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

                // Convert to DynamicImage for processing
                let img_buffer = image::ImageBuffer::from_raw(
                    image.width() as u32,
                    image.height() as u32,
                    image.to_vec(),
                )
                .ok_or(ScreenCaptureError::CompressionFailed("Failed to create image buffer".to_string()))?;
                
                let dynamic_image = DynamicImage::ImageRgba8(img_buffer);
                
                // Compress and save with the specified format
                let (_filename, output_path) = Self::get_filename_and_path_with_index(
                    &output_dir, timestamp.to_string(), i, &format
                );
                
                Self::save_with_compression(dynamic_image, &output_path, format)?;
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
    
    // Helper function to get filename and path based on compression format
    fn get_filename_and_path<P: AsRef<Path>>(
        output_dir: P,
        timestamp: String,
        format: &CompressionFormat,
    ) -> (String, std::path::PathBuf) {
        let extension = match format {
            CompressionFormat::PNG => "png",
            CompressionFormat::JPEG(_) => "jpg",
            CompressionFormat::WebP(_) => "webp",
        };
        
        let filename = format!("screenshot_{}.{}", timestamp, extension);
        let output_path = output_dir.as_ref().join(&filename);
        
        (filename, output_path)
    }
    
    // Helper function to get filename and path with screen index
    fn get_filename_and_path_with_index<P: AsRef<Path>>(
        output_dir: P,
        timestamp: String,
        index: usize,
        format: &CompressionFormat,
    ) -> (String, std::path::PathBuf) {
        let extension = match format {
            CompressionFormat::PNG => "png",
            CompressionFormat::JPEG(_) => "jpg",
            CompressionFormat::WebP(_) => "webp",
        };
        
        let filename = format!("screenshot_{}_{}.{}", index, timestamp, extension);
        let output_path = output_dir.as_ref().join(&filename);
        
        (filename, output_path)
    }
    
    // Helper function to save an image with the specified compression format
    fn save_with_compression(
        image: DynamicImage,
        path: &std::path::Path,
        format: CompressionFormat,
    ) -> Result<(), ScreenCaptureError> {
        // Optional: Resize large images to reduce file size further if needed
        // let image = image.resize(width, height, image::imageops::FilterType::Lanczos3);
        
        match format {
            CompressionFormat::PNG => {
                image
                    .save_with_format(path, ImageFormat::Png)
                    .map_err(|e| ScreenCaptureError::SaveFailed(e.to_string()))?;
            }
            CompressionFormat::JPEG(quality) => {
                // Convert to RGB for JPEG (JPEG doesn't support alpha channel)
                let rgb_image = image.to_rgb8();
                let mut output = Vec::new();
                let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut output, quality);
                encoder
                    .encode_image(&rgb_image)
                    .map_err(|e| ScreenCaptureError::CompressionFailed(e.to_string()))?;
                fs::write(path, output)
                    .map_err(|e| ScreenCaptureError::SaveFailed(e.to_string()))?;
            }
            CompressionFormat::WebP(quality) => {
                // WebP format provides the best compression ratio
                let rgba = image.to_rgba8();
                let width = rgba.width() as u32;
                let height = rgba.height() as u32;
                
                let encoder = Encoder::from_rgba(&rgba, width, height);
                let webp_memory: WebPMemory = encoder.encode(quality);
                fs::write(path, &*webp_memory)
                    .map_err(|e| ScreenCaptureError::SaveFailed(e.to_string()))?;
            }
        }
        
        Ok(())
    }
}

// Recommended compression settings for different use cases
impl CompressionFormat {
    /// Returns optimal compression settings for smallest file size (WebP with high compression)
    pub fn smallest_size() -> Self {
        CompressionFormat::WebP(70.0)  // Good quality with high compression
    }
    
    /// Returns settings for balanced quality vs size (WebP with medium compression)
    pub fn balanced() -> Self {
        CompressionFormat::WebP(85.0)
    }
    
    /// Returns settings optimized for quality but still with good compression
    pub fn high_quality() -> Self {
        CompressionFormat::JPEG(92)
    }
    
    /// Returns settings for maximum quality (PNG lossless)
    pub fn max_quality() -> Self {
        CompressionFormat::PNG
    }
}
