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

// Initialize logging
fn init_logging() {
    let _ = env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .try_init();
}

// Tauri command handler: get opportunities
#[tauri::command]
async fn get_opportunities(ipc: tauri::State<'_, Arc<IPCHandler>>) -> Result<String, String> {
    Ok(ipc.handle_get_opportunities().await)
}

// Tauri command handler: get stream status
#[tauri::command]
async fn get_stream_status(ipc: tauri::State<'_, Arc<IPCHandler>>) -> Result<String, String> {
    Ok(ipc.handle_get_stream_status().await)
}

// Tauri command handler: update configuration
#[tauri::command]
async fn update_config(
    geyser_url: String,
    backup_url: String,
    ipc: tauri::State<'_, Arc<IPCHandler>>,
) -> Result<String, String> {
    Ok(ipc.handle_update_config(geyser_url, backup_url).await)
}

// Tauri command handler: get vault configuration
#[tauri::command]
async fn get_vault_config(ipc: tauri::State<'_, Arc<IPCHandler>>) -> Result<String, String> {
    Ok(ipc.handle_get_vault_config().await)
}

#[tokio::main]
async fn main() {
    info!("🚀 Solana Arbitrage Engine v1.0.0 Starting...");
    
    // Initialize vault for encrypted config storage
    let vault = match vault::SecureVault::new().await {
        Ok(v) => {
            info!("✅ Vault initialized");
            Arc::new(v)
        }
        Err(e) => {
            error!("❌ Failed to initialize vault: {}", e);
            std::process::exit(1);
        }
    };
    
    // Initialize arbitrage engine
    let engine = match ArbitrageEngine::new(vault.clone()).await {
        Ok(e) => {
            info!("✅ Arbitrage engine initialized");
            Arc::new(RwLock::new(e))
        }
        Err(e) => {
            error!("❌ Failed to initialize engine: {}", e);
            std::process::exit(1);
        }
    };
    
    // Initialize gRPC stream manager
    let stream_manager = match GeyserStreamManager::new(vault.clone()).await {
        Ok(s) => {
            info!("✅ Stream manager initialized");
            Arc::new(s)
        }
        Err(e) => {
            error!("❌ Failed to initialize stream: {}", e);
            std::process::exit(1);
        }
    };
    
    // Create IPC handler
    let ipc_handler = Arc::new(IPCHandler::new(
        engine,
        stream_manager,
        vault,
    ));
    
    info!("✅ All systems initialized. Starting Tauri application...");
    
    // Build and run Tauri application
    let ipc_for_setup = ipc_handler.clone();
    tauri::Builder::default()
        .manage(ipc_handler)
        .invoke_handler(tauri::generate_handler![
            get_opportunities,
            get_stream_status,
            update_config,
            get_vault_config,
        ])
        .setup(move |_app| {
            info!("✅ Tauri frontend connected successfully");
            
            // Start stream manager in background after Tauri is ready
            let ipc_clone = ipc_for_setup.clone();
            tokio::spawn(async move {
                if let Err(e) = ipc_clone.start_stream().await {
                    error!("⚠️ Stream startup warning: {}", e);
                }
            });
            
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("❌ failed to build tauri application")
        .run(|_app_handle, event| {
            match event {
                tauri::RunEvent::ExitRequested { api, .. } => {
                    api.prevent_exit();
                    info!("🛑 Exit requested, shutting down gracefully...");
                    std::process::exit(0);
                }
                _ => {}
            }
        });
}
