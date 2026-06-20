/// Flash Loan Fee Calculator
/// 
/// Calculates fees for different DeFi protocols
/// Used to determine profitability of arbitrage trades

/// Flash loan fees by protocol (in basis points, where 1 bps = 0.01%)
const ORCA_FLASH_LOAN_FEE_BPS: u64 = 275; // 0.0275%
const RAYDIUM_FLASH_LOAN_FEE_BPS: u64 = 500; // 0.05%
const SOLEND_FLASH_LOAN_FEE_BPS: u64 = 900; // 0.09%
const MARINADE_FLASH_LOAN_FEE_BPS: u64 = 100; // 0.01% (stSOL only)

pub struct FlashLoanFeeCalculator;

impl FlashLoanFeeCalculator {
    pub fn new() -> Self {
        Self
    }

    /// Calculate flash loan fee for a protocol
    ///
    /// # Arguments
    /// * `protocol` - Protocol name: "orca", "raydium", "solend", "marinade"
    /// * `amount` - Amount to borrow in smallest unit
    ///
    /// # Returns
    /// Fee amount in smallest unit
    pub fn calculate_fee(&self, protocol: &str, amount: u64) -> Result<u64, String> {
        let fee_bps = match protocol.to_lowercase().as_str() {
            "orca" => ORCA_FLASH_LOAN_FEE_BPS,
            "raydium" => RAYDIUM_FLASH_LOAN_FEE_BPS,
            "solend" => SOLEND_FLASH_LOAN_FEE_BPS,
            "marinade" => MARINADE_FLASH_LOAN_FEE_BPS,
            _ => return Err(format!("Unknown protocol: {}", protocol)),
        };

        // Fee = amount * (fee_bps / 10000)
        // We use u64 to avoid floating point precision issues
        Ok((amount / 10000) * fee_bps)
    }

    /// Get protocol information including fee
    pub fn get_protocol_info(&self, protocol: &str) -> Result<ProtocolInfo, String> {
        match protocol.to_lowercase().as_str() {
            "orca" => Ok(ProtocolInfo {
                name: "Orca".to_string(),
                fee_bps: ORCA_FLASH_LOAN_FEE_BPS,
                min_amount: 1_000, // $0.001
                max_amount: u64::MAX,
                description: "Low fee flash loans with excellent liquidity".to_string(),
                supported: true,
            }),
            "raydium" => Ok(ProtocolInfo {
                name: "Raydium".to_string(),
                fee_bps: RAYDIUM_FLASH_LOAN_FEE_BPS,
                min_amount: 1_000,
                max_amount: u64::MAX,
                description: "High liquidity, moderate fee".to_string(),
                supported: true,
            }),
            "solend" => Ok(ProtocolInfo {
                name: "Solend".to_string(),
                fee_bps: SOLEND_FLASH_LOAN_FEE_BPS,
                min_amount: 1_000,
                max_amount: u64::MAX,
                description: "Lending protocol with flash loan support".to_string(),
                supported: true,
            }),
            "marinade" => Ok(ProtocolInfo {
                name: "Marinade".to_string(),
                fee_bps: MARINADE_FLASH_LOAN_FEE_BPS,
                min_amount: 1_000,
                max_amount: u64::MAX,
                description: "Lowest fee for stSOL (liquid staking)".to_string(),
                supported: true,
            }),
            _ => Err(format!("Unknown protocol: {}", protocol)),
        }
    }

    /// Get all supported protocols
    pub fn get_supported_protocols(&self) -> Vec<String> {
        vec![
            "orca".to_string(),
            "raydium".to_string(),
            "solend".to_string(),
            "marinade".to_string(),
        ]
    }

    /// Find cheapest protocol for given amount
    pub fn find_cheapest_protocol(&self, amount: u64) -> Result<String, String> {
        let protocols = self.get_supported_protocols();
        let mut cheapest_protocol = "orca".to_string();
        let mut cheapest_fee = u64::MAX;

        for protocol in protocols {
            if let Ok(fee) = self.calculate_fee(&protocol, amount) {
                if fee < cheapest_fee {
                    cheapest_fee = fee;
                    cheapest_protocol = protocol;
                }
            }
        }

        Ok(cheapest_protocol)
    }

    /// Convert basis points to percentage string
    pub fn format_fee_percentage(fee_bps: u64) -> String {
        let whole = fee_bps / 100;
        let decimal = fee_bps % 100;
        format!("{}%", whole as f64 + (decimal as f64 / 100.0))
    }
}

/// Protocol information
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProtocolInfo {
    pub name: String,
    pub fee_bps: u64,
    pub min_amount: u64,
    pub max_amount: u64,
    pub description: String,
    pub supported: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orca_fee_calculation() {
        let calculator = FlashLoanFeeCalculator::new();
        let amount = 1_000_000; // 1M lamports
        let fee = calculator.calculate_fee("orca", amount).unwrap();
        
        // 0.0275% of 1M = 275 lamports
        assert_eq!(fee, 275);
    }

    #[test]
    fn test_raydium_fee_calculation() {
        let calculator = FlashLoanFeeCalculator::new();
        let amount = 1_000_000;
        let fee = calculator.calculate_fee("raydium", amount).unwrap();
        
        // 0.05% of 1M = 500 lamports
        assert_eq!(fee, 500);
    }

    #[test]
    fn test_marinade_fee_calculation() {
        let calculator = FlashLoanFeeCalculator::new();
        let amount = 1_000_000;
        let fee = calculator.calculate_fee("marinade", amount).unwrap();
        
        // 0.01% of 1M = 100 lamports
        assert_eq!(fee, 100);
    }

    #[test]
    fn test_invalid_protocol() {
        let calculator = FlashLoanFeeCalculator::new();
        let result = calculator.calculate_fee("invalid", 1_000_000);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_cheapest_protocol() {
        let calculator = FlashLoanFeeCalculator::new();
        let amount = 1_000_000;
        let cheapest = calculator.find_cheapest_protocol(amount).unwrap();
        
        // Marinade should be cheapest (0.01%)
        assert_eq!(cheapest, "marinade");
    }

    #[test]
    fn test_format_fee_percentage() {
        assert_eq!(
            FlashLoanFeeCalculator::format_fee_percentage(275),
            "2.75%"
        );
        assert_eq!(
            FlashLoanFeeCalculator::format_fee_percentage(100),
            "1%"
        );
        assert_eq!(
            FlashLoanFeeCalculator::format_fee_percentage(500),
            "5%"
        );
    }

    #[test]
    fn test_get_protocol_info() {
        let calculator = FlashLoanFeeCalculator::new();
        let info = calculator.get_protocol_info("orca").unwrap();
        
        assert_eq!(info.name, "Orca");
        assert_eq!(info.fee_bps, 275);
        assert!(info.supported);
    }

    #[test]
    fn test_get_supported_protocols() {
        let calculator = FlashLoanFeeCalculator::new();
        let protocols = calculator.get_supported_protocols();
        
        assert!(protocols.contains(&"orca".to_string()));
        assert!(protocols.contains(&"raydium".to_string()));
        assert!(protocols.contains(&"solend".to_string()));
        assert!(protocols.contains(&"marinade".to_string()));
        assert_eq!(protocols.len(), 4);
    }

    #[test]
    fn test_large_amount_fee() {
        let calculator = FlashLoanFeeCalculator::new();
        let amount = 1_000_000_000_000; // 1 trillion lamports
        let fee = calculator.calculate_fee("orca", amount).unwrap();
        
        // 0.0275% of 1T = 275,000,000 lamports
        assert_eq!(fee, 275_000_000);
    }
}
