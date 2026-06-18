// src/backend/streaming/mod.rs
use crate::vault::SecureVault;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use log::{info, warn};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StreamStatus {
    GeyserConnected,
    GeyserLagging,
    RPCFallback,
    Disconnected,
}

pub struct GeyserStreamManager {
    vault: Arc<SecureVault>,
    status: Arc<RwLock<StreamStatus>>,
    last_heartbeat: Arc<RwLock<i64>>,
    slot_lag: Arc<RwLock<u64>>,
}

impl GeyserStreamManager {
    pub async fn new(vault: Arc<SecureVault>) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(GeyserStreamManager {
            vault,
            status: Arc::new(RwLock::new(StreamStatus::Disconnected)),
            last_heartbeat: Arc::new(RwLock::new(0)),
            slot_lag: Arc::new(RwLock::new(0)),
        })
    }
    
    /// Start Geyser gRPC stream with automatic fallover
    pub async fn start_stream(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("🔗 Attempting Geyser gRPC connection...");
        
        // Load config
        let config = self.vault.load_config().await?;
        
        // Attempt Geyser connection
        match self.connect_geyser(&config.geyser_rpc_url).await {
            Ok(_) => {
                *self.status.write().await = StreamStatus::GeyserConnected;
                info!("✅ Geyser gRPC connected");
                
                // Start lag detection
                self.spawn_lag_detector().await;
            }
            Err(e) => {
                warn!("⚠️ Geyser failed: {}. Falling back to JSON-RPC...", e);
                *self.status.write().await = StreamStatus::RPCFallback;
                
                // Start JSON-RPC fallback
                self.start_rpc_polling(&config.backup_rpc_url).await?;
            }
        }
        
        Ok(())
    }
    
    /// Connect to Yellowstone Geyser gRPC
    async fn connect_geyser(&self, url: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Simulated gRPC connection
        // In production, use tonic to connect to actual Geyser endpoint
        
        if url.contains("helius") || url.contains("geyser") {
            info!("✅ Connected to Geyser: {}", url);
            Ok(())
        } else {
            Err("Invalid Geyser URL".into())
        }
    }
    
    /// Detect lag in Geyser stream
    async fn spawn_lag_detector(&self) {
        let status = self.status.clone();
        let slot_lag = self.slot_lag.clone();
        let heartbeat = self.last_heartbeat.clone();
        
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
                
                // Simulate slot lag detection
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;
                
                let last = *heartbeat.read().await;
                let age = now - last;
                
                if age > 2 {
                    // More than 2 slots worth of time (800ms each)
                    let mut s = status.write().await;
                    if *s == StreamStatus::GeyserConnected {
                        warn!("⚠️ Geyser lag detected. Lag: {} slots", age / 400);
                        *s = StreamStatus::GeyserLagging;
                    }
                } else if age < 1 {
                    let mut s = status.write().await;
                    *s = StreamStatus::GeyserConnected;
                }
                
                *slot_lag.write().await = age as u64;
            }
        });
    }
    
    /// Start JSON-RPC polling as fallback
    async fn start_rpc_polling(&self, url: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("🔄 Started JSON-RPC polling: {}", url);
        
        // In production, this would make HTTP requests to the RPC endpoint
        // For simulation:
        *self.status.write().await = StreamStatus::RPCFallback;
        
        Ok(())
    }
    
    /// Get current stream status
    pub async fn get_status(&self) -> StreamStatus {
        *self.status.read().await
    }
    
    /// Update stream heartbeat
    pub async fn heartbeat(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        
        *self.last_heartbeat.write().await = now;
    }
}
