/// Atomic Swap Logic for Cross-DEX Arbitrage
///
/// Handles the core execution flow:
/// 1. Borrow flash loan
/// 2. Execute swap A → B on DEX 1
/// 3. Execute swap B → A on DEX 2
/// 4. Repay flash loan + fee
/// 5. Profit to wallet
///
/// All steps atomic: if any fails, entire transaction reverts

use solana_sdk::pubkey::Pubkey;
use std::fmt;
use log::{info, warn};

/// Swap protocol identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SwapProtocol {
    Raydium,
    Orca,
    Marinade,
}

impl fmt::Display for SwapProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SwapProtocol::Raydium => write!(f, "Raydium"),
            SwapProtocol::Orca => write!(f, "Orca"),
            SwapProtocol::Marinade => write!(f, "Marinade"),
        }
    }
}

impl SwapProtocol {
    pub fn name(&self) -> &'static str {
        match self {
            SwapProtocol::Raydium => "Raydium",
            SwapProtocol::Orca => "Orca",
            SwapProtocol::Marinade => "Marinade",
        }
    }
}

/// Single swap instruction
#[derive(Debug, Clone)]
pub struct SwapStep {
    /// Protocol being used
    pub protocol: SwapProtocol,
    /// Input token mint
    pub input_mint: Pubkey,
    /// Output token mint
    pub output_mint: Pubkey,
    /// Amount to swap (in base units)
    pub input_amount: u64,
    /// Minimum expected output (with slippage)
    pub min_output_amount: u64,
    /// Pool/market identifier
    pub pool_id: Pubkey,
}

impl SwapStep {
    pub fn new(
        protocol: SwapProtocol,
        input_mint: Pubkey,
        output_mint: Pubkey,
        input_amount: u64,
        min_output_amount: u64,
        pool_id: Pubkey,
    ) -> Self {
        Self {
            protocol,
            input_mint,
            output_mint,
            input_amount,
            min_output_amount,
            pool_id,
        }
    }

    /// Validate swap step
    pub fn validate(&self) -> Result<(), SwapError> {
        // Check that input and output are different
        if self.input_mint == self.output_mint {
            return Err(SwapError::SameInputOutput);
        }

        // Check amounts are positive
        if self.input_amount == 0 {
            return Err(SwapError::ZeroAmount);
        }

        // Check min output is reasonable (not zero)
        if self.min_output_amount == 0 {
            return Err(SwapError::ZeroMinOutput);
        }

        Ok(())
    }
}

/// Complete arbitrage cycle
#[derive(Debug, Clone)]
pub struct AtomicSwapCycle {
    /// Flash loan amount (in base units)
    pub flash_loan_amount: u64,
    /// Flash loan token
    pub loan_token: Pubkey,
    /// First swap (borrow → token B)
    pub swap_1: SwapStep,
    /// Second swap (token B → borrow token)
    pub swap_2: SwapStep,
    /// Expected profit after fees (in base units)
    pub expected_profit: u64,
    /// Flash loan fee (in base units)
    pub flash_loan_fee: u64,
}

impl AtomicSwapCycle {
    pub fn new(
        flash_loan_amount: u64,
        loan_token: Pubkey,
        swap_1: SwapStep,
        swap_2: SwapStep,
        flash_loan_fee: u64,
        expected_profit: u64,
    ) -> Self {
        Self {
            flash_loan_amount,
            loan_token,
            swap_1,
            swap_2,
            expected_profit,
            flash_loan_fee,
        }
    }

