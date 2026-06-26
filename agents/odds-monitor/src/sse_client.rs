//! SSE Client for TxLINE odds stream
//!
//! Handles connection, reconnection with exponential backoff,
//! and event parsing for the TxLINE Server-Sent Events protocol.

use anyhow::{Context, Result};
use reqwest::Client;
use std::time::Duration;
use tokio::sync::mpsc;
use strategy_engine::OddsEvent;

/// Connect to the TxLINE SSE stream and return the response stream.
pub async fn connect(
    client: &Client,
    url: &str,
    api_token: &str,
) -> Result<reqwest::Response> {
    let response = client
        .get(url)
        .header("Authorization", format!("Bearer {}", api_token))
        .header("Accept", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .send()
        .await
        .context("Failed to connect to SSE stream")?;
    
    if !response.status().is_success() {
        anyhow::bail!("SSE connection failed: HTTP {}", response.status());
    }
    
    println!("[SSE] Connected to TxLINE stream");
    Ok(response)
}

/// Process SSE events, parsing odds updates and publishing to the strategy engine.
pub async fn process_stream(
    response: reqwest::Response,
    tx: mpsc::Sender<OddsEvent>,
) -> Result<()> {
    use futures::StreamExt;
    
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();
    let mut retry_count = 0u32;
    let max_retries = 10;
    
    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                retry_count = 0;
                let text = String::from_utf8_lossy(&chunk);
                buffer.push_str(&text);
                
                // Parse SSE events from buffer
                while let Some(event_end) = buffer.find("\n\n") {
                    let event_str = buffer[..event_end].to_string();
                    buffer = buffer[event_end + 2..].to_string();
                    
                    if let Some(event) = parse_sse_event(&event_str) {
                        if tx.send(event).await.is_err() {
                            println!("[SSE] Strategy engine channel closed");
                            return Ok(());
                        }
                    }
                }
            }
            Err(e) => {
                retry_count += 1;
                eprintln!("[SSE] Stream error (attempt {}/{}): {}", retry_count, max_retries, e);
                
                if retry_count >= max_retries {
                    anyhow::bail!("SSE stream failed after {} retries", max_retries);
                }
                
                let backoff = Duration::from_millis(100 * 2u64.pow(retry_count.min(10)));
                tokio::time::sleep(backoff).await;
            }
        }
    }
    
    println!("[SSE] Stream ended normally");
    Ok(())
}

fn parse_sse_event(raw: &str) -> Option<OddsEvent> {
    let mut event_type = String::new();
    let mut data = String::new();
    
    for line in raw.lines() {
        if let Some(value) = line.strip_prefix("event: ") {
            event_type = value.trim().to_string();
        } else if let Some(value) = line.strip_prefix("data: ") {
            data = value.trim().to_string();
        }
    }
    
    if data.is_empty() {
        return None;
    }
    
    serde_json::from_str::<OddsEvent>(&data).ok()
}
