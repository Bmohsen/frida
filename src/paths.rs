//! Centralized path management for Project FRIDA.
//!
//! This module ensures that all file paths used by the application are absolute
//! and relative to the location of the agent's module (EXE or DLL). This is
//! critical for ensuring that when the agent is injected into another process,
//! it still writes its files to the correct, intended location.

use once_cell::sync::Lazy;
use std::path::PathBuf;

#[cfg(windows)]
use winapi::um::libloaderapi::{GetModuleHandleW, GetModuleFileNameW};
#[cfg(windows)]
use std::os::windows::ffi::OsStringExt;
#[cfg(windows)]
use std::ffi::OsString;

/// A singleton that holds the base directory for all application data.
static APP_DIRS: Lazy<AppDirs> = Lazy::new(AppDirs::new);

/// Holds the base paths for the application.
#[derive(Debug)]
pub struct AppDirs {
    /// The directory where the agent (EXE or DLL) is located.
    pub base_dir: PathBuf,
    /// The directory where all logs and output data should be stored.
    pub data_dir: PathBuf,
}

impl AppDirs {
    fn new() -> Self {
        let base_dir = Self::get_current_module_path().unwrap_or_default();
        let data_dir = base_dir.join("frida_data");

        // Ensure the data directory exists.
        if !data_dir.exists() {
            let _ = std::fs::create_dir_all(&data_dir);
        }

        AppDirs { base_dir, data_dir }
    }

    /// Gets the path of the current module (EXE or DLL).
    #[cfg(windows)]
    fn get_current_module_path() -> Option<PathBuf> {
        unsafe {
            let hmodule = GetModuleHandleW(std::ptr::null_mut());
            if hmodule.is_null() { return None; }

            let mut buffer = vec![0u16; 260]; // MAX_PATH
            let len = GetModuleFileNameW(hmodule, buffer.as_mut_ptr(), buffer.len() as u32);

            if len == 0 { return None; }

            Some(PathBuf::from(OsString::from_wide(&buffer[..len as usize])))
        }
        .and_then(|p| p.parent().map(PathBuf::from))
    }

    #[cfg(not(windows))]
    fn get_current_module_path() -> Option<PathBuf> {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(PathBuf::from))
    }
}

/// Returns a reference to the global AppDirs instance.
pub fn get() -> &'static AppDirs {
    &APP_DIRS
}
