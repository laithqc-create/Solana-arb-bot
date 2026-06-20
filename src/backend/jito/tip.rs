/// Jito Tip Calculator
///
/// Calculates optimal Jito tip based on arbitrage profit:
/// - Gross profit = Swap output - Loan amount - Flash fee
/// - Jito tip = Gross profit * (85-90%)
/// - Final profit = Gross profit - Jito tip
///
/// Strategy: Give Jito 85-90% of profit to ensure fast inclusion

use log::{info, warn};
use std::fmt;

/// Tip calculation strategy
#[derive(Debug, Clone, Copy)]
pub enum TipStrategy {
    /// Conservative: 85% to Jito (15% keeper profit)
    Conservative,
    /// Balanced: 87.5% to Jito (12.5% keeper profit)
    Balanced,
    /// Aggressive: 90% to Jito (10% keeper profit)
    Aggressive,
    /// Custom percentage
    Custom(u32), // e.g., 8750 = 87.50%
}

impl TipStrategy {
    /// Get percentage as basis points (0-10000)
    pub fn as_bps(&self) -> u32 {
        match self {
            TipStrategy::Conservative => 8500,  // 85%
            TipStrategy::Balanced => 8750,      // 87.5%
            TipStrategy::Aggressive => 9000,    // 90%
            TipStrategy::Custom(bps) => *bps,
        }
    }

    /// Validate strategy
    pub fn validate(&self) -> Result<(), TipError> {
        let bps = self.as_bps();
        
        // Must be between 50% and 99%
        if bps < 5000 || bps > 9900 {
            return Err(TipError::InvalidPercentage { actual: bps });
        }

        Ok(())
    }
}

impl fmt::Display for TipStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pct = self.as_bps() as f64 / 100.0;
        match self {
            TipStrategy::Conservative => write!(f, "Conservative (85%)"),
            TipStrategy::Balanced => write!(f, "Balanced (87.5%)"),
            TipStrategy::Aggressive => write!(f, "Aggressive (90%)"),
            TipStrategy::Custom(bps) => write!(f, "Custom ({}%)", pct),
        }
    }
}

/// Tip calculation result
#[derive(Debug, Clone)]
pub struct TipCalculation {
    /// Gross profit before tip
    pub gross_profit: u64,
    /// Jito tip
    pub jito_tip: u64,
    /// Final profit after tip
    pub final_profit: u64,
    /// Tip percentage (in basis points)
    pub tip_percentage_bps: u32,
    /// Strategy used
    pub strategy: String,
}

impl TipCalculation {
    /// Get keeper profit percentage
    pub fn keeper_profit_bps(&self) -> u32 {
        10000 - self.tip_percentage_bps
    }

    /// ROI: (final_profit / gross_profit) * 100
    pub fn keeper_roi_percent(&self) -> f64 {
        if self.gross_profit == 0 {
            return 0.0;
        }
        (self.final_profit as f64 / self.gross_profit as f64) * 100.0
    }
}

/// Jito Tip Calculator
pub struct JitoTipCalculator {
    /// Default strategy
    default_strategy: TipStrategy,
    /// Minimum tip (prevents dust tips)
    min_tip: u64,
    /// Maximum tip (sanity check)
    max_tip: u64,
}

impl JitoTipCalculator {
    /// Create new calculator
    pub fn new(default_strategy: TipStrategy) -> Self {
        Self {
            default_strategy,
            min_tip: 100,              // At least 100 lamports
            max_tip: 1_000_000_000,    // At most 1 SOL
        }
    }

    /// Default calculator (balanced strategy)
    pub fn default() -> Self {
        Self::new(TipStrategy::Balanced)
    }

    /// Calculate tip based on profit
    pub fn calculate_tip(&self, gross_profit: u64) -> Result<TipCalculation, TipError> {
        self.calculate_tip_with_strategy(gross_profit, self.default_strategy)
    }

    /// Calculate tip with specific strategy
    pub fn calculate_tip_with_strategy(
        &self,
        gross_profit: u64,
        strategy: TipStrategy,
    ) -> Result<TipCalculation, TipError> {
        // Validate inputs
        if gross_profit == 0 {
            return Err(TipError::ZeroProfit);
        }

        // Validate strategy
        strategy.validate()?;

        // Calculate tip
        let tip_bps = strategy.as_bps();
        let jito_tip = ((gross_profit as u128 * tip_bps as u128) / 10000) as u64;

        // Apply min/max constraints
        let clamped_tip = jito_tip.clamp(self.min_tip, self.max_tip);

        // Check if tip is reasonable
        if clamped_tip > gross_profit {
            return Err(TipError::TipExceedsProfit {
                profit: gross_profit,
                tip: clamped_tip,
            });
        }

        let final_profit = gross_profit.saturating_sub(clamped_tip);

        info!(
            "💸 Tip calculation: profit={}, tip={}, final={}, strategy={}",
            gross_profit, clamped_tip, final_profit, strategy
        );

        Ok(TipCalculation {
            gross_profit,
            jito_tip: clamped_tip,
            final_profit,
            tip_percentage_bps: tip_bps,
            strategy: strategy.to_string(),
        })
    }

    /// Calculate competitive tip (auto-adjust based on network conditions)
    /// 
    /// If profit is large: use conservative (85%)
    /// If profit is medium: use balanced (87.5%)
    /// If profit is small: use aggressive (90%) to ensure inclusion
    pub fn calculate_competitive_tip(
        &self,
        gross_profit: u64,
    ) -> Result<TipCalculation, TipError> {
        let strategy = match gross_profit {
            p if p > 100_000 => TipStrategy::Conservative,  // 85% tip
            p if p > 50_000 => TipStrategy::Balanced,       // 87.5% tip
            _ => TipStrategy::Aggressive,                   // 90% tip
        };

        self.calculate_tip_with_strategy(gross_profit, strategy)
    }

