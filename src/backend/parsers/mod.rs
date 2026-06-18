// src/backend/parsers/mod.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum AmmType {
    Raydium,
    Orca,
    Meteora,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolData {
    pub address: String,
    pub amm_type: AmmType,
    pub mint_a: String,
    pub mint_b: String,
    
    // AMM state
    pub balance_a: f64,
    pub balance_b: f64,
    pub spot_price: f64,          // Price of B in terms of A
    pub liquidity_usd: f64,
    pub tvl_usd: f64,
    
    // For concentrated liquidity (Orca)
    pub current_tick: Option<i32>,
    pub active_liquidity: Option<f64>,
    
    // For DLMM (Meteora)
    pub active_bin: Option<u64>,
    pub bin_liquidity: Option<f64>,
    
    pub last_updated: i64,        // Unix timestamp
}

impl PoolData {
    pub fn new_raydium(
        address: String,
        mint_a: String,
        mint_b: String,
        balance_a: f64,
        balance_b: f64,
        _decimals_a: u8,
        _decimals_b: u8,
    ) -> Self {
        let spot_price = balance_b / balance_a;
        let liquidity_usd = (balance_a * 190.0) + (balance_b * 1.0);  // Rough USD equiv
        
        PoolData {
            address,
            amm_type: AmmType::Raydium,
            mint_a,
            mint_b,
            balance_a,
            balance_b,
            spot_price,
            liquidity_usd,
            tvl_usd: liquidity_usd * 2.0,  // Rough TVL
            current_tick: None,
            active_liquidity: None,
            active_bin: None,
            bin_liquidity: None,
            last_updated: chrono::Utc::now().timestamp(),
        }
    }
    
    pub fn new_orca(
        address: String,
        mint_a: String,
        mint_b: String,
        spot_price: f64,
        liquidity_usd: f64,
        current_tick: i32,
        active_liquidity: f64,
    ) -> Self {
        PoolData {
            address,
            amm_type: AmmType::Orca,
            mint_a,
            mint_b,
            balance_a: 0.0,  // Not directly tracked in Orca
            balance_b: 0.0,
            spot_price,
            liquidity_usd,
            tvl_usd: liquidity_usd * 2.0,
            current_tick: Some(current_tick),
            active_liquidity: Some(active_liquidity),
            active_bin: None,
            bin_liquidity: None,
            last_updated: chrono::Utc::now().timestamp(),
        }
    }
    
    pub fn new_meteora(
        address: String,
        mint_a: String,
        mint_b: String,
        spot_price: f64,
        liquidity_usd: f64,
        active_bin: u64,
        bin_liquidity: f64,
    ) -> Self {
        PoolData {
            address,
            amm_type: AmmType::Meteora,
            mint_a,
            mint_b,
            balance_a: 0.0,
            balance_b: 0.0,
            spot_price,
            liquidity_usd,
            tvl_usd: liquidity_usd * 2.0,
            current_tick: None,
            active_liquidity: None,
            active_bin: Some(active_bin),
            bin_liquidity: Some(bin_liquidity),
            last_updated: chrono::Utc::now().timestamp(),
        }
    }
}

/// Simulate parsing Raydium pools from account data
pub fn parse_raydium_pool(_account_data: &[u8], address: String) -> Option<PoolData> {
    // In real implementation, deserialize from Raydium account structure
    // For now, return a simulation
    Some(PoolData::new_raydium(
        address,
        "EPjFWaLb3hyccqJ1ddWP63K3GstrpgoxsDQ7KKUUmo".to_string(),  // USDC
        "So11111111111111111111111111111111111111112".to_string(),  // SOL
        5_000_000.0,  // 5M USDC
        25_000.0,     // 25k SOL
        6,
        9,
    ))
}

/// Simulate parsing Orca whirlpool pools
pub fn parse_orca_whirlpool(_account_data: &[u8], address: String) -> Option<PoolData> {
    // In real implementation, deserialize from Orca tick arrays
    Some(PoolData::new_orca(
        address,
        "EPjFWaLb3hyccqJ1ddWP63K3GstrpgoxsDQ7KKUUmo".to_string(),
        "So11111111111111111111111111111111111111112".to_string(),
        200.0,  // USDC/SOL spot price
        8_000_000.0,  // 8M TVL
        0,
        1_000_000.0,
    ))
}

/// Simulate parsing Meteora DLMM pools
pub fn parse_meteora_dlmm(_account_data: &[u8], address: String) -> Option<PoolData> {
    // In real implementation, deserialize from Meteora bin structure
    Some(PoolData::new_meteora(
        address,
        "EPjFWaLb3hyccqJ1ddWP63K3GstrpgoxsDQ7KKUUmo".to_string(),
        "So11111111111111111111111111111111111111112".to_string(),
        198.5,  // Slightly different price
        7_500_000.0,
        1000,
        950_000.0,
    ))
}
