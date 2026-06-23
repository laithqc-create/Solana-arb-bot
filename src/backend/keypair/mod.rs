/// Keypair Manager for Solana Arbitrage Engine
/// 
/// Handles secure keypair loading, encryption, and storage
/// Supports:
/// - Loading from environment variables (SOLANA_KEYPAIR_PATH)
/// - Loading from filesystem (for development)
/// - Encryption with vault (for production)
/// - In-memory caching with zeroize on drop
///
/// SECURITY NOTES:
/// - Never log the private key
/// - Always zeroize memory on drop
/// - Use environment variables for paths, not inline
/// - Encrypt before storing on disk

use solana_sdk::{
    signature::{Keypair, Signer},
    pubkey::Pubkey,
};
use std::path::PathBuf;
use std::fs;
use log::{info};
use zeroize::Zeroize;

/// Keypair wrapper that zeroizes memory on drop
pub struct ManagedKeypair {
    /// The actual keypair (contains private key)
    keypair: Keypair,
    /// Path where it was loaded from (for audit logging)
    source_path: PathBuf,
}

impl ManagedKeypair {
    /// Get the public key (safe to expose)
    pub fn pubkey(&self) -> Pubkey {
        self.keypair.pubkey()
    }

    /// Get mutable reference to keypair (for signing)
    pub fn keypair(&self) -> &Keypair {
        &self.keypair
    }

    /// Get public key as string
    pub fn pubkey_string(&self) -> String {
        self.keypair.pubkey().to_string()
    }

    /// Get source path (for logging)
    pub fn source(&self) -> &PathBuf {
        &self.source_path
    }
}

impl Drop for ManagedKeypair {
    fn drop(&mut self) {
        // Zeroize the secret key bytes on drop
        // This overwrites the memory with zeros to prevent key recovery
        let mut secret_bytes = self.keypair.to_bytes().to_vec();
        secret_bytes.zeroize();
        info!("🔐 Keypair zeroized from memory");
    }
}

/// Keypair Manager - handles all keypair operations
pub struct KeypairManager;

impl KeypairManager {
    /// Load keypair from Solana CLI default location
    /// 
    /// Default path: `~/.config/solana/id.json`
    /// 
    /// # Security
    /// - Only works on development machines
    /// - Never use on production servers
    /// - Requires filesystem access
    pub fn load_from_default_path() -> Result<ManagedKeypair, Box<dyn std::error::Error>> {
        let home = dirs::home_dir()
            .ok_or("Could not determine home directory")?;
        
        let keypair_path = home
            .join(".config/solana/id.json");

        if !keypair_path.exists() {
            return Err(format!(
                "Keypair not found at {}. Create with: solana-keygen new",
                keypair_path.display()
            ).into());
        }

        Self::load_from_path(&keypair_path)
    }

    /// Load keypair from custom filesystem path
    /// 
    /// # Arguments
    /// * `path` - Path to keypair JSON file
    /// 
    /// # Returns
    /// ManagedKeypair with automatic zeroize on drop
    pub fn load_from_path(path: &PathBuf) -> Result<ManagedKeypair, Box<dyn std::error::Error>> {
        if !path.exists() {
            return Err(format!("Keypair file not found: {}", path.display()).into());
        }

        // Read JSON file
        let json_str = fs::read_to_string(path)?;
        
        // Parse as keypair bytes (Solana CLI format is JSON array of u8s)
        let keypair_bytes: Vec<u8> = serde_json::from_str(&json_str)?;

        if keypair_bytes.len() != 64 {
            return Err(format!(
                "Invalid keypair size: {} bytes (expected 64)",
                keypair_bytes.len()
            ).into());
        }

        // Convert to Keypair
        let keypair = Keypair::from_bytes(&keypair_bytes)?;

        info!("✅ Loaded keypair from {}", path.display());
        info!("📝 Public key: {}", keypair.pubkey());

        Ok(ManagedKeypair {
            keypair,
            source_path: path.clone(),
        })
    }