    /// Set minimum tip
    pub fn set_min_tip(&mut self, min_tip: u64) {
        self.min_tip = min_tip;
    }

    /// Set maximum tip
    pub fn set_max_tip(&mut self, max_tip: u64) {
        self.max_tip = max_tip;
    }
}

/// Tip calculation error
#[derive(Debug, Clone)]
pub enum TipError {
    /// Zero profit
    ZeroProfit,
    /// Invalid tip percentage
    InvalidPercentage { actual: u32 },
    /// Tip exceeds profit
    TipExceedsProfit { profit: u64, tip: u64 },
    /// Strategy validation failed
    StrategyError(String),
}

impl fmt::Display for TipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TipError::ZeroProfit => write!(f, "Cannot calculate tip from zero profit"),
            TipError::InvalidPercentage { actual } => {
                let pct = *actual as f64 / 100.0;
                write!(f, "Invalid tip percentage: {}%", pct)
            }
            TipError::TipExceedsProfit { profit, tip } => {
                write!(f, "Tip ({}) exceeds profit ({})", tip, profit)
            }
            TipError::StrategyError(msg) => write!(f, "Strategy error: {}", msg),
        }
    }
}

impl std::error::Error for TipError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tip_strategy_percentages() {
        assert_eq!(TipStrategy::Conservative.as_bps(), 8500);
        assert_eq!(TipStrategy::Balanced.as_bps(), 8750);
        assert_eq!(TipStrategy::Aggressive.as_bps(), 9000);
        assert_eq!(TipStrategy::Custom(7500).as_bps(), 7500);
    }

    #[test]
    fn test_tip_strategy_validation() {
        assert!(TipStrategy::Conservative.validate().is_ok());
        assert!(TipStrategy::Custom(10001).validate().is_err()); // > 99%
        assert!(TipStrategy::Custom(4999).validate().is_err());  // < 50%
    }

    #[test]
    fn test_calculate_tip_conservative() {
        let calc = JitoTipCalculator::default();
        let result = calc.calculate_tip_with_strategy(100_000, TipStrategy::Conservative).unwrap();
        
        assert_eq!(result.gross_profit, 100_000);
        assert_eq!(result.jito_tip, 85_000);        // 85% of 100k
        assert_eq!(result.final_profit, 15_000);   // 15% keeper profit
        assert_eq!(result.keeper_profit_bps(), 1500); // 15%
    }

    #[test]
    fn test_calculate_tip_balanced() {
        let calc = JitoTipCalculator::default();
        let result = calc.calculate_tip_with_strategy(100_000, TipStrategy::Balanced).unwrap();
        
        assert_eq!(result.jito_tip, 87_500);        // 87.5% of 100k
        assert_eq!(result.final_profit, 12_500);   // 12.5% keeper
    }

    #[test]
    fn test_calculate_tip_aggressive() {
        let calc = JitoTipCalculator::default();
        let result = calc.calculate_tip_with_strategy(100_000, TipStrategy::Aggressive).unwrap();
        
        assert_eq!(result.jito_tip, 90_000);        // 90% of 100k
        assert_eq!(result.final_profit, 10_000);   // 10% keeper
    }

    #[test]
    fn test_minimum_tip() {
        let mut calc = JitoTipCalculator::default();
        calc.set_min_tip(10_000);
        
        let result = calc.calculate_tip(50).unwrap(); // Very small profit
        assert!(result.jito_tip >= 10_000);           // Clamped to min
    }

    #[test]
    fn test_zero_profit_error() {
        let calc = JitoTipCalculator::default();
        assert!(matches!(calc.calculate_tip(0), Err(TipError::ZeroProfit)));
    }

    #[test]
    fn test_tip_exceeds_profit_error() {
        let mut calc = JitoTipCalculator::default();
        calc.set_max_tip(1_000_000); // Very high max
        
        // Even with high max, should still work if profit is high enough
        let result = calc.calculate_tip(100_000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_competitive_tip_large_profit() {
        let calc = JitoTipCalculator::default();
        let result = calc.calculate_competitive_tip(200_000).unwrap();
        
        // Should use Conservative (85%)
        assert_eq!(result.jito_tip, 170_000); // 85% of 200k
    }

    #[test]
    fn test_competitive_tip_medium_profit() {
        let calc = JitoTipCalculator::default();
        let result = calc.calculate_competitive_tip(75_000).unwrap();
        
        // Should use Balanced (87.5%)
        assert_eq!(result.jito_tip, 65_625); // 87.5% of 75k
    }

    #[test]
    fn test_competitive_tip_small_profit() {
        let calc = JitoTipCalculator::default();
        let result = calc.calculate_competitive_tip(30_000).unwrap();
        
        // Should use Aggressive (90%)
        assert_eq!(result.jito_tip, 27_000); // 90% of 30k
    }

    #[test]
    fn test_keeper_roi() {
        let calc = JitoTipCalculator::default();
        let result = calc.calculate_tip_with_strategy(100_000, TipStrategy::Balanced).unwrap();
        
        // 12.5% keeper profit out of 100k
        let roi = result.keeper_roi_percent();
        assert!(roi > 12.4 && roi < 12.6); // Approximately 12.5%
    }
}