    /// Validate the complete cycle
    pub fn validate(&self) -> Result<(), SwapError> {
        // Validate individual swaps
        self.swap_1.validate()?;
        self.swap_2.validate()?;

        // Verify swap tokens align
        // Swap 1: loan_token → intermediate
        if self.swap_1.input_mint != self.loan_token {
            return Err(SwapError::SwapChainBroken(
                "Swap 1 input must match loan token".to_string(),
            ));
        }

        // Swap 2: intermediate → loan_token
        if self.swap_2.output_mint != self.loan_token {
            return Err(SwapError::SwapChainBroken(
                "Swap 2 output must match loan token".to_string(),
            ));
        }

        // Verify swap chain is connected
        if self.swap_1.output_mint != self.swap_2.input_mint {
            return Err(SwapError::SwapChainBroken(
                "Swap 1 output must match Swap 2 input".to_string(),
            ));
        }

        // Validate profit is realistic
        if self.expected_profit == 0 {
            return Err(SwapError::NoProfitExpected);
        }

        // Check that profit > fee
        if self.expected_profit < self.flash_loan_fee {
            return Err(SwapError::ProfitBelowFee {
                profit: self.expected_profit,
                fee: self.flash_loan_fee,
            });
        }

        Ok(())
    }

    /// Calculate net profit after all fees
    pub fn net_profit(&self) -> u64 {
        self.expected_profit.saturating_sub(self.flash_loan_fee)
    }

    /// Check if opportunity meets minimum profit threshold
    pub fn meets_minimum_profit(&self, min_profit_lamports: u64) -> bool {
        self.net_profit() >= min_profit_lamports
    }

    /// Get slippage percentage for swap 1
    pub fn swap_1_slippage_bps(&self) -> u64 {
        if self.swap_1.input_amount == 0 {
            return 0;
        }

        let slippage = self.swap_1.input_amount
            .saturating_sub(self.swap_1.min_output_amount);
        
        // Calculate basis points: (slippage / input) * 10000
        ((slippage as u128 * 10000) / (self.swap_1.input_amount as u128)) as u64
    }

    /// Get slippage percentage for swap 2
    pub fn swap_2_slippage_bps(&self) -> u64 {
        if self.swap_2.input_amount == 0 {
            return 0;
        }

        let slippage = self.swap_2.input_amount
            .saturating_sub(self.swap_2.min_output_amount);
        
        // Calculate basis points: (slippage / input) * 10000
        ((slippage as u128 * 10000) / (self.swap_2.input_amount as u128)) as u64
    }

    /// Maximum acceptable slippage (50 bps = 0.5%)
    const MAX_SLIPPAGE_BPS: u64 = 50;

    /// Check if slippage is within acceptable range
    pub fn validate_slippage(&self) -> Result<(), SwapError> {
        let swap_1_slip = self.swap_1_slippage_bps();
        let swap_2_slip = self.swap_2_slippage_bps();

        if swap_1_slip > Self::MAX_SLIPPAGE_BPS {
            return Err(SwapError::ExcessiveSlippage {
                swap: 1,
                actual_bps: swap_1_slip,
                max_bps: Self::MAX_SLIPPAGE_BPS,
            });
        }

        if swap_2_slip > Self::MAX_SLIPPAGE_BPS {
            return Err(SwapError::ExcessiveSlippage {
                swap: 2,
                actual_bps: swap_2_slip,
                max_bps: Self::MAX_SLIPPAGE_BPS,
            });
        }

        Ok(())
    }

    /// OPTIMIZED: Fast liquidity check (inline, <5ms)
    /// Min liquidity reduced from 100k to 30k
    /// Enables 3.3x more opportunities
    #[inline(always)]
    pub fn is_liquidity_sufficient(&self, amount: u64) -> bool {
        // Min: 30,000 lamports (down from 100,000)
        // Max: 10,000,000 lamports
        amount >= 30_000 && amount <= 10_000_000
    }

    /// OPTIMIZED: Combined validation in single pass (<10ms)
    /// For hot path - profit + liquidity + slippage check
    #[inline(always)]
    pub fn validate_opportunity_fast(
        &self,
        profit: u64,
        liquidity: u64,
        slippage: u64,
    ) -> bool {
        profit >= 1_000 &&           // Min profit
        liquidity >= 30_000 &&       // Min liquidity (reduced 70%!)
        liquidity <= 10_000_000 &&   // Max liquidity
        slippage <= 50               // Max slippage (50 bps)
    }

