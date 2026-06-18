// src/backend/engine/mod.rs
use crate::vault::SecureVault;
use crate::parsers::{PoolData, AmmType};
use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageOpportunity {
    pub pair: String,
    pub entry_pool: PoolData,
    pub exit_pool: PoolData,
    pub raw_spread_bps: f64,      // basis points
    pub adjusted_capital: f64,      // After 30% gap rule
    pub gross_profit_bps: f64,     // Before fees
    pub net_profit_bps: f64,       // After all fees
    pub profitable: bool,
    pub profit_check: ProfitCheck,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfitCheck {
    pub gross_profit_pct: f64,
    pub jito_tip_pct: f64,
    pub pool_fees_pct: f64,
    pub compute_fees_sol: f64,
    pub net_profit_pct: f64,
    pub meets_floor: bool,         // >= 0.8%
}

pub struct ArbitrageEngine {
    vault: Arc<SecureVault>,
    pools: Arc<tokio::sync::RwLock<HashMap<String, Vec<PoolData>>>>,
    
    // Configuration
    tvl_minimum_usd: f64,
    gap_rule_pct: f64,              // 30% rule
    profit_floor_bps: f64,          // 80 bps = 0.8%
}

impl ArbitrageEngine {
    pub async fn new(vault: Arc<SecureVault>) -> Result<Self, String> {
        Ok(ArbitrageEngine {
            vault,
            pools: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            tvl_minimum_usd: 100_000.0,
            gap_rule_pct: 0.30,
            profit_floor_bps: 80.0,    // 0.8% = 80 basis points
        })
    }
    
    /// Add or update pool data from stream
    pub async fn update_pool(&self, pool: PoolData) {
        let mut pools = self.pools.write().await;
        let pair_key = format!("{}/{}", pool.mint_a, pool.mint_b);
        
        pools.entry(pair_key)
            .or_insert_with(Vec::new)
            .push(pool);
    }
    
    /// Detect arbitrage opportunities across all pools
    pub async fn detect_opportunities(&self) -> Vec<ArbitrageOpportunity> {
        let pools = self.pools.read().await;
        let mut opportunities = Vec::new();
        
        // Group pools by token pair
        let mut pair_map: HashMap<String, Vec<&PoolData>> = HashMap::new();
        
        for pool in pools.values().flatten() {
            // Filter: TVL must be >= $100k
            if pool.tvl_usd < self.tvl_minimum_usd {
                continue;
            }
            
            let pair_key = format!("{}/{}", pool.mint_a, pool.mint_b);
            pair_map.entry(pair_key).or_insert_with(Vec::new).push(pool);
        }
        
        // For each pair, find price gaps
        for (pair_key, pool_group) in pair_map.iter() {
            if pool_group.len() < 2 {
                continue;  // Need at least 2 pools for arbitrage
            }
            
            // Find cheapest and most expensive pools
            let mut sorted = pool_group.clone();
            sorted.sort_by(|a, b| a.spot_price.partial_cmp(&b.spot_price).unwrap());
            
            let cheap_pool = sorted[0];
            let expensive_pool = sorted[sorted.len() - 1];
            
            let spread_pct = (expensive_pool.spot_price - cheap_pool.spot_price) 
                / cheap_pool.spot_price;
            
            // Calculate opportunity
            if let Some(opp) = self.calculate_opportunity(
                pair_key.clone(),
                cheap_pool,
                expensive_pool,
                spread_pct,
            ) {
                if opp.profitable {
                    opportunities.push(opp);
                }
            }
        }
        
        // Sort by net profit
        opportunities.sort_by(|a, b| b.net_profit_bps.partial_cmp(&a.net_profit_bps).unwrap());
        opportunities
    }
    
    /// Calculate detailed arbitrage metrics for a pair
    fn calculate_opportunity(
        &self,
        pair: String,
        cheap_pool: &PoolData,
        expensive_pool: &PoolData,
        spread_pct: f64,
    ) -> Option<ArbitrageOpportunity> {
        // Apply 30% gap rule: limit capital so entry swap impact stays <= 30% of spread
        let max_impact_pct = spread_pct * self.gap_rule_pct;
        let adjusted_capital = self.calculate_max_capital(cheap_pool, max_impact_pct)?;
        
        // Estimate output from cheap pool
        let output_from_cheap = self.estimate_swap_output(cheap_pool, adjusted_capital)?;
        
        // Estimate sale price on expensive pool
        let input_to_expensive = output_from_cheap;
        let gross_return = self.estimate_swap_output(expensive_pool, input_to_expensive)?;
        
        // Gross profit in base units
        let gross_profit_units = gross_return - adjusted_capital;
        let gross_profit_pct = gross_profit_units / adjusted_capital;
        let gross_profit_bps = gross_profit_pct * 10_000.0;
        
        // Subtract fees
        let pool_fee_pct = 0.0025;  // 0.25% typical
        let jito_tip_pct = 0.005;   // 0.5% of gross (conservative)
        let compute_fees_sol = 0.00001;  // ~1 cent
        
        let total_fees_pct = pool_fee_pct + jito_tip_pct;
        let net_profit_pct = gross_profit_pct - total_fees_pct - (compute_fees_sol / adjusted_capital);
        let net_profit_bps = net_profit_pct * 10_000.0;
        
        let meets_floor = net_profit_bps >= self.profit_floor_bps;
        
        Some(ArbitrageOpportunity {
            pair,
            entry_pool: cheap_pool.clone(),
            exit_pool: expensive_pool.clone(),
            raw_spread_bps: spread_pct * 10_000.0,
            adjusted_capital,
            gross_profit_bps,
            net_profit_bps,
            profitable: meets_floor,
            profit_check: ProfitCheck {
                gross_profit_pct,
                jito_tip_pct,
                pool_fees_pct: pool_fee_pct,
                compute_fees_sol,
                net_profit_pct,
                meets_floor,
            },
        })
    }
    
    /// Calculate maximum capital input while respecting 30% gap rule
    fn calculate_max_capital(&self, pool: &PoolData, max_impact_pct: f64) -> Option<f64> {
        // Simplified: assume linear impact for small trades
        // For x*y=k: impact = (amount / liquidity) 
        Some(pool.liquidity_usd * max_impact_pct * 0.5)  // Conservative estimate
    }
    
    /// Estimate output from a swap using AMM formula
    fn estimate_swap_output(&self, pool: &PoolData, input_amount: f64) -> Option<f64> {
        match pool.amm_type {
            AmmType::Raydium => {
                // x * y = k formula
                let k = pool.balance_a * pool.balance_b;
                let new_x = pool.balance_a + input_amount;
                let new_y = k / new_x;
                let output = pool.balance_b - new_y;
                Some(output * (1.0 - 0.0025))  // Apply 0.25% fee
            }
            AmmType::Orca => {
                // Simplified whirlpool calculation
                let output = input_amount * pool.spot_price * (1.0 - 0.0025);
                Some(output)
            }
            AmmType::Meteora => {
                // DLMM: simplified liquidity-weighted calculation
                let output = input_amount * pool.spot_price * (1.0 - 0.0025);
                Some(output)
            }
        }
    }
}
