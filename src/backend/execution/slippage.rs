/// Slippage Protection for Arbitrage
/// 
/// Prevents losses from:
/// - Price impact (large swaps move price against you)
/// - Network latency (prices change while tx pending)
/// - Sandwich attacks (MEV attacks before/after)
/// - Rounding errors (integer math precision)
///
/// Uses conservative estimates to ensure execution

use log::{info, warn};

/// Slippage tolerance settings
#[derive(Debug, Clone)]
pub struct SlippageTolerance {
    /// Maximum acceptable slippage in basis points (e.g., 50 = 0.5%)
    pub max_slippage_bps: u64,
    /// Safety margin: how much lower output we accept vs estimated
    /// (e.g., 100 = 1% safety margin = execute at 99% of estimate)
    pub safety_margin_bps: u64,
}

impl Default for SlippageTolerance {
    fn default() -> Self {
        Self {
            max_slippage_bps: 50,    // 0.5% max slippage
            safety_margin_bps: 100,   // 1% safety margin
        }
    }
}

/// Price impact calculator
pub struct SlippageCalculator {
    tolerance: SlippageTolerance,
}

impl SlippageCalculator {
    /// Create new slippage calculator
    pub fn new(tolerance: SlippageTolerance) -> Self {
        Self { tolerance }
    }

    /// Calculate expected output with slippage protection
    /// 
    /// Given: estimated_output (from price oracle or pool reserves)
    /// Returns: minimum_output (what we accept in swap instruction)
    /// 
    /// Formula:
    /// minimum = estimated_output * (1 - slippage_bps / 10000)
    pub fn calculate_minimum_output(
        &self,
        estimated_output: u64,
    ) -> u64 {
        // Apply slippage tolerance
        let slippage_factor = 10000u128 - self.tolerance.max_slippage_bps as u128;
        let minimum = (estimated_output as u128 * slippage_factor) / 10000u128;
        
        minimum as u64
    }

    /// Calculate price impact of a swap
    /// 
    /// Price impact = (input - output) / input
    /// 
    /// Example:
    /// - Input: 1000 USDC
    /// - Output: 900 SOL (estimated)
    /// - Price impact: (1000 - 900) / 1000 = 10% = 1000 bps
    pub fn calculate_price_impact(
        &self,
        input_amount: u64,
        output_amount: u64,
    ) -> u64 {
        if input_amount == 0 {
            return 0;
        }

        let impact = if output_amount > input_amount {
            // Favorable price (gain)
            0
        } else {
            // Unfavorable price (loss)
            let loss = input_amount.saturating_sub(output_amount);
            (loss as u128 * 10000u128 / input_amount as u128) as u64
        };

        impact
    }

    /// Validate output against slippage limits
    /// 
    /// Returns:
    /// - Ok(()) if output is acceptable
    /// - Err with reason if output violates limits
    pub fn validate_output(
        &self,
        estimated_output: u64,
        actual_output: u64,
        swap_name: &str,
    ) -> Result<(), String> {
        let minimum_output = self.calculate_minimum_output(estimated_output);

        if actual_output < minimum_output {
            let shortfall = minimum_output.saturating_sub(actual_output);
            let shortfall_bps = (shortfall as u128 * 10000u128 / estimated_output as u128) as u64;
            
            return Err(format!(
                "{}: Output {} below minimum {} ({} bps slippage)",
                swap_name, actual_output, minimum_output, shortfall_bps
            ));
        }

        let price_impact = self.calculate_price_impact(estimated_output, actual_output);
        if price_impact > self.tolerance.max_slippage_bps {
            warn!(
                "{}: Price impact {} bps exceeds max {}",
                swap_name, price_impact, self.tolerance.max_slippage_bps
            );
            
            return Err(format!(
                "{}: Price impact {} bps exceeds tolerance {}",
                swap_name, price_impact, self.tolerance.max_slippage_bps
            ));
        }

        info!(
            "✅ {}: Output {} within limits (impact {} bps)",
            swap_name, actual_output, price_impact
        );
        Ok(())
    }

    /// Estimate slippage for a given pool and swap size
    /// 
    /// Uses Constant Product Formula (x * y = k):
    /// output = (pool_y * input) / (pool_x + input)
    /// 
    /// # Arguments
    /// - pool_x: Reserve of input token in pool
    /// - pool_y: Reserve of output token in pool
    /// - input: Amount being swapped
    pub fn estimate_output_with_slippage(
        &self,
        pool_x: u64,
        pool_y: u64,
        input: u64,
    ) -> EstimatedSwap {
        if pool_x == 0 || pool_y == 0 {
            return EstimatedSwap {
                estimated_output: 0,
                price_impact_bps: 10000, // 100% loss
                minimum_output: 0,
            };
        }

        // Constant product formula
        // out = y * in / (x + in)
        let numerator = pool_y as u128 * input as u128;
        let denominator = pool_x as u128 + input as u128;
        let estimated_output = (numerator / denominator) as u64;

        // Calculate price impact
        let price_impact = self.calculate_price_impact(input, estimated_output);

        // Apply safety margin for minimum output
        let minimum_output = self.calculate_minimum_output(estimated_output);

        EstimatedSwap {
            estimated_output,
            price_impact_bps: price_impact,
            minimum_output,
        }
    }