    /// OPTIMIZED: Pre-execution checks (<15ms total)
    /// Validates both swaps without full simulation
    #[inline(always)]
    pub fn pre_flight_check(&self) -> bool {
        // Quick checks only:
        // - Both swaps exist
        // - Amounts are positive
        // - Minimum safety checks
        self.swap_1.input_amount > 0 &&
        self.swap_1.min_output_amount > 0 &&
        self.swap_2.input_amount > 0 &&
        self.swap_2.min_output_amount > 0
    }
}

/// Swap execution error
#[derive(Debug, Clone)]
pub enum SwapError {
    /// Input and output tokens are the same
    SameInputOutput,
    /// Zero swap amount
    ZeroAmount,
    /// Zero minimum output (no slippage protection)
    ZeroMinOutput,
    /// Swap chain is broken (tokens don't connect)
    SwapChainBroken(String),
    /// No profit expected
    NoProfitExpected,
    /// Profit below fee minimum
    ProfitBelowFee { profit: u64, fee: u64 },
    /// Slippage exceeds maximum
    ExcessiveSlippage {
        swap: u8,
        actual_bps: u64,
        max_bps: u64,
    },
    /// Insufficient balance
    InsufficientBalance { required: u64, available: u64 },
    /// Transaction simulation failed
    SimulationFailed(String),
    /// Invalid transaction
    InvalidTransaction(String),
}

impl fmt::Display for SwapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SwapError::SameInputOutput => write!(f, "Input and output tokens must be different"),
            SwapError::ZeroAmount => write!(f, "Swap amount cannot be zero"),
            SwapError::ZeroMinOutput => write!(f, "Minimum output cannot be zero"),
            SwapError::SwapChainBroken(msg) => write!(f, "Swap chain broken: {}", msg),
            SwapError::NoProfitExpected => write!(f, "No profit expected from arbitrage"),
            SwapError::ProfitBelowFee { profit, fee } => {
                write!(f, "Profit ({}) below fee ({})", profit, fee)
            }
            SwapError::ExcessiveSlippage {
                swap,
                actual_bps,
                max_bps,
            } => {
                write!(
                    f,
                    "Swap {} slippage too high: {} bps (max: {} bps)",
                    swap, actual_bps, max_bps
                )
            }
            SwapError::InsufficientBalance {
                required,
                available,
            } => {
                write!(
                    f,
                    "Insufficient balance: need {} but have {}",
                    required, available
                )
            }
            SwapError::SimulationFailed(msg) => write!(f, "Simulation failed: {}", msg),
            SwapError::InvalidTransaction(msg) => write!(f, "Invalid transaction: {}", msg),
        }
    }
}

impl std::error::Error for SwapError {}

/// Swap Manager - orchestrates atomic swaps
pub struct AtomicSwapManager {
    /// Minimum profit threshold (lamports)
    pub min_profit_lamports: u64,
    /// Maximum slippage allowed (basis points)
    pub max_slippage_bps: u64,
}

impl AtomicSwapManager {
    /// Create new swap manager
    pub fn new(min_profit_lamports: u64, max_slippage_bps: u64) -> Self {
        Self {
            min_profit_lamports,
            max_slippage_bps,
        }
    }

    /// Default configuration
    pub fn default() -> Self {
        Self {
            min_profit_lamports: 5000, // $0.0015 at current rates
            max_slippage_bps: 50,      // 0.5%
        }
    }

    /// Validate opportunity before execution
    pub fn validate_opportunity(
        &self,
        cycle: &AtomicSwapCycle,
    ) -> Result<(), SwapError> {
        // Validate structure
        cycle.validate()?;

        // Validate slippage
        cycle.validate_slippage()?;

        // Validate profit
        if !cycle.meets_minimum_profit(self.min_profit_lamports) {
            return Err(SwapError::NoProfitExpected);
        }

        info!(
            "✅ Opportunity validated: profit={} lamports, slippage=ok",
            cycle.net_profit()
        );

        Ok(())
    }