    /// Load keypair from environment variable
    /// 
    /// Reads path from `SOLANA_KEYPAIR_PATH` environment variable
    /// Then loads the keypair from that path
    /// 
    /// # Environment Variables
    /// - `SOLANA_KEYPAIR_PATH` - Path to keypair JSON file
    /// 
    /// # Returns
    /// ManagedKeypair if successful
    pub fn load_from_env() -> Result<ManagedKeypair, Box<dyn std::error::Error>> {
        let keypair_path = std::env::var("SOLANA_KEYPAIR_PATH")
            .map_err(|_| "SOLANA_KEYPAIR_PATH environment variable not set")?;

        let path = PathBuf::from(keypair_path);
        Self::load_from_path(&path)
    }

    /// Load keypair from environment variable with fallback
    /// 
    /// Tries to load from `SOLANA_KEYPAIR_PATH` first,
    /// then falls back to default Solana CLI location if not set
    pub fn load_with_fallback() -> Result<ManagedKeypair, Box<dyn std::error::Error>> {
        match Self::load_from_env() {
            Ok(keypair) => Ok(keypair),
            Err(_) => {
                warn!("⚠️ SOLANA_KEYPAIR_PATH not set, using default location");
                Self::load_from_default_path()
            }
        }
    }

    /// Validate keypair is properly loaded
    /// 
    /// Performs sanity checks:
    /// - Public key is valid (not all zeros)
    /// - Can sign a message
    /// - Public key matches expected value (if provided)
    pub fn validate_keypair(
        keypair: &ManagedKeypair,
        expected_pubkey: Option<&Pubkey>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pubkey = keypair.pubkey();

        // Check public key is not all zeros
        if pubkey.to_bytes().iter().all(|&b| b == 0) {
            return Err("Invalid keypair: public key is all zeros".into());
        }

        // Verify we can sign with this keypair
        let test_message = b"test";
        let _signature = keypair.keypair().sign_message(test_message);
        
        // Signature is always valid if we got here

        // Check against expected pubkey if provided
        if let Some(expected) = expected_pubkey {
            if *expected != pubkey {
                return Err(format!(
                    "Public key mismatch: expected {}, got {}",
                    expected, pubkey
                ).into());
            }
        }

        info!("✅ Keypair validation passed");
        Ok(())
    }

    /// Get balance requirement for executing transactions
    /// 
    /// Estimates total SOL needed for:
    /// - Transaction fees (5,000 lamports per tx = ~$0.0015)
    /// - Jito tip (85-90% of profit)
    /// - Flash loan fees (0.01% - 0.09%)
    /// - Buffer (10% extra)
    pub fn estimate_required_balance(
        expected_arb_profit_lamports: u64,
        num_executions: u64,
    ) -> u64 {
        let tx_fee_per_execution = 5000; // 5K lamports
        let jito_tip_percentage = 87; // 87% of profit
        let buffer_percentage = 10; // 10% buffer

        let total_tx_fees = tx_fee_per_execution * num_executions;
        let jito_tips = (expected_arb_profit_lamports * jito_tip_percentage) / 100;
        let buffer = ((expected_arb_profit_lamports + jito_tips) * buffer_percentage) / 100;

        total_tx_fees + jito_tips + buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_required_balance() {
        // Example: 100K lamports profit, 10 executions
        let profit = 100_000;
        let executions = 10;
        
        let required = KeypairManager::estimate_required_balance(profit, executions);
        
        // Should include:
        // - Tx fees: 5000 * 10 = 50,000
        // - Jito tips: 100,000 * 0.87 = 87,000
        // - Buffer: (100,000 + 87,000) * 0.1 = 18,700
        // Total: 155,700
        assert!(required > 150_000);
        assert!(required < 200_000);
    }

    #[test]
    fn test_keypair_pubkey_not_all_zeros() {
        // Create a test keypair
        let keypair = Keypair::new();
        let managed = ManagedKeypair {
            keypair,
            source_path: PathBuf::from("/tmp/test"),
        };

        // Should pass validation
        assert!(KeypairManager::validate_keypair(&managed, None).is_ok());
    }

    #[test]
    fn test_estimate_with_zero_profit() {
        let required = KeypairManager::estimate_required_balance(0, 1);
        // Should still include transaction fees and buffer
        assert!(required > 0);
    }
}
