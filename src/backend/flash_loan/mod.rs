/// Flash Loan Manager for Solana DEX arbitrage
/// 
/// Handles flash loan borrowing from Orca protocol
/// Supports atomic swaps within a single transaction
///
/// Current Implementation: Orca Flash Loans (0.0275% fee)
/// Future: Raydium, Solend, Marinade

use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Signature,
    transaction::Transaction,
};
use spl_token::state::Account as TokenAccount;
use std::str::FromStr;

pub mod fee_calculator;

pub use fee_calculator::FlashLoanFeeCalculator;

/// Configuration for flash loan execution
#[derive(Debug, Clone)]
pub struct FlashLoanConfig {
    /// Token to borrow (mint address)
    pub token_mint: Pubkey,
    /// Amount to borrow in smallest unit (lamports for SOL, actual for tokens)
    pub borrow_amount: u64,
    /// Which protocol to use (currently: "orca", future: "raydium", "solend")
    pub protocol: String,
    /// User's wallet address (receiver of loan)
    pub user_wallet: Pubkey,
    /// Maximum acceptable slippage in basis points (e.g., 50 = 0.5%)
    pub max_slippage_bps: u64,
}

/// Flash loan manager - coordinates flash loan borrowing and repayment
pub struct FlashLoanManager {
    /// RPC client for blockchain interactions
    rpc_client: RpcClient,
    /// User's public key (payer)
    payer: Pubkey,
    /// Fee calculator for different protocols
    fee_calculator: FlashLoanFeeCalculator,
}

impl FlashLoanManager {
    /// Create new flash loan manager
    pub fn new(rpc_url: &str, payer: Pubkey) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            rpc_client: RpcClient::new(rpc_url.to_string()),
            payer,
            fee_calculator: FlashLoanFeeCalculator::new(),
        })
    }

    /// Get flash loan fee for a given protocol and amount
    ///
    /// # Arguments
    /// * `protocol` - Flash loan protocol ("orca", "raydium", "solend", "marinade")
    /// * `amount` - Loan amount in smallest unit
    ///
    /// # Returns
    /// Fee amount in smallest unit
    pub fn get_flash_loan_fee(&self, protocol: &str, amount: u64) -> Result<u64, String> {
        self.fee_calculator.calculate_fee(protocol, amount)
    }

    /// Calculate minimum return amount after flash loan repayment
    ///
    /// For arbitrage to be profitable:
    /// final_amount > borrowed_amount + fee + gas_cost
    ///
    /// # Arguments
    /// * `borrowed_amount` - Amount borrowed
    /// * `protocol` - Flash loan protocol
    /// * `estimated_gas_cost` - Estimated gas cost in lamports
    ///
    /// # Returns
    /// Minimum return needed to break even
    pub fn calculate_minimum_return(
        &self,
        borrowed_amount: u64,
        protocol: &str,
        estimated_gas_cost: u64,
    ) -> Result<u64, String> {
        let fee = self.get_flash_loan_fee(protocol, borrowed_amount)?;
        Ok(borrowed_amount + fee + estimated_gas_cost)
    }

    /// Build flash loan instruction for Orca protocol
    ///
    /// Orca Flash Loan Program: 9W957QEUQMax4GSLCxDLXpTK63gbLosLvmWXNrWgAg7
    ///
    /// # Arguments
    /// * `pool_address` - Address of Orca pool
    /// * `token_mint` - Token to borrow
    /// * `amount` - Amount to borrow
    /// * `instruction_data` - Encoded instruction data for flash loan callback
    ///
    /// # Returns
    /// Instruction ready for transaction
    pub fn build_orca_flash_loan_instruction(
        &self,
        pool_address: &str,
        token_mint: &str,
        _amount: u64,
        instruction_data: Vec<u8>,
    ) -> Result<Instruction, Box<dyn std::error::Error>> {
        let pool = Pubkey::from_str(pool_address)?;
        let token = Pubkey::from_str(token_mint)?;

        // Orca Flash Loan Program ID
        let flash_loan_program = Pubkey::from_str(
            "9W957QEUQMax4GSLCxDLXpTK63gbLosLvmWXNrWgAg7",
        )?;

        // Build account metadata for flash loan
        // Order matters! Orca expects: pool, mint, receiver, authority, etc.
        let accounts = vec![
            AccountMeta::new(pool, false),                    // Pool account (mutable)
            AccountMeta::new_readonly(token, false),          // Token mint (read-only)
            AccountMeta::new(self.payer, false),              // Receiver = payer (mutable)
            AccountMeta::new_readonly(flash_loan_program, false), // Program (read-only)
        ];

        Ok(Instruction {
            program_id: flash_loan_program,
            accounts,
            data: instruction_data,
        })
    }

    /// Build flash loan instruction for Raydium protocol (future implementation)
    pub fn build_raydium_flash_loan_instruction(
        &self,
        _pool_address: &str,
        _token_mint: &str,
        _amount: u64,
        _instruction_data: Vec<u8>,
    ) -> Result<Instruction, Box<dyn std::error::Error>> {
        Err("Raydium flash loans not yet implemented".into())
    }

    /// Simulate flash loan transaction before submission
    ///
    /// Critical safety check - never submit without simulation!
    ///
    /// # Arguments
    /// * `tx` - Transaction to simulate
    ///
    /// # Returns
    /// Ok(true) if simulation successful, Err if simulation failed
    pub async fn simulate_flash_loan_transaction(
        &self,
        tx: &Transaction,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let sim_result = self.rpc_client.simulate_transaction(tx)?;

        // Check for errors
        if let Some(err) = &sim_result.value.err {
            return Err(format!("Simulation failed: {:?}", err).into());
        }

        // Check logs for errors or panics
        if let Some(logs) = &sim_result.value.logs {
            for log in logs {
                if log.contains("Error") || log.contains("Panic") {
                    return Err(format!("Simulation error in logs: {}", log).into());
                }
            }
        }

        // Check compute units used
        if let Some(units) = extract_compute_units(&sim_result.value.logs) {
            if units > 1_400_000 {
                // Solana limit is 1.4M for standard transactions
                return Err(format!("Compute units exceeded: {} > 1,400,000", units).into());
            }
        }

        Ok(true)
    }

    /// Submit transaction to blockchain
    ///
    /// Only call after successful simulation!
    ///
    /// # Arguments
    /// * `tx` - Signed transaction ready for submission
    ///
    /// # Returns
    /// Transaction signature
    pub async fn submit_transaction(
        &self,
        tx: &Transaction,
    ) -> Result<Signature, Box<dyn std::error::Error>> {
        let signature = self.rpc_client.send_transaction(tx)?;
        Ok(signature)
    }

    /// Check if wallet has sufficient balance for flash loan execution
    ///
    /// # Arguments
    /// * `required_for_gas_and_fees` - Total SOL needed for gas + jito tip + buffer
    ///
    /// # Returns
    /// Ok(balance) if sufficient, Err if insufficient
    pub async fn check_wallet_balance(
        &self,
        required_for_gas_and_fees: u64,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let balance = self.rpc_client.get_balance(&self.payer)?;

        if balance < required_for_gas_and_fees {
            return Err(format!(
                "Insufficient balance: {} < {} required",
                balance, required_for_gas_and_fees
            )
            .into());
        }

        Ok(balance)
    }

    /// Get token account information for validation
    pub async fn get_token_account_balance(
        &self,
        token_account: &Pubkey,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        let account = self.rpc_client.get_account(token_account)?;
        
        // Parse as TokenAccount
        let token_account_data = TokenAccount::unpack(&account.data)?;
        Ok(token_account_data.amount)
    }
}

