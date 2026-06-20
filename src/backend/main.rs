// src/backend/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod engine;
mod parsers;
mod vault;
mod ipc;
mod streaming;
mod flash_loan;
mod keypair;
mod rpc;

use engine::ArbitrageEngine;
use ipc::IPCHandler;
use streaming::GeyserStreamManager;
use flash_loan::FlashLoanManager;
use keypair::KeypairManager;
use rpc::{RpcClientManager, RpcConfig};
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

// Tauri command handler: get flash loan fee
#[tauri::command]
async fn get_flash_loan_fee(protocol: String, amount: String) -> Result<String, String> {
    let amount_u64: u64 = amount.parse()
        .map_err(|_| "Invalid amount format".to_string())?;
    
    let calculator = flash_loan::FlashLoanFeeCalculator::new();
    let fee = calculator.calculate_fee(&protocol, amount_u64)
        .map_err(|e| format!("Fee calculation error: {}", e))?;
    
    Ok(serde_json::json!({
        "protocol": protocol,
        "amount": amount_u64,
        "fee": fee,
        "fee_percentage": flash_loan::FlashLoanFeeCalculator::format_fee_percentage(
            calculator.get_protocol_info(&protocol)
                .map(|info| info.fee_bps)
                .unwrap_or(0)
        )
    }).to_string())
}

// Tauri command handler: get supported flash loan protocols
#[tauri::command]
async fn get_flash_loan_protocols() -> Result<String, String> {
    let calculator = flash_loan::FlashLoanFeeCalculator::new();
    let protocols = calculator.get_supported_protocols();
    
    let protocol_infos: Result<Vec<_>, _> = protocols
        .iter()
        .map(|p| calculator.get_protocol_info(p))
        .collect();
    
    match protocol_infos {
        Ok(infos) => Ok(serde_json::to_string(&infos).unwrap_or_default()),
        Err(e) => Err(format!("Failed to get protocol info: {}", e)),
    }
}

// Tauri command handler: load keypair from environment
#[tauri::command]
async fn load_keypair_from_env() -> Result<String, String> {
    match KeypairManager::load_from_env() {
        Ok(keypair) => {
            let pubkey = keypair.pubkey_string();
            info!("✅ Keypair loaded from environment: {}", pubkey);
            Ok(serde_json::json!({
                "success": true,
                "public_key": pubkey,
                "message": "Keypair loaded successfully"
            }).to_string())
        }
        Err(e) => {
            error!("❌ Failed to load keypair: {}", e);
            Err(format!("Keypair load failed: {}", e))
        }
    }
}

// Tauri command handler: load keypair with fallback
#[tauri::command]
async fn load_keypair_with_fallback() -> Result<String, String> {
    match KeypairManager::load_with_fallback() {
        Ok(keypair) => {
            let pubkey = keypair.pubkey_string();
            let source = keypair.source().to_string_lossy().to_string();
            info!("✅ Keypair loaded from: {}", source);
            Ok(serde_json::json!({
                "success": true,
                "public_key": pubkey,
                "source": source,
                "message": "Keypair loaded successfully with fallback"
            }).to_string())
        }
        Err(e) => {
            error!("❌ Failed to load keypair: {}", e);
            Err(format!("Keypair load failed: {}", e))
        }
    }
}

// Tauri command handler: estimate balance requirement
#[tauri::command]
async fn estimate_balance_requirement(
    expected_profit_lamports: String,
    num_executions: String,
) -> Result<String, String> {
    let profit: u64 = expected_profit_lamports
        .parse()
        .map_err(|_| "Invalid profit format".to_string())?;
    
    let executions: u64 = num_executions
        .parse()
        .map_err(|_| "Invalid execution count".to_string())?;

    let required = KeypairManager::estimate_required_balance(profit, executions);
    
    let required_sol = required as f64 / 1_000_000_000.0; // Convert to SOL

    Ok(serde_json::json!({
        "required_lamports": required,
        "required_sol": format!("{:.6}", required_sol),
        "profit_per_execution": profit,
        "num_executions": executions
    }).to_string())
}

// Tauri command handler: initialize RPC manager
#[tauri::command]
async fn initialize_rpc(helius_api_key: String) -> Result<String, String> {
    match RpcClientManager::new_with_helius(&helius_api_key) {
        Ok(manager) => {
            let endpoints = manager.get_endpoints();
            info!("✅ RPC Manager initialized with {} endpoints", endpoints.len());
            Ok(serde_json::json!({
                "success": true,
                "endpoints": endpoints,
                "message": "RPC manager initialized successfully"
            }).to_string())
        }
        Err(e) => {
            error!("❌ Failed to initialize RPC: {}", e);
            Err(format!("RPC initialization failed: {}", e))
        }
    }
}

// Tauri command handler: check RPC health
#[tauri::command]
async fn check_rpc_health(helius_api_key: String) -> Result<String, String> {
    match RpcClientManager::new_with_helius(&helius_api_key) {
        Ok(manager) => {
            match manager.check_health().await {
                Ok(_) => {
                    let endpoint = manager.get_current_endpoint_name().await;
                    Ok(serde_json::json!({
                        "healthy": true,
                        "endpoint": endpoint,
                        "message": "RPC connection healthy"
                    }).to_string())
                }
                Err(e) => {
                    warn!("⚠️ RPC health check failed: {}", e);
                    Ok(serde_json::json!({
                        "healthy": false,
                        "error": e.to_string(),
                        "message": "RPC connection unhealthy"
                    }).to_string())
                }
            }
        }
        Err(e) => {
            error!("❌ Failed to check RPC health: {}", e);
            Err(format!("RPC health check failed: {}", e))
        }
    }
}

// Tauri command handler: get RPC configuration options
#[tauri::command]
async fn get_rpc_config_info() -> Result<String, String> {
    Ok(serde_json::json!({
        "networks": ["mainnet", "testnet", "devnet"],
        "helius_url": "https://www.helius.dev",
        "helius_free_tier": "100K requests/day",
        "environment_variables": {
            "HELIUS_API_KEY": "Required for mainnet/testnet",
            "SOLANA_NETWORK": "Optional: mainnet (default), testnet, or devnet"
        },
        "setup_steps": [
            "1. Go to https://www.helius.dev",
            "2. Sign up for free account",
            "3. Create project → copy API key",
            "4. Set HELIUS_API_KEY environment variable",
            "5. Restart application"
        ]
    }).to_string())
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
            get_flash_loan_fee,
            get_flash_loan_protocols,
            load_keypair_from_env,
            load_keypair_with_fallback,
            estimate_balance_requirement,
            initialize_rpc,
            check_rpc_health,
            get_rpc_config_info,
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
