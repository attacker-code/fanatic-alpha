//! FANatic Alpha — Odds Monitor
//! 
//! Connects to TxLINE SSE stream for real-time World Cup odds ingestion.
//! Publishes parsed odds events to the strategy engine via MPSC channel.

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::mpsc;
use strategy_engine::{OddsEvent, Signal};

mod sse_client;

#[derive(Debug, Deserialize)]
struct TxLineConfig {
    api_token: String,
    sse_url: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("[FANatic Alpha] Odds Monitor starting...");
    
    let config = load_config()?;
    let (tx, mut rx) = mpsc::channel::<OddsEvent>(1024);
    let (signal_tx, signal_rx) = mpsc::channel::<Signal>(256);
    
    // Spawn strategy engine
    let strategy_handle = tokio::spawn(async move {
        strategy_engine::run(signal_rx).await
    });
    
    // Connect SSE
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;
    
    println!("[FANatic Alpha] Connecting to TxLINE SSE stream...");
    let sse_stream = sse_client::connect(&client, &config.sse_url, &config.api_token).await?;
    
    println!("[FANatic Alpha] Odds Monitor running. Waiting for events...");
    
    tokio::select! {
        _ = sse_client::process_stream(sse_stream, tx.clone()) => {
            println!("[FANatic Alpha] SSE stream ended");
        }
        _ = strategy_handle => {
            println!("[FANatic Alpha] Strategy engine exited");
        }
        _ = tokio::signal::ctrl_c() => {
            println!("[FANatic Alpha] Shutdown signal received");
        }
    }
    
    Ok(())
}

fn load_config() -> Result<TxLineConfig> {
    Ok(TxLineConfig {
        api_token: std::env::var("TXLINE_API_TOKEN")
            .context("TXLINE_API_TOKEN not set")?,
        sse_url: std::env::var("TXLINE_SSE_URL")
            .unwrap_or_else(|_| "https://txline.txodds.com/stream/odds".into()),
    })
}
