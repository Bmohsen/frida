//! Process monitoring and script injection module
//!
//! This module provides functionality to monitor running processes
//! and execute Python scripts against target processes.

use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use sysinfo::{Pid, Process, System};

use crate::writer;

const SCRIPTS_DIR: &str = "scripts";
const PYTHON_DIR: &str = "python_embedded";
const PROCESS_LOG_FILE: &str = "logs/process_monitor.log";
const CHECK_INTERVAL_SECONDS: u64 = 5;

/// Structure to hold process information
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cmd: Vec<String>,
    pub memory_usage_kb: u64,
    pub cpu_usage: f32,
}

impl ProcessInfo {
    /// Create ProcessInfo from sysinfo::Process
    fn from_process(process: &Process) -> Self {
        Self {
            pid: process.pid().as_u32(),
            name: process.name().to_string(),
            cmd: process.cmd().to_vec(),
            memory_usage_kb: process.memory() / 1024,
            cpu_usage: process.cpu_usage() as f32,
        }
    }

    /// Format process info as a string
    #[allow(dead_code)]
    fn to_string(&self) -> String {
        format!(
            "Process: {} (PID: {})\n  Command: {}\n  Memory: {} KB\n  CPU: {:.2}%",
            self.name,
            self.pid,
            self.cmd.join(" "),
            self.memory_usage_kb,
            self.cpu_usage
        )
    }
}

/// Process monitor and script execution service
pub struct ProcessMonitor {
    system: Arc<Mutex<System>>,
}

