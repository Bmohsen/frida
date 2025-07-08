//! Integration tests module entry for Cargo.
//! This file simply makes the Rust module system aware of individual
//! integration test files inside this directory so that `tests/mod.rs`
//! can `pub mod integration;` without compile errors.

pub mod system_monitoring_test;
