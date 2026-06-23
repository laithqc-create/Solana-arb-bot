/// RPC Client Manager for Solana Arbitrage Engine
/// 
/// Handles connections to multiple RPC providers with intelligent failover:
/// - Primary: Helius RPC (ultra-low latency, Geyser integration)
/// - Fallback: Standard Solana RPC endpoints
/// - Retry: Automatic reconnection with exponential backoff
///
/// Supports:
/// - Geyser WebSocket connections (real-time updates)
/// - JSON-RPC HTTP connections (transaction submission)
/// - Connection health monitoring
/// - Automatic provider switching on failure
/// - Configurable retry policies

use solana_client::rpc_client::RpcClient;
use std::time::Duration;
use log::{info};
use std::sync::Arc;
use tokio::sync::RwLock;

/// RPC endpoint configuration
#[derive(Debug, Clone)]
pub struct RpcEndpoint {
    /// Endpoint name for logging
    pub name: String,
    /// HTTP endpoint URL
    pub http_url: String,
    /// WebSocket endpoint URL (optional)
    pub ws_url: Option<String>,
    /// Priority (lower = preferred)
    pub priority: u8,
    /// Max retries before switching
    pub max_retries: u32,
}

impl RpcEndpoint {
    pub fn new(name: &str, http_url: &str) -> Self {
        Self {
            name: name.to_string(),
            http_url: http_url.to_string(),
            ws_url: None,
            priority: 100,
            max_retries: 3,
        }
    }

    pub fn with_ws(mut self, ws_url: &str) -> Self {
        self.ws_url = Some(ws_url.to_string());
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }
}

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Initial delay before first retry (ms)
    pub initial_delay_ms: u64,
    /// Maximum delay between retries (ms)
    pub max_delay_ms: u64,
    /// Multiplier for exponential backoff
    pub backoff_multiplier: f64,
    /// Max total retries
    pub max_attempts: u32,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0,
            max_attempts: 5,
        }
    }
}

/// RPC Client wrapper with failover support
pub struct RpcClientManager {
    /// Primary RPC endpoint (Helius)
    primary_endpoint: RpcEndpoint,
    /// Fallback RPC endpoints
    fallback_endpoints: Vec<RpcEndpoint>,
    /// Current RPC client
    current_client: Arc<RwLock<RpcClient>>,
    /// Current endpoint being used
    current_endpoint: Arc<RwLock<usize>>, // Index into endpoints list
    /// Retry configuration
    retry_config: RetryConfig,
    /// All endpoints (primary + fallbacks)
    all_endpoints: Vec<RpcEndpoint>,
}

impl RpcClientManager {
    /// Create new RPC manager with Helius as primary
    /// 
    /// # Arguments
    /// * `helius_api_key` - Helius API key (get free at helius.dev)
    /// 
    /// # Example
    /// ```ignore
    /// let manager = RpcClientManager::new_with_helius("your-api-key")?;
    /// ```
    pub fn new_with_helius(helius_api_key: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let helius_http = format!(
            "https://mainnet.helius-rpc.com/?api-key={}",
            helius_api_key
        );
        let helius_ws = format!(
            "wss://mainnet.helius-rpc.com/?api-key={}",
            helius_api_key
        );

        let primary = RpcEndpoint::new("Helius", &helius_http)
            .with_ws(&helius_ws)
            .with_priority(0);

        let fallbacks = vec![
            RpcEndpoint::new("Solana Mainnet", "https://api.mainnet-beta.solana.com")
                .with_priority(1),
            RpcEndpoint::new("Solana Backup", "https://api.rpcpool.com")
                .with_priority(2),
        ];

        Self::new(primary, fallbacks)
    }

