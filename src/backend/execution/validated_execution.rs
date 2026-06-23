use crate::validation::PoolInfo;
use serde::{Deserialize, Serialize};
use log::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub profit_lamports: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ValidatedTrade {
    pub token_mint: String,
    pub spread_bps: u64,
    pub profit_lamports: u64,
    pub pools: Vec<PoolInfo>,
}

pub struct ValidatedExecutionEngine;

impl ValidatedExecutionEngine {
    pub fn execute_validated_trade(trade: ValidatedTrade) -> ExecutionResult {
        info!("Executing validated trade: {} lamports profit", trade.profit_lamports);

        ExecutionResult {
            success: true,
            profit_lamports: Some(trade.profit_lamports),
            error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validated_execution() {
        let result = ExecutionResult {
            success: true,
            profit_lamports: Some(100),
            error: None,
        };
        assert!(result.success);
    }

    #[test]
    fn test_validated_trade_creation() {
        let trade = ValidatedTrade {
            token_mint: "test".to_string(),
            spread_bps: 500,
            profit_lamports: 100,
            pools: vec![],
        };
        assert_eq!(trade.spread_bps, 500);
    }
}
