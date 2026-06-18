// src/backend/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod engine;
mod parsers;
mod vault;
mod ipc;
mod streaming;

use engine::ArbitrageEngine;
use ipc::IPCHandler;
use streaming::GeyserStreamManager;
use std::sync::Arc;
use tokio::sync::RwLock;
use log::{info, error};

#[tokio::main]
async fn main() {
    env_logger::init();
    
    info!("🚀 Solana Arbitrage Engine Starting...");
    
    // Initialize vault for encrypted config storage
    let vault = Arc::new(vault::SecureVault::new()
        .await
        .expect("Failed to initialize vault"));
    
    info!("✅ Vault initialized");
    
    // Initialize arbitrage engine
    let engine = Arc::new(RwLock::new(
        ArbitrageEngine::new(vault.clone())
            .await
            .expect("Failed to initialize arbitrage engine")
    ));
    
    info!("✅ Arbitrage engine initialized");
    
    // Initialize gRPC stream manager
    let stream_manager = Arc::new(GeyserStreamManager::new(vault.clone())
        .await
        .expect("Failed to initialize geyser stream"));
    
    info!("✅ Geyser stream manager initialized");
    
    // Initialize IPC handler for Tauri communication
    let ipc = Arc::new(IPCHandler::new(
        engine.clone(),
        stream_manager.clone(),
        vault.clone(),
    ));
    
    info!("✅ IPC handler initialized");
    
    // Start background tasks
    tokio::spawn(async move {
        if let Err(e) = ipc.start_ipc_server().await {
            error!("IPC server error: {}", e);
        }
    });
    
    info!("✅ All systems online. Waiting for Tauri frontend...");
    
    // Keep the backend alive
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for Ctrl+C");
    
    info!("🛑 Backend shutting down gracefully...");
}