    /// Create new RPC manager with custom endpoints
    pub fn new(
        primary: RpcEndpoint,
        fallbacks: Vec<RpcEndpoint>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Combine all endpoints
        let mut all_endpoints = vec![primary.clone()];
        all_endpoints.extend(fallbacks.clone());

        // Sort by priority (ascending)
        all_endpoints.sort_by_key(|ep| ep.priority);

        // Create initial RPC client with primary
        let rpc_client = RpcClient::new(primary.http_url.clone());

        info!(
            "📡 Initialized RPC manager with {} endpoints",
            all_endpoints.len()
        );
        for (i, ep) in all_endpoints.iter().enumerate() {
            info!("   [{}] {} (priority: {})", i, ep.name, ep.priority);
        }

        Ok(Self {
            primary_endpoint: primary,
            fallback_endpoints: fallbacks,
            current_client: Arc::new(RwLock::new(rpc_client)),
            current_endpoint: Arc::new(RwLock::new(0)),
            retry_config: RetryConfig::default(),
            all_endpoints,
        })
    }

    /// Get current RPC client
    pub async fn get_client(&self) -> Arc<RwLock<RpcClient>> {
        Arc::clone(&self.current_client)
    }

    /// Get current endpoint name
    pub async fn get_current_endpoint_name(&self) -> String {
        let idx = *self.current_endpoint.read().await;
        self.all_endpoints
            .get(idx)
            .map(|ep| ep.name.clone())
            .unwrap_or_else(|| "Unknown".to_string())
    }

    /// Check RPC connection health
    /// 
    /// Performs a simple getVersion RPC call to verify connection
    pub async fn check_health(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let client = self.current_client.read().await;
        match client.get_version() {
            Ok(version) => {
                info!(
                    "✅ RPC health check passed: {}",
                    version.solana_core.to_string()
                );
                Ok(true)
            }
            Err(e) => {
                warn!("⚠️ RPC health check failed: {}", e);
                Err(format!("Health check failed: {}", e).into())
            }
        }
    }

    /// Switch to next available RPC endpoint
    /// 
    /// Tries endpoints in priority order until one succeeds
    pub async fn switch_endpoint(&self) -> Result<String, Box<dyn std::error::Error>> {
        let current_idx = *self.current_endpoint.read().await;

        for (i, endpoint) in self.all_endpoints.iter().enumerate().skip(current_idx + 1) {
            let client = RpcClient::new(endpoint.http_url.clone());
            
            // Try to verify the connection
            if client.get_version().is_ok() {
                info!("🔄 Switched to RPC: {} ({})", endpoint.name, endpoint.http_url);

                let mut current = self.current_endpoint.write().await;
                *current = i;

                let mut rpc = self.current_client.write().await;
                *rpc = client;

                return Ok(endpoint.name.clone());
            }
        }

        Err("All RPC endpoints failed".into())
    }

    /// Execute operation with automatic retry and failover
    /// 
    /// Retries the operation with exponential backoff
    /// Automatically switches endpoints if all retries fail
    pub async fn execute_with_retry<F, T>(
        &self,
        mut op: F,
    ) -> Result<T, Box<dyn std::error::Error>>
    where
        F: FnMut(&RpcClient) -> Result<T, Box<dyn std::error::Error>>,
    {
        let mut attempt = 0;

        loop {
            // Try current endpoint
            let client = self.current_client.read().await;
            match op(&client) {
                Ok(result) => {
                    info!("✅ Operation succeeded on attempt {}", attempt + 1);
                    return Ok(result);
                }
                Err(e) => {
                    attempt += 1;

                    if attempt >= self.retry_config.max_attempts {
                        // Try to switch endpoint before giving up
                        drop(client); // Release lock before switching
                        warn!(
                            "⚠️ Max retries ({}) exceeded, attempting endpoint switch",
                            self.retry_config.max_attempts
                        );

                        match self.switch_endpoint().await {
                            Ok(new_endpoint) => {
                                info!(
                                    "🔄 Switched to {} after {} failed attempts",
                                    new_endpoint, attempt
                                );
                                attempt = 0; // Reset attempts for new endpoint
                            }
                            Err(_) => {
                                return Err(format!(
                                    "Operation failed after {} attempts and endpoint switch: {}",
                                    attempt, e
                                )
                                .into());
                            }
                        }
                    } else {
                        drop(client);

                        // Calculate exponential backoff delay
                        let delay_ms = std::cmp::min(
                            self.retry_config.initial_delay_ms * 
                                (self.retry_config.backoff_multiplier.powi(attempt as i32 - 1) as u64),
                            self.retry_config.max_delay_ms,
                        );

                        warn!(
                            "⚠️ Attempt {} failed: {}. Retrying in {}ms",
                            attempt, e, delay_ms
                        );

                        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    }
                }
            }
        }
    }

