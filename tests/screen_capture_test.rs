//! Integration test for the cross-platform screen_capture module

use frida_rust::screen_capture::ScreenCapture;
use std::fs;

#[test]
fn test_capture_and_save() {
    let output_dir = "logs/test_screenshots";
    // Clean up any old test output
    let _ = fs::remove_dir_all(output_dir);
    fs::create_dir_all(output_dir).expect("Failed to create output dir");
    let result = ScreenCapture::capture_and_save(output_dir, None);
    assert!(result.is_ok(), "Screen capture failed: {:?}", result.err());
    let paths = result.unwrap();
    assert!(!paths.is_empty(), "No screenshots were captured");
    let path = &paths[0];
    assert!(path.ends_with(".png"), "Screenshot file is not a PNG");
    assert!(
        fs::metadata(path).is_ok(),
        "Screenshot file was not created"
    );
    // Clean up after test (optional)
    let _ = fs::remove_dir_all(output_dir);
}