/// Extract compute units from simulation logs
/// Format: "Program consumed X units"
fn extract_compute_units(logs: &Option<Vec<String>>) -> Option<u64> {
    if let Some(log_vec) = logs {
        for log in log_vec {
            if log.contains("consumed") {
                // Parse "Program consumed X units"
                if let Some(start) = log.find("consumed ") {
                    let after_consumed = &log[start + 9..];
                    if let Some(end) = after_consumed.find(" ") {
                        if let Ok(units) = after_consumed[..end].parse::<u64>() {
                            return Some(units);
                        }
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flash_loan_manager_creation() {
        let payer = Pubkey::new_unique();
        let manager = FlashLoanManager::new("https://api.mainnet-beta.solana.com", payer);
        assert!(manager.is_ok());
    }

    #[test]
    fn test_get_flash_loan_fee_orca() {
        let payer = Pubkey::new_unique();
        let manager = FlashLoanManager::new("https://api.mainnet-beta.solana.com", payer)
            .expect("Failed to create manager");

        // Test Orca fee: 0.0275% = 275 basis points
        let amount = 1_000_000; // 1M lamports
        let fee = manager
            .get_flash_loan_fee("orca", amount)
            .expect("Failed to get fee");

        // 0.0275% of 1M = 275 lamports
        assert_eq!(fee, 275);
    }

    #[test]
    fn test_calculate_minimum_return() {
        let payer = Pubkey::new_unique();
        let manager = FlashLoanManager::new("https://api.mainnet-beta.solana.com", payer)
            .expect("Failed to create manager");

        let borrowed = 1_000_000;
        let gas_cost = 10_000;
        let min_return = manager
            .calculate_minimum_return(borrowed, "orca", gas_cost)
            .expect("Failed to calculate");

        // Should be: 1,000,000 + 275 (fee) + 10,000 (gas) = 1,010,275
        assert_eq!(min_return, 1_010_275);
    }

    #[test]
    fn test_extract_compute_units() {
        let logs = Some(vec![
            "Program BPFLoader1111111111111111111111111111111111 invoke [1]".to_string(),
            "Program 9W957QEUQMax4GSLCxDLXpTK63gbLosLvmWXNrWgAg7 consumed 123456 units".to_string(),
            "Program 9W957QEUQMax4GSLCxDLXpTK63gbLosLvmWXNrWgAg7 success".to_string(),
        ]);

        let units = extract_compute_units(&logs);
        assert_eq!(units, Some(123456));
    }
}