impl ProcessMonitor {
    /// Create a new process monitor
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        Self {
            system: Arc::new(Mutex::new(system)),
        }
    }

    /// List all running processes
    pub fn list_processes(&self) -> Vec<ProcessInfo> {
        let mut system = self.system.lock().unwrap();
        system.refresh_processes();

        system
            .processes()
            .values()
            .map(ProcessInfo::from_process)
            .collect()
    }

    /// Get a process by PID
    pub fn get_process(&self, pid: u32) -> Option<ProcessInfo> {
        let mut system = self.system.lock().unwrap();
        system.refresh_processes();

        system
            .process(Pid::from_u32(pid))
            .map(ProcessInfo::from_process)
    }

    /// Execute a Python script against a target process
    pub fn execute_python_script(&self, target_pid: u32, script_name: &str) -> Result<(), String> {
        // Get exe directory
        let exe_dir = std::env::current_exe()
            .map_err(|e| e.to_string())?
            .parent()
            .ok_or("Cannot determine exe directory")?
            .to_path_buf();

        let python_dir = exe_dir.join(PYTHON_DIR);
        let python_exe = python_dir.join("python.exe");
        let script_path = exe_dir.join(SCRIPTS_DIR).join(script_name);

        if !python_exe.exists() {
            return Err(format!("Python executable not found at {:?}", python_exe));
        }
        if !script_path.exists() {
            return Err(format!("Script not found at {:?}", script_path));
        }

        // Check if the target process exists
        if self.get_process(target_pid).is_none() {
            return Err(format!("Process with PID {} not found", target_pid));
        }

        log::info!(
            "Executing Python script: {:?} on PID: {}",
            script_path,
            target_pid
        );

        // Execute the Python script with the target PID as an argument
        let output = Command::new(&python_exe)
            .arg(&script_path)
            .arg(target_pid.to_string())
            .env("PYTHONHOME", &python_dir)
            .env("PYTHONPATH", &python_dir.join("Lib"))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| format!("Failed to execute Python script: {}", e))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            log::error!("Python script execution failed: {}", error);
            return Err(format!("Python script execution failed: {}", error));
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        log::info!("Python script execution result: {}", output_str);

        Ok(())
    }

    /// Monitor for suspicious processes
    pub fn monitor_suspicious_activity(&self) -> Vec<(ProcessInfo, String)> {
        let mut system = self.system.lock().unwrap();
        system.refresh_processes();

        system
            .processes()
            .values()
            .filter_map(|process| {
                let cpu_usage = process.cpu_usage() as f32;
                let memory_usage_mb = process.memory() / 1024 / 1024;

                if cpu_usage > 90.0 {
                    Some((
                        ProcessInfo::from_process(process),
                        format!("High CPU usage: {:.2}%", cpu_usage),
                    ))
                } else if memory_usage_mb > 1000 {
                    Some((
                        ProcessInfo::from_process(process),
                        format!("High memory usage: {} MB", memory_usage_mb),
                    ))
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Creates the scripts directory and sample Python scripts
fn create_sample_scripts() -> Result<(), String> {
    // Get exe directory
    let exe_dir = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .ok_or("Cannot determine exe directory")?
        .to_path_buf();

    // Create scripts directory
    let scripts_dir = exe_dir.join(SCRIPTS_DIR);
    std::fs::create_dir_all(&scripts_dir).map_err(|e| e.to_string())?;

    // Create sample process analysis script
    let process_analysis_script = scripts_dir.join("process_analysis.py");
    if !process_analysis_script.exists() {
        let script_content = r#"#!/usr/bin/env python3
# Process Analysis Script
# For use with Project Frida process monitoring

import sys
import os
import time

def analyze_process(pid):
    """Analyze a process by its PID"""
    print(f"Analyzing process with PID: {pid}")
    
    # This would be expanded with actual process analysis code
    # using libraries like psutil in a real implementation
    
    print(f"Process analysis complete for PID: {pid}")
    print("Results would show memory maps, network connections,")
    print("file handles, and other process characteristics")
    
if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: process_analysis.py <pid>")
        sys.exit(1)
    
    try:
        pid = int(sys.argv[1])
        analyze_process(pid)
    except ValueError:
        print("Error: PID must be a number")
        sys.exit(1)
"#;
        std::fs::write(&process_analysis_script, script_content).map_err(|e| e.to_string())?;
        log::info!(
            "Created sample Python script at: {:?}",
            process_analysis_script
        );
    }

    // Create sample memory analysis script
    let memory_analysis_script = scripts_dir.join("memory_analysis.py");
    if !memory_analysis_script.exists() {
        let script_content = r#"#!/usr/bin/env python3
# Memory Analysis Script
# For use with Project Frida process monitoring

import sys
import os

def analyze_memory(pid):
    """Analyze memory usage of a process"""
    print(f"Analyzing memory for process with PID: {pid}")
    
    # This would be expanded with actual memory analysis code
    # using libraries like psutil in a real implementation
    
    print(f"Memory analysis complete for PID: {pid}")
    print("Results would show memory regions, heap analysis,")
    print("and potential memory leaks")
    
if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: memory_analysis.py <pid>")
        sys.exit(1)
    
    try:
        pid = int(sys.argv[1])
        analyze_memory(pid)
    except ValueError:
        print("Error: PID must be a number")
        sys.exit(1)
"#;
        std::fs::write(&memory_analysis_script, script_content).map_err(|e| e.to_string())?;
        log::info!(
            "Created sample Python script at: {:?}",
            memory_analysis_script
        );
    }

    Ok(())
}

/// Starts the process monitoring service in a separate thread
pub fn start_injector_service() {
    log::info!("Process monitoring service starting...");

    // Try to create sample scripts
    if let Err(e) = create_sample_scripts() {
        log::error!("Failed to create sample scripts: {}", e);
    }

    let monitor = Arc::new(ProcessMonitor::new());
    let monitor_clone = Arc::clone(&monitor);

    // Start with an initial list of processes
    let initial_processes = monitor.list_processes();
    log::info!("Initial process count: {}", initial_processes.len());

    // Log a few example processes
    for (i, proc) in initial_processes.iter().take(5).enumerate() {
        log::info!("Process {}: {} (PID: {})", i + 1, proc.name, proc.pid);
    }

    // Start the monitoring thread
    thread::spawn(move || {
        let mut last_check = Instant::now();

        loop {
            // Periodically check for suspicious activity
            if last_check.elapsed() > Duration::from_secs(CHECK_INTERVAL_SECONDS * 2) {
                let suspicious = monitor_clone.monitor_suspicious_activity();

                if !suspicious.is_empty() {
                    log::info!("Found {} suspicious activities", suspicious.len());

                    for (proc, reason) in suspicious {
                        let alert = format!(
                            "Suspicious process activity: {} (PID: {}) - {}",
                            proc.name, proc.pid, reason
                        );

                        log::info!("{}", alert);

                        // Save to log file
                        if let Err(e) = writer::save_output(&alert, PROCESS_LOG_FILE, true) {
                            log::error!("Failed to write suspicious activity: {}", e);
                        }
                    }
                }

                last_check = Instant::now();
            }

            // Sleep before checking again
            thread::sleep(Duration::from_secs(CHECK_INTERVAL_SECONDS));
        }
    });

    log::info!("Process monitoring service is running");
}
