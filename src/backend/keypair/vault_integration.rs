/// Vault Keypair Integration
/// 
/// Encrypts keypairs for storage in the vault
/// Supports:
/// - Encrypt keypair before saving to disk
/// - Decrypt encrypted keypair for use
/// - Store encrypted keypair in vault config
/// - Retrieve encrypted keypair from vault config

use super::ManagedKeypair;
use crate::backend::vault::SecureVault;
use log::{info, warn};
use solana_sdk::signature::Keypair;
use std::path::PathBuf;

/// Vault wrapper for keypair operations
pub struct VaultKeypairManager {
    vault: std::sync::Arc<SecureVault>,
}

impl VaultKeypairManager {
    /// Create new vault keypair manager
    pub fn new(vault: std::sync::Arc<SecureVault>) -> Self {
        Self { vault }
    }

    /// Encrypt a keypair for storage in vault
    /// 
    /// Converts keypair to bytes, encrypts with AES-256-GCM,
    /// returns hex-encoded string suitable for vault storage
    pub fn encrypt_keypair(
        &self,
        keypair: &Keypair,
        password: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Convert keypair to bytes
        let keypair_bytes = keypair.to_bytes();
        let keypair_json = serde_json::to_string(&keypair_bytes.to_vec())?;

        // Derive encryption key from password
        let encryption_key = SecureVault::derive_key_from_password(password);

        // Encrypt
        let encrypted = SecureVault::encrypt_data(&keypair_json, &encryption_key)?;

        info!("✅ Keypair encrypted for vault storage");
        Ok(encrypted)
    }

    /// Decrypt a keypair from vault storage
    /// 
    /// Takes hex-encoded encrypted keypair, decrypts with password,
    /// converts back to Keypair object
    pub fn decrypt_keypair(
        &self,
        encrypted_keypair: &str,
        password: &str,
    ) -> Result<ManagedKeypair, Box<dyn std::error::Error>> {
        // Derive encryption key from password
        let encryption_key = SecureVault::derive_key_from_password(password);

        // Decrypt
        let decrypted_json = SecureVault::decrypt_data(encrypted_keypair, &encryption_key)?;

        // Parse back to bytes
        let keypair_bytes: Vec<u8> = serde_json::from_str(&decrypted_json)?;

        if keypair_bytes.len() != 64 {
            return Err(format!(
                "Invalid keypair size after decryption: {} bytes (expected 64)",
                keypair_bytes.len()
            )
            .into());
        }

        // Convert to Keypair
        let keypair = Keypair::from_bytes(&keypair_bytes)?;

        info!("✅ Keypair decrypted from vault");
        info!("📝 Public key: {}", keypair.pubkey());

        Ok(ManagedKeypair {
            keypair,
            source_path: PathBuf::from("[vault]"),
        })
    }

    /// Save encrypted keypair to vault config
    /// 
    /// # Arguments
    /// * `keypair` - Keypair to encrypt and save
    /// * `password` - Password for encryption
    pub async fn save_encrypted_keypair(
        &self,
        keypair: &Keypair,
        password: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Encrypt the keypair
        let encrypted = self.encrypt_keypair(keypair, password)?;

        // Load current config
        let mut config = self.vault.load_config().await?;

        // Update with encrypted keypair
        config.private_key_encrypted = Some(encrypted);

        // Save back to vault
        self.vault.save_config(&config).await?;

        info!("✅ Encrypted keypair saved to vault");
        Ok(())
    }

    /// Load encrypted keypair from vault config
    /// 
    /// # Arguments
    /// * `password` - Password for decryption
    pub async fn load_encrypted_keypair(
        &self,
        password: &str,
    ) -> Result<ManagedKeypair, Box<dyn std::error::Error>> {
        // Load config from vault
        let config = self.vault.load_config().await?;

        // Check if keypair exists in vault
        let encrypted_keypair = config
            .private_key_encrypted
            .ok_or("No encrypted keypair found in vault")?;

        // Decrypt and return
        self.decrypt_keypair(&encrypted_keypair, password)
    }

    /// Check if keypair is stored in vault
    pub async fn has_encrypted_keypair(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let config = self.vault.load_config().await?;
        Ok(config.private_key_encrypted.is_some())
    }

    /// Remove encrypted keypair from vault (careful!)
    pub async fn delete_encrypted_keypair(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut config = self.vault.load_config().await?;
        config.private_key_encrypted = None;
        self.vault.save_config(&config).await?;

        warn!("⚠️ Encrypted keypair deleted from vault");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_encrypt_decrypt_cycle() {
        // This test would need a vault instance
        // Skipping for now as vault requires filesystem setup
        // Would test: encrypt -> decrypt -> verify same keypair
    }

    #[test]
    fn test_keypair_to_bytes_conversion() {
        let keypair = Keypair::new();
        let bytes = keypair.to_bytes();

        assert_eq!(bytes.len(), 64); // Ed25519 keypair is 64 bytes
        assert_eq!(bytes[32..], keypair.pubkey().to_bytes()); // Second half is pubkey
    }
}
