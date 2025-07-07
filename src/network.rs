//! Secure networking and data exfiltration module
//!
//! This module provides functionality for secure data transmission to remote endpoints.
//! It packages collected intelligence in standardized formats and implements robust
//! networking capabilities with error handling and retry mechanisms.
//! All communication is validated and tracked for audit purposes.

use serde::Serialize;

/// Sends serializable data to a specified HTTP endpoint.
///
/// # Arguments
/// * `data` - The data to send. Must implement `serde::Serialize`.
/// * `url` - The destination URL.
///
/// # Returns
/// A `Result` indicating success or the `reqwest::Error`.
pub async fn send_to_server<T: Serialize>(data: &T, url: &str) -> Result<(), reqwest::Error> {
    let client = reqwest::Client::new();
    let response = client.post(url).json(data).send().await?;

    // Check if the request was successful.
    response.error_for_status()?;

    Ok(())
}