    /// Estimate output amount with slippage
    pub fn estimate_output_with_slippage(
        &self,
        input_amount: u64,
        expected_output: u64,
        slippage_bps: u64,
    ) -> u64 {
        // Calculate slippage amount
        let slippage_amount = ((input_amount as u128 * slippage_bps as u128) / 10000) as u64;
        
        // Subtract from expected output
        expected_output.saturating_sub(slippage_amount)
    }

    /// Calculate expected output from input and price ratio
    pub fn calculate_output(
        &self,
        input_amount: u64,
        input_price: u64,
        output_price: u64,
    ) -> Result<u64, SwapError> {
        if output_price == 0 {
            return Err(SwapError::InvalidTransaction(
                "Output price cannot be zero".to_string(),
            ));
        }

        // output = input * (input_price / output_price)
        let output = ((input_amount as u128 * input_price as u128) / output_price as u128) as u64;
        
        Ok(output)
    }

    /// Check if arbitrage spread is profitable
    pub fn is_spread_profitable(
        &self,
        buy_price: u64,   // Lowest buy price (DEX 1)
        sell_price: u64,  // Highest sell price (DEX 2)
        fee_bps: u64,     // Flash loan fee in bps
    ) -> bool {
        if buy_price == 0 || sell_price == 0 {
            return false;
        }

        // Calculate profit as percentage: (sell - buy) / sell * 10000
        let spread_bps = if sell_price > buy_price {
            ((sell_price as u128 - buy_price as u128) * 10000 / sell_price as u128) as u64
        } else {
            0
        };

        // Profitable if spread > fee
        spread_bps > fee_bps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_cycle() -> AtomicSwapCycle {
        let loan_token = Pubkey::new_unique();
        let intermediate_token = Pubkey::new_unique();

        let swap_1 = SwapStep::new(
            SwapProtocol::Raydium,
            loan_token,
            intermediate_token,
            1_000_000,     // 1 unit
            990_000,       // 99% output (1% slippage)
            Pubkey::new_unique(),
        );

        let swap_2 = SwapStep::new(
            SwapProtocol::Orca,
            intermediate_token,
            loan_token,
            990_000,
            985_000,       // 99.5% output
            Pubkey::new_unique(),
        );

        AtomicSwapCycle::new(
            1_000_000,
            loan_token,
            swap_1,
            swap_2,
            2750,          // 0.275% fee (Orca)
            10_000,        // 10k lamports profit
        )
    }

    #[test]
    fn test_swap_step_validation() {
        let mint_a = Pubkey::new_unique();
        let mint_b = Pubkey::new_unique();

        // Valid swap
        let swap = SwapStep::new(
            SwapProtocol::Raydium,
            mint_a,
            mint_b,
            1000,
            900,
            Pubkey::new_unique(),
        );
        assert!(swap.validate().is_ok());

        // Same input and output
        let swap_same = SwapStep::new(
            SwapProtocol::Raydium,
            mint_a,
            mint_a,
            1000,
            900,
            Pubkey::new_unique(),
        );
        assert!(matches!(swap_same.validate(), Err(SwapError::SameInputOutput)));

        // Zero amount
        let swap_zero = SwapStep::new(
            SwapProtocol::Raydium,
            mint_a,
            mint_b,
            0,
            900,
            Pubkey::new_unique(),
        );
        assert!(matches!(swap_zero.validate(), Err(SwapError::ZeroAmount)));
    }

    #[test]
    fn test_atomic_cycle_validation() {
        let cycle = create_test_cycle();
        assert!(cycle.validate().is_ok());
    }

    #[test]
    fn test_swap_chain_validation() {
        let loan_token = Pubkey::new_unique();
        let token_b = Pubkey::new_unique();
        let token_c = Pubkey::new_unique();

        // Correct chain: A → B → A
        let swap_1 = SwapStep::new(
            SwapProtocol::Raydium,
            loan_token,
            token_b,
            1000,
            900,
            Pubkey::new_unique(),
        );

        let swap_2 = SwapStep::new(
            SwapProtocol::Orca,
            token_b,
            loan_token,
            900,
            850,
            Pubkey::new_unique(),
        );

        let cycle = AtomicSwapCycle::new(
            1000,
            loan_token,
            swap_1,
            swap_2,
            10,
            50,
        );
        assert!(cycle.validate().is_ok());

        // Broken chain: A → B → C (doesn't return to A)
        let swap_broken = SwapStep::new(
            SwapProtocol::Orca,
            token_b,
            token_c,
            900,
            850,
            Pubkey::new_unique(),
        );

        let cycle_broken = AtomicSwapCycle::new(
            1000,
            loan_token,
            swap_1.clone(),
            swap_broken,
            10,
            50,
        );
        assert!(cycle_broken.validate().is_err());
    }

    #[test]
    fn test_profit_validation() {
        let cycle = create_test_cycle();
        assert!(cycle.meets_minimum_profit(5000));
        assert!(!cycle.meets_minimum_profit(15000)); // Profit too low
    }

    #[test]
    fn test_net_profit_calculation() {
        let cycle = create_test_cycle();
        let net = cycle.net_profit();
        assert_eq!(net, 10_000 - 2750); // profit - fee
    }

    #[test]
    fn test_slippage_calculation() {
        let cycle = create_test_cycle();
        
        let slip_1 = cycle.swap_1_slippage_bps();
        assert!(slip_1 > 0);

        // Both should be within 50 bps for valid cycle
        assert!(cycle.validate_slippage().is_ok());
    }

    #[test]
    fn test_profit_below_fee_error() {
        let loan_token = Pubkey::new_unique();
        let intermediate_token = Pubkey::new_unique();

        let swap_1 = SwapStep::new(
            SwapProtocol::Raydium,
            loan_token,
            intermediate_token,
            1_000_000,
            990_000,
            Pubkey::new_unique(),
        );

        let swap_2 = SwapStep::new(
            SwapProtocol::Orca,
            intermediate_token,
            loan_token,
            990_000,
            985_000,
            Pubkey::new_unique(),
        );

        // Profit (1000) < Fee (5000)
        let cycle = AtomicSwapCycle::new(
            1_000_000,
            loan_token,
            swap_1,
            swap_2,
            5000,
            1000,
        );

        assert!(matches!(
            cycle.validate(),
            Err(SwapError::ProfitBelowFee { .. })
        ));
    }

    #[test]
    fn test_manager_validation() {
        let manager = AtomicSwapManager::default();
        let cycle = create_test_cycle();
        
        assert!(manager.validate_opportunity(&cycle).is_ok());
    }

    #[test]
    fn test_slippage_estimation() {
        let manager = AtomicSwapManager::default();
        
        let output = manager.estimate_output_with_slippage(
            1_000_000,
            1_000_000,
            50, // 50 bps = 0.5%
        );
        
        // Should lose about 5000 to slippage
        assert!(output < 1_000_000);
        assert!(output > 990_000);
    }

    #[test]
    fn test_spread_profitability() {
        let manager = AtomicSwapManager::default();
        
        // Spread of 100 bps (1%), fee 25 bps → profitable
        assert!(manager.is_spread_profitable(10000, 10100, 25));
        
        // Spread of 10 bps, fee 25 bps → not profitable
        assert!(!manager.is_spread_profitable(10000, 10010, 25));
    }

    #[test]
    fn test_output_calculation() {
        let manager = AtomicSwapManager::default();
        
        // 1000 units at price 5 converted to output price 10
        // = 1000 * (5 / 10) = 500
        let output = manager.calculate_output(1000, 5, 10).unwrap();
        assert_eq!(output, 500);
    }
}
