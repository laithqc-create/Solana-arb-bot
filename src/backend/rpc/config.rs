/// RPC Configuration Manager
/// 
/// Handles loading RPC configuration from:
/// - Environment variables
/// - Config files
/// - Defaults
/// 
/// Supports both development and production setups

use super::{RpcClientManager, RpcEndpoint};
use std::path::PathBuf;
use log::{info, warn};

/// RPC Configuration
#[derive(Debug, Clone)]
pub struct RpcConfig {
    /// Helius API key (get from https://www.helius.dev)
    pub helius_api_key: String,
    /// Use testnet instead of mainnet
    pub is_testnet: bool,
    /// Custom Helius endpoint (optional override)
    pub custom_helius_url: Option<String>,
    /// Use devnet for testing
    pub is_devnet: bool,
}

impl RpcConfig {
    /// Load configuration from environment variables
    /// 
    /// Variables:
    /// - `HELIUS_API_KEY` - Required for mainnet/testnet
    /// - `SOLANA_NETWORK` - "mainnet" (default), "testnet", or "devnet"
    /// - `CUSTOM_HELIUS_URL` - Override Helius endpoint
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let helius_api_key = std::env::var("HELIUS_API_KEY")
            .map_err(|_| "HELIUS_API_KEY environment variable not set")?;

        let network = std::env::var("SOLANA_NETWORK").unwrap_or_else(|_| "mainnet".to_string());
        let custom_url = std::env::var("CUSTOM_HELIUS_URL").ok();

        let (is_testnet, is_devnet) = match network.to_lowercase().as_str() {
            "testnet" => (true, false),
            "devnet" => (false, true),
            "mainnet" => (false, false),
            _ => {
                warn!("⚠️ Unknown network: {}. Using mainnet", network);
                (false, false)
            }
        };

        info!(
            "📡 Loaded RPC config: network={}, has_api_key={}",
            if is_devnet {
                "devnet"
            } else if is_testnet {
                "testnet"
            } else {
                "mainnet"
            },
            !helius_api_key.is_empty()
        );

        Ok(Self {
            helius_api_key,
            is_testnet,
            custom_helius_url: custom_url,
            is_devnet,
        })
    }

    /// Load from config file
    /// 
    /// Expected JSON format:
    /// ```json
    /// {
    ///   "helius_api_key": "your-key",
    ///   "network": "mainnet",
    ///   "custom_helius_url": null
    /// }
    /// ```
    pub fn from_file(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let config_json = std::fs::read_to_string(path)?;
        let config: serde_json::Value = serde_json::from_str(&config_json)?;

        let helius_api_key = config["helius_api_key"]
            .as_str()
            .ok_or("helius_api_key not found in config")?
            .to_string();

        let network = config["network"]
            .as_str()
            .unwrap_or("mainnet")
            .to_lowercase();

        let custom_url = config["custom_helius_url"]
            .as_str()
            .map(|s| s.to_string());

        let (is_testnet, is_devnet) = match network.as_str() {
            "testnet" => (true, false),
            "devnet" => (false, true),
            _ => (false, false),
        };

        info!("📄 Loaded RPC config from: {}", path.display());

        Ok(Self {
            helius_api_key,
            is_testnet,
            custom_helius_url: custom_url,
            is_devnet,
        })
    }

    /// Load with fallback chain: env → file → defaults
    pub fn load_with_fallback(config_file: Option<&PathBuf>) -> Result<Self, Box<dyn std::error::Error>> {
        // Try environment first
        if let Ok(config) = Self::from_env() {
            if !config.helius_api_key.is_empty() {
                return Ok(config);
            }
        }

        // Try config file if provided
        if let Some(file_path) = config_file {
            if let Ok(config) = Self::from_file(file_path) {
                return Ok(config);
            }
        }

        // Fallback to defaults (devnet)
        warn!("⚠️ No RPC config found, using devnet defaults");
        Ok(Self {
            helius_api_key: String::new(), // Empty for devnet
            is_testnet: false,
            custom_helius_url: None,
            is_devnet: true,
        })
    }

    /// Create RPC client manager from this config
    pub fn create_manager(&self) -> Result<RpcClientManager, Box<dyn std::error::Error>> {
        if self.is_devnet {
            // Devnet: use standard public endpoints
            let primary = RpcEndpoint::new("Solana Devnet", "https://api.devnet.solana.com")
                .with_priority(0);
            let fallbacks = vec![
                RpcEndpoint::new("Solana Devnet Backup", "https://api.devnet.solana.com")
                    .with_priority(1),
            ];

            info!("🔧 Using DEVNET endpoints (for testing)");
            RpcClientManager::new(primary, fallbacks)
        } else if self.helius_api_key.is_empty() {
            return Err("Helius API key required for mainnet/testnet".into());
        } else {
            // Mainnet/Testnet: use Helius
            let rpc_url = if let Some(custom_url) = &self.custom_helius_url {
                custom_url.clone()
            } else if self.is_testnet {
                format!(
                    "https://testnet.helius-rpc.com/?api-key={}",
                    self.helius_api_key
                )
            } else {
                format!(
                    "https://mainnet.helius-rpc.com/?api-key={}",
                    self.helius_api_key
                )
            };

            let network = if self.is_testnet { "TESTNET" } else { "MAINNET" };
            info!("📡 Using Helius RPC for {}", network);

            // Create manager from custom URL
            let primary = RpcEndpoint::new("Helius", &rpc_url).with_priority(0);
            let fallbacks = vec![
                RpcEndpoint::new(
                    "Solana Standard",
                    if self.is_testnet {
                        "https://api.testnet.solana.com"
                    } else {
                        "https://api.mainnet-beta.solana.com"
                    },
                )
                .with_priority(1),
            ];

            RpcClientManager::new(primary, fallbacks)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rpc_config_devnet() {
        let config = RpcConfig {
            helius_api_key: String::new(),
            is_testnet: false,
            custom_helius_url: None,
            is_devnet: true,
        };

        assert!(config.is_devnet);
        assert!(!config.is_testnet);
    }

    #[test]
    fn test_rpc_config_mainnet() {
        let config = RpcConfig {
            helius_api_key: "test-key-123".to_string(),
            is_testnet: false,
            custom_helius_url: None,
            is_devnet: false,
        };

        assert!(!config.is_devnet);
        assert!(!config.is_testnet);
        assert_eq!(config.helius_api_key, "test-key-123");
    }

    #[test]
    fn test_rpc_config_testnet() {
        let config = RpcConfig {
            helius_api_key: "test-key-456".to_string(),
            is_testnet: true,
            custom_helius_url: None,
            is_devnet: false,
        };

        assert!(!config.is_devnet);
        assert!(config.is_testnet);
    }

    #[test]
    fn test_custom_helius_url() {
        let config = RpcConfig {
            helius_api_key: String::new(),
            is_testnet: false,
            custom_helius_url: Some("https://custom.rpc.com".to_string()),
            is_devnet: false,
        };

        assert_eq!(
            config.custom_helius_url,
            Some("https://custom.rpc.com".to_string())
        );
    }
}