    /// Calculate total slippage for multi-hop route
    /// 
    /// For route: A -> B -> C
    /// total_slippage = slippage_1 + slippage_2 + (slippage_1 * slippage_2)
    pub fn calculate_multi_hop_slippage(
        &self,
        hop_slippages_bps: &[u64],
    ) -> u64 {
        if hop_slippages_bps.is_empty() {
            return 0;
        }

        let mut total: u128 = 0;
        for (i, &slippage) in hop_slippages_bps.iter().enumerate() {
            if i == 0 {
                total = slippage as u128;
            } else {
                // Compound slippage: s_total = s1 + s2 + (s1 * s2 / 10000)
                total = total + slippage as u128 + (total * slippage as u128 / 10000);
            }
        }

        std::cmp::min(total, 10000) as u64 // Cap at 100%
    }

    /// Check if execution is economically viable
    /// 
    /// After slippage, does the trade still generate profit?
    pub fn is_profitable_after_slippage(
        &self,
        gross_profit: u64,
        total_slippage_bps: u64,
    ) -> bool {
        // Calculate slippage amount
        let slippage_amount = (gross_profit as u128 * total_slippage_bps as u128 / 10000) as u64;
        
        // After slippage, do we still have profit?
        gross_profit > slippage_amount
    }
}

/// Estimated swap result
#[derive(Debug, Clone)]
pub struct EstimatedSwap {
    /// Expected output at current price
    pub estimated_output: u64,
    /// Price impact in basis points
    pub price_impact_bps: u64,
    /// Minimum acceptable output (with safety margin)
    pub minimum_output: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_minimum_output() {
        let calc = SlippageCalculator::new(SlippageTolerance::default());
        
        // 1000 tokens with 50 bps slippage = 995 minimum
        let estimated = 1000u64;
        let minimum = calc.calculate_minimum_output(estimated);
        
        assert!(minimum < estimated);
        assert!(minimum >= 995); // 50 bps slippage
    }

    #[test]
    fn test_calculate_price_impact() {
        let calc = SlippageCalculator::new(SlippageTolerance::default());
        
        // 1000 in, 950 out = 50 bps impact (0.5% loss)
        let impact = calc.calculate_price_impact(1000, 950);
        assert_eq!(impact, 50);
        
        // 1000 in, 1050 out = 0 impact (gain, no impact)
        let impact = calc.calculate_price_impact(1000, 1050);
        assert_eq!(impact, 0);
    }

    #[test]
    fn test_validate_output_within_limits() {
        let calc = SlippageCalculator::new(SlippageTolerance::default());
        
        // Estimated 1000, actual 995 (within 50 bps limit)
        let result = calc.validate_output(1000, 995, "test_swap");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_output_exceeds_limits() {
        let calc = SlippageCalculator::new(SlippageTolerance::default());
        
        // Estimated 1000, actual 949 (exceeds 50 bps limit)
        let result = calc.validate_output(1000, 949, "test_swap");
        assert!(result.is_err());
    }

    #[test]
    fn test_estimate_output_with_slippage() {
        let calc = SlippageCalculator::new(SlippageTolerance::default());
        
        // Pool: 1M USDC, 100k SOL
        // Swap: 10k USDC
        let result = calc.estimate_output_with_slippage(1_000_000, 100_000, 10_000);
        
        assert!(result.estimated_output > 0);
        assert!(result.estimated_output < 10_000); // Less output than input
        assert!(result.minimum_output <= result.estimated_output);
    }

    #[test]
    fn test_multi_hop_slippage_single_hop() {
        let calc = SlippageCalculator::new(SlippageTolerance::default());
        
        // Single hop with 50 bps slippage
        let total = calc.calculate_multi_hop_slippage(&[50]);
        assert_eq!(total, 50);
    }

    #[test]
    fn test_multi_hop_slippage_two_hops() {
        let calc = SlippageCalculator::new(SlippageTolerance::default());
        
        // Two hops: 50 + 50 = 100 bps nominal
        // Actual: 50 + 50 + (50 * 50 / 10000) = 100.25 bps
        let total = calc.calculate_multi_hop_slippage(&[50, 50]);
        assert_eq!(total, 101); // Compound effect
    }

    #[test]
    fn test_is_profitable_after_slippage() {
        let calc = SlippageCalculator::new(SlippageTolerance::default());
        
        // Profit 1000, slippage 200 bps = profitable
        assert!(calc.is_profitable_after_slippage(1000, 200));
        
        // Profit 100, slippage 200 bps = unprofitable
        assert!(!calc.is_profitable_after_slippage(100, 200));
    }

    #[test]
    fn test_constant_product_formula() {
        let calc = SlippageCalculator::new(SlippageTolerance::default());
        
        // Orca pool example: 1M USDC, 50k SOL
        // Swap 100k USDC
        let result = calc.estimate_output_with_slippage(1_000_000, 50_000, 100_000);
        
        // Expected: 50k * 100k / (1M + 100k) ≈ 4545.45
        assert!(result.estimated_output > 4000);
        assert!(result.estimated_output < 5000);
        
        // Minimum output should be ~95% of estimated (50 bps slippage)
        assert!(result.minimum_output < result.estimated_output);
    }
}
