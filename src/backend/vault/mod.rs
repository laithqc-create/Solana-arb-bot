// src/backend/vault/mod.rs
use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm, Nonce,
};
use argon2::{Argon2, PasswordHasher};
use rand::Rng;
use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use log::info;

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultConfig {
    pub geyser_rpc_url: String,
    pub backup_rpc_url: String,
    pub jito_region: String,
    pub private_key_encrypted: Option<String>,  // Encrypted, not plaintext
}

impl Default for VaultConfig {
    fn default() -> Self {
        VaultConfig {
            geyser_rpc_url: "wss://mainnet.helius-rpc.com/ws".to_string(),
            backup_rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            jito_region: "us-west".to_string(),
            private_key_encrypted: None,
        }
    }
}

pub struct SecureVault {
    vault_path: PathBuf,
    encryption_key: Option<[u8; 32]>,
}

impl SecureVault {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let vault_dir = PathBuf::from("src/infra/vault");
        
        // Create vault directory if it doesn't exist
        if !vault_dir.exists() {
            fs::create_dir_all(&vault_dir)?;
            info!("✅ Created vault directory: {}", vault_dir.display());
        }
        
        Ok(SecureVault {
            vault_path: vault_dir,
            encryption_key: None,
        })
    }
    
    /// Load or initialize config
    pub async fn load_config(&self) -> Result<VaultConfig, Box<dyn std::error::Error>> {
        let config_path = self.vault_path.join("config.json");
        
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let config: VaultConfig = serde_json::from_str(&content)?;
            info!("✅ Loaded vault config");
            Ok(config)
        } else {
            let config = VaultConfig::default();
            self.save_config(&config).await?;
            info!("✅ Created new vault config with defaults");
            Ok(config)
        }
    }
    
    /// Save config to disk
    pub async fn save_config(&self, config: &VaultConfig) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = self.vault_path.join("config.json");
        let content = serde_json::to_string_pretty(&config)?;
        fs::write(&config_path, content)?;
        info!("✅ Saved vault config");
        Ok(())
    }
    
    /// Derive encryption key from password using Argon2
    pub fn derive_key_from_password(password: &str) -> [u8; 32] {
        use argon2::password_hash::SaltString;
        
        // Generate deterministic salt (in real app, store & retrieve)
        let salt = SaltString::encode_b64(b"solana-arb-engine").expect("Failed to create salt");
        let argon2 = Argon2::default();
        
        // Hash password
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .expect("Failed to hash password");
        
        // Extract first 32 bytes as key
        let hash_str = password_hash.hash.expect("No hash").to_string();
        let mut key = [0u8; 32];
        for (i, byte) in hash_str.as_bytes().iter().enumerate().take(32) {
            key[i] = *byte;
        }
        key
    }
    
    /// Encrypt data using AES-256-GCM
    pub fn encrypt_data(data: &str, key: &[u8; 32]) -> Result<String, Box<dyn std::error::Error>> {
        let cipher = Aes256Gcm::new_from_slice(key)?;
        let mut rng = rand::thread_rng();
        let nonce_bytes: [u8; 12] = rng.gen();
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let ciphertext = cipher.encrypt(nonce, Payload::from(data.as_bytes()))
            .map_err(|e| format!("Encryption error: {:?}", e))?;
        
        // Combine nonce + ciphertext and hex encode
        let mut result = nonce_bytes.to_vec();
        result.extend_from_slice(&ciphertext);
        
        Ok(hex::encode(result))
    }
    
    /// Decrypt data using AES-256-GCM
    pub fn decrypt_data(encrypted: &str, key: &[u8; 32]) -> Result<String, Box<dyn std::error::Error>> {
        let decoded = hex::decode(encrypted)?;
        if decoded.len() < 12 {
            return Err("Invalid encrypted data".into());
        }
        
        let (nonce_bytes, ciphertext) = decoded.split_at(12);
        let nonce = Nonce::from_slice(nonce_bytes);
        
        let cipher = Aes256Gcm::new_from_slice(key)?;
        let plaintext = cipher.decrypt(nonce, Payload::from(ciphertext))
            .map_err(|e| format!("Decryption error: {:?}", e))?;
        
        Ok(String::from_utf8(plaintext)?)
    }
}
