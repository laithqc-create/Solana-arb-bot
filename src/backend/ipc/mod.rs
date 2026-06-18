// src/backend/ipc/mod.rs
use crate::engine::ArbitrageEngine;
use crate::streaming::GeyserStreamManager;
use crate::vault::SecureVault;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json::json;
use log::info;

pub struct IPCHandler {
    engine: Arc<RwLock<ArbitrageEngine>>,
    stream_manager: Arc<GeyserStreamManager>,
    vault: Arc<SecureVault>,
}

// IPCHandler is Send + Sync because all its fields are Arc (thread-safe shared pointers)
unsafe impl Send for IPCHandler {}
unsafe impl Sync for IPCHandler {}

impl IPCHandler {
    pub fn new(
        engine: Arc<RwLock<ArbitrageEngine>>,
        stream_manager: Arc<GeyserStreamManager>,
        vault: Arc<SecureVault>,
    ) -> Self {
        IPCHandler {
            engine,
            stream_manager,
            vault,
        }
    }
    
    pub async fn start_ipc_server(&self) -> Result<(), String> {
        info!("🚀 IPC server starting");
        
        // In a real implementation, this would use unix domain sockets or named pipes
        // For Tauri integration, we use the invoke! mechanism instead
        
        Ok(())
    }
    
    /// Start the stream manager
    pub async fn start_stream(&self) -> Result<(), String> {
        self.stream_manager.start_stream().await
    }
    
    /// Handle incoming Tauri command: get_opportunities
    pub async fn handle_get_opportunities(&self) -> String {
        let engine = self.engine.read().await;
        let opps = engine.detect_opportunities().await;
        
        json!({
            "success": true,
            "opportunities": opps,
            "count": opps.len(),
        }).to_string()
    }
    
    /// Handle incoming Tauri command: get_stream_status
    pub async fn handle_get_stream_status(&self) -> String {
        let status = self.stream_manager.get_status().await;
        
        json!({
            "status": format!("{:?}", status),
        }).to_string()
    }
    
    /// Handle incoming Tauri command: update_config
    pub async fn handle_update_config(&self, geyser_url: String, backup_url: String) -> String {
        match async {
            let mut config = self.vault.load_config().await?;
            config.geyser_rpc_url = geyser_url;
            config.backup_rpc_url = backup_url;
            self.vault.save_config(&config).await?;
            Ok::<_, String>(())
        }.await {
            Ok(_) => {
                json!({
                    "success": true,
                    "message": "Configuration updated",
                }).to_string()
            }
            Err(e) => {
                json!({
                    "success": false,
                    "error": e,
                }).to_string()
            }
        }
    }
    
    /// Handle incoming Tauri command: get_vault_config
    pub async fn handle_get_vault_config(&self) -> String {
        match self.vault.load_config().await {
            Ok(config) => {
                json!({
                    "success": true,
                    "config": {
                        "geyser_rpc_url": config.geyser_rpc_url,
                        "backup_rpc_url": config.backup_rpc_url,
                        "jito_region": config.jito_region,
                    }
                }).to_string()
            }
            Err(e) => {
                json!({
                    "success": false,
                    "error": format!("{}", e),
                }).to_string()
            }
        }
    }
}
