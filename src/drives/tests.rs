use super::*;

#[test]
fn test_list_drives_returns_data() {
    let drives = list_drives();
    // Basic validation that we get something back
    assert!(!drives.is_empty(), "Should return at least one drive");
}

#[test]
fn test_sys_info_contains_valid_data() {
    let info = sys_info();
    
    // Basic validation of returned data
    assert!(!info.current_user.is_empty(), "Username should not be empty");
    assert!(!info.cpu.is_empty(), "CPU information should not be empty");
    assert!(info.cpu_cores > 0, "Should detect at least one CPU core");
}

// For functions that might have side effects or access sensitive data,
// use conditional compilation to only run in test environments
#[cfg(test)]
mod secure_tests {
    use super::*;
    
    #[test]
    #[ignore = "Only run manually in secure environment"]
    fn test_sensitive_operations() {
        // Tests that might access or log sensitive data
        // Mark these with #[ignore] to prevent accidental execution
    }
}