    /// Get list of all available endpoints
    pub fn get_endpoints(&self) -> Vec<(String, String)> {
        self.all_endpoints
            .iter()
            .map(|ep| (ep.name.clone(), ep.http_url.clone()))
            .collect()
    }

    /// Update Helius API key
    /// 
    /// Useful for rotating keys or updating configuration
    pub async fn update_helius_key(
        &mut self,
        new_api_key: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let helius_http = format!(
            "https://mainnet.helius-rpc.com/?api-key={}",
            new_api_key
        );
        let helius_ws = format!(
            "wss://mainnet.helius-rpc.com/?api-key={}",
            new_api_key
        );

        let new_primary = RpcEndpoint::new("Helius", &helius_http)
            .with_ws(&helius_ws)
            .with_priority(0);

        let new_client = RpcClient::new(helius_http);

        let primary = &mut self.primary_endpoint;
        primary.http_url = new_primary.http_url;
        primary.ws_url = new_primary.ws_url;

        let mut client = self.current_client.write().await;
        *client = new_client;

        info!("🔑 Updated Helius API key");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rpc_endpoint_builder() {
        let endpoint = RpcEndpoint::new("Test", "https://api.example.com")
            .with_ws("wss://ws.example.com")
            .with_priority(1)
            .with_max_retries(5);

        assert_eq!(endpoint.name, "Test");
        assert_eq!(endpoint.http_url, "https://api.example.com");
        assert_eq!(endpoint.ws_url, Some("wss://ws.example.com".to_string()));
        assert_eq!(endpoint.priority, 1);
        assert_eq!(endpoint.max_retries, 5);
    }

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.initial_delay_ms, 100);
        assert_eq!(config.max_delay_ms, 5000);
        assert_eq!(config.backoff_multiplier, 2.0);
        assert_eq!(config.max_attempts, 5);
    }

    #[test]
    fn test_endpoint_sorting() {
        let primary = RpcEndpoint::new("Primary", "http://1").with_priority(0);
        let fallback1 = RpcEndpoint::new("Fallback1", "http://2").with_priority(2);
        let fallback2 = RpcEndpoint::new("Fallback2", "http://3").with_priority(1);

        let manager = RpcClientManager::new(primary, vec![fallback1, fallback2])
            .expect("Failed to create manager");

        // Check endpoints are sorted by priority
        assert_eq!(manager.all_endpoints[0].priority, 0);
        assert_eq!(manager.all_endpoints[1].priority, 1);
        assert_eq!(manager.all_endpoints[2].priority, 2);
    }

    #[test]
    fn test_exponential_backoff_calculation() {
        let config = RetryConfig::default();

        // Attempt 1: 100ms
        let delay1 = std::cmp::min(
            config.initial_delay_ms * (config.backoff_multiplier.powi(0) as u64),
            config.max_delay_ms,
        );
        assert_eq!(delay1, 100);

        // Attempt 2: 200ms
        let delay2 = std::cmp::min(
            config.initial_delay_ms * (config.backoff_multiplier.powi(1) as u64),
            config.max_delay_ms,
        );
        assert_eq!(delay2, 200);

        // Attempt 3: 400ms
        let delay3 = std::cmp::min(
            config.initial_delay_ms * (config.backoff_multiplier.powi(2) as u64),
            config.max_delay_ms,
        );
        assert_eq!(delay3, 400);
    }
}
