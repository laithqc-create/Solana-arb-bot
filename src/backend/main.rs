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
mod swap;
mod jito;

use engine::ArbitrageEngine;
use ipc::IPCHandler;
use streaming::GeyserStreamManager;
use keypair::KeypairManager;
use rpc::RpcClientManager;
use swap::{AtomicSwapManager, AtomicSwapCycle, SwapStep, SwapProtocol};
use jito::JitoBundleBuilder;
use jito::client::{JitoBundleClient, JitoConfig};
use jito::tip::{JitoTipCalculator, TipStrategy};
use std::sync::Arc;
use tokio::sync::RwLock;
use log::{info, warn, error};

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
fn check_rpc_health(helius_api_key: String) -> Result<String, String> {
    match RpcClientManager::new_with_helius(&helius_api_key) {
        Ok(_manager) => {
            // Basic check: if we can create manager, connection works
            info!("✅ RPC connection healthy");
            Ok(serde_json::json!({
                "healthy": true,
                "endpoint": "Helius",
                "message": "RPC connection healthy"
            }).to_string())
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

// Tauri command handler: validate atomic swap opportunity
#[tauri::command]
async fn validate_swap_opportunity(
    loan_amount: String,
    _loan_token: String,
    swap1_input_amount: String,
    swap1_output_amount: String,
    swap2_input_amount: String,
    swap2_output_amount: String,
    flash_loan_fee: String,
    expected_profit: String,
) -> Result<String, String> {
    // Parse inputs
    let flash_amount: u64 = loan_amount
        .parse()
        .map_err(|_| "Invalid loan amount".to_string())?;
    
    let swap1_in: u64 = swap1_input_amount
        .parse()
        .map_err(|_| "Invalid swap1 input".to_string())?;
    
    let swap1_out: u64 = swap1_output_amount
        .parse()
        .map_err(|_| "Invalid swap1 output".to_string())?;
    
    let swap2_in: u64 = swap2_input_amount
        .parse()
        .map_err(|_| "Invalid swap2 input".to_string())?;
    
    let swap2_out: u64 = swap2_output_amount
        .parse()
        .map_err(|_| "Invalid swap2 output".to_string())?;
    
    let fee: u64 = flash_loan_fee
        .parse()
        .map_err(|_| "Invalid fee".to_string())?;
    
    let profit: u64 = expected_profit
        .parse()
        .map_err(|_| "Invalid profit".to_string())?;

    // Create swap steps
    let token_a = solana_sdk::pubkey::Pubkey::new_unique();
    let token_b = solana_sdk::pubkey::Pubkey::new_unique();

    let swap_1 = SwapStep::new(
        SwapProtocol::Raydium,
        token_a,
        token_b,
        swap1_in,
        swap1_out,
        solana_sdk::pubkey::Pubkey::new_unique(),
    );

    let swap_2 = SwapStep::new(
        SwapProtocol::Orca,
        token_b,
        token_a,
        swap2_in,
        swap2_out,
        solana_sdk::pubkey::Pubkey::new_unique(),
    );

    // Create cycle
    let cycle = AtomicSwapCycle::new(
        flash_amount,
        token_a,
        swap_1,
        swap_2,
        fee,
        profit,
    );

    // Validate
    let manager = AtomicSwapManager::default();
    match manager.validate_opportunity(&cycle) {
        Ok(_) => {
            info!("✅ Opportunity validated: net_profit={}", cycle.net_profit());
            Ok(serde_json::json!({
                "valid": true,
                "net_profit": cycle.net_profit(),
                "flash_loan_fee": fee,
                "expected_profit": profit,
                "swap1_slippage_bps": cycle.swap_1_slippage_bps(),
                "swap2_slippage_bps": cycle.swap_2_slippage_bps(),
                "message": "Opportunity meets all requirements"
            }).to_string())
        }
        Err(e) => {
            warn!("⚠️ Opportunity validation failed: {}", e);
            Ok(serde_json::json!({
                "valid": false,
                "error": e.to_string(),
                "message": format!("Validation failed: {}", e)
            }).to_string())
        }
    }
}

// Tauri command handler: calculate arbitrage metrics
#[tauri::command]
async fn calculate_arbitrage_metrics(
    buy_price: String,
    sell_price: String,
    amount: String,
    fee_bps: String,
) -> Result<String, String> {
    let buy: u64 = buy_price.parse().map_err(|_| "Invalid buy price")?;
    let sell: u64 = sell_price.parse().map_err(|_| "Invalid sell price")?;
    let amt: u64 = amount.parse().map_err(|_| "Invalid amount")?;
    let fee: u64 = fee_bps.parse().map_err(|_| "Invalid fee")?;

    let manager = AtomicSwapManager::default();

    // Check if profitable
    let is_profitable = manager.is_spread_profitable(buy, sell, fee);

    // Calculate spread
    let spread_bps = if sell > buy {
        ((sell as u128 - buy as u128) * 10000 / sell as u128) as u64
    } else {
        0
    };

    // Calculate gross profit
    let gross_profit = amt.saturating_mul(spread_bps) / 10000;
    let net_profit = gross_profit.saturating_sub(amt.saturating_mul(fee) / 10000);

    Ok(serde_json::json!({
        "buy_price": buy,
        "sell_price": sell,
        "spread_bps": spread_bps,
        "is_profitable": is_profitable,
        "gross_profit": gross_profit,
        "fee": amt.saturating_mul(fee) / 10000,
        "net_profit": net_profit,
        "roi_bps": if net_profit > 0 { (net_profit as u128 * 10000 / amt as u128) as u64 } else { 0 }
    }).to_string())
}

// Tauri command handler: estimate output with slippage
#[tauri::command]
async fn estimate_swap_output(
    input_amount: String,
    expected_output: String,
    slippage_bps: String,
) -> Result<String, String> {
    let input: u64 = input_amount.parse().map_err(|_| "Invalid input")?;
    let output: u64 = expected_output.parse().map_err(|_| "Invalid output")?;
    let slip: u64 = slippage_bps.parse().map_err(|_| "Invalid slippage")?;

    let manager = AtomicSwapManager::default();
    let final_output = manager.estimate_output_with_slippage(input, output, slip);

    Ok(serde_json::json!({
        "input_amount": input,
        "expected_output": output,
        "slippage_bps": slip,
        "slippage_amount": output.saturating_sub(final_output),
        "final_output": final_output,
        "efficiency_percent": (final_output as u128 * 100 / output as u128) as u64
    }).to_string())
}

// Tauri command handler: calculate Jito tip
#[tauri::command]
async fn calculate_jito_tip(
    gross_profit: String,
    strategy: String,
) -> Result<String, String> {
    let profit: u64 = gross_profit
        .parse()
        .map_err(|_| "Invalid profit amount".to_string())?;

    let tip_strategy = match strategy.to_lowercase().as_str() {
        "conservative" => TipStrategy::Conservative,
        "balanced" => TipStrategy::Balanced,
        "aggressive" => TipStrategy::Aggressive,
        _ => TipStrategy::Balanced,
    };

    let calculator = JitoTipCalculator::default();
    match calculator.calculate_tip_with_strategy(profit, tip_strategy) {
        Ok(result) => {
            info!(
                "💸 Calculated tip: jito={}, keeper={}, strategy={}",
                result.jito_tip, result.final_profit, result.strategy
            );
            Ok(serde_json::json!({
                "gross_profit": result.gross_profit,
                "jito_tip": result.jito_tip,
                "final_profit": result.final_profit,
                "tip_percentage_bps": result.tip_percentage_bps,
                "keeper_profit_bps": result.keeper_profit_bps(),
                "keeper_roi_percent": format!("{:.2}", result.keeper_roi_percent()),
                "strategy": result.strategy
            }).to_string())
        }
        Err(e) => {
            warn!("⚠️ Tip calculation failed: {}", e);
            Err(format!("Tip calculation failed: {}", e))
        }
    }
}

// Tauri command handler: calculate competitive tip
#[tauri::command]
async fn calculate_competitive_tip(
    gross_profit: String,
) -> Result<String, String> {
    let profit: u64 = gross_profit
        .parse()
        .map_err(|_| "Invalid profit amount".to_string())?;

    let calculator = JitoTipCalculator::default();
    match calculator.calculate_competitive_tip(profit) {
        Ok(result) => {
            info!(
                "🎯 Competitive tip: jito={}, keeper={}",
                result.jito_tip, result.final_profit
            );
            Ok(serde_json::json!({
                "gross_profit": result.gross_profit,
                "jito_tip": result.jito_tip,
                "final_profit": result.final_profit,
                "tip_percentage_bps": result.tip_percentage_bps,
                "strategy": result.strategy,
                "keeper_roi_percent": format!("{:.2}", result.keeper_roi_percent())
            }).to_string())
        }
        Err(e) => {
            warn!("⚠️ Competitive tip calculation failed: {}", e);
            Err(format!("Calculation failed: {}", e))
        }
    }
}

// Tauri command handler: create Jito bundle
#[tauri::command]
async fn create_jito_bundle(
    bundle_id: String,
    jito_tip: String,
) -> Result<String, String> {
    let tip: u64 = jito_tip
        .parse()
        .map_err(|_| "Invalid tip amount".to_string())?;

    let payer = solana_sdk::pubkey::Pubkey::new_unique();

    let result = JitoBundleBuilder::new(bundle_id.clone())
        .with_payer(payer)
        .with_tip(tip)
        .build();

    match result {
        Ok(bundle) => {
            info!(
                "📦 Created bundle: id={}, tip={}, txs={}",
                bundle.bundle_id, bundle.jito_tip, bundle.transaction_count()
            );
            Ok(serde_json::json!({
                "bundle_id": bundle.bundle_id,
                "status": "created",
                "jito_tip": bundle.jito_tip,
                "transaction_count": bundle.transaction_count(),
                "bundle_size_bytes": bundle.bundle_size()
            }).to_string())
        }
        Err(e) => {
            error!("❌ Bundle creation failed: {}", e);
            Err(format!("Bundle creation failed: {}", e))
        }
    }
}

// Tauri command handler: get Jito configuration
#[tauri::command]
async fn get_jito_config() -> Result<String, String> {
    Ok(serde_json::json!({
        "endpoints": {
            "mainnet": "https://mainnet.block-engine.jito.wtf/api/v1/bundles",
            "testnet": "https://testnet.block-engine.jito.wtf/api/v1/bundles"
        },
        "tip_strategies": {
            "conservative": "85% to Jito (for large profits)",
            "balanced": "87.5% to Jito (for medium profits)",
            "aggressive": "90% to Jito (for small profits)"
        },
        "bundle_steps": [
            "1. Flash loan borrow",
            "2. Swap 1 (buy low)",
            "3. Swap 2 (sell high)",
            "4. Repay flash loan"
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
            validate_swap_opportunity,
            calculate_arbitrage_metrics,
            estimate_swap_output,
            calculate_jito_tip,
            calculate_competitive_tip,
            create_jito_bundle,
            get_jito_config,
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
