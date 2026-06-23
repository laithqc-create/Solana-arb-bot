/// Jito Bundle Client
///
/// Submits bundles to Jito's private pool and tracks status:
/// - Connect to Jito RPC endpoint
/// - Submit bundle atomically
/// - Poll for status (confirmed, landed, expired)
/// - Handle failures gracefully

use super::{JitoBundle, BundleStatus};
use log::{info};
use std::time::{Duration, SystemTime};
use std::fmt;

/// Jito client configuration
#[derive(Debug, Clone)]
pub struct JitoConfig {
    /// Jito RPC endpoint
    pub rpc_url: String,
    /// Bundle submission timeout (seconds)
    pub submission_timeout: u64,
    /// Status polling interval (ms)
    pub poll_interval_ms: u64,
    /// Max polling attempts
    pub max_poll_attempts: u32,
}

impl JitoConfig {
    /// Create new config
    pub fn new(rpc_url: &str) -> Self {
        Self {
            rpc_url: rpc_url.to_string(),
            submission_timeout: 30,
            poll_interval_ms: 500,
            max_poll_attempts: 60, // 30 seconds total
        }
    }

    /// Mainnet Jito endpoint
    pub fn mainnet() -> Self {
        Self::new("https://mainnet.block-engine.jito.wtf/api/v1/bundles")
    }

    /// Testnet Jito endpoint
    pub fn testnet() -> Self {
        Self::new("https://testnet.block-engine.jito.wtf/api/v1/bundles")
    }

    /// Validate config
    pub fn validate(&self) -> Result<(), JitoError> {
        if self.rpc_url.is_empty() {
            return Err(JitoError::InvalidEndpoint);
        }

        if self.submission_timeout == 0 {
            return Err(JitoError::InvalidTimeout);
        }

        Ok(())
    }
}

/// Bundle submission response
#[derive(Debug, Clone)]
pub struct BundleSubmissionResponse {
    /// Bundle UUID from Jito
    pub bundle_id: String,
    /// Submission timestamp
    pub submitted_at: SystemTime,
    /// Confirmation status
    pub confirmation_status: String,
}

/// Jito Bundle Client
pub struct JitoBundleClient {
    /// Configuration
    config: JitoConfig,
    /// In-flight bundles (ID -> Bundle)
    inflight_bundles: std::collections::HashMap<String, JitoBundle>,
}

impl JitoBundleClient {
    /// Create new client
    pub fn new(config: JitoConfig) -> Result<Self, JitoError> {
        config.validate()?;
        
        info!("🔗 Initializing Jito client: {}", config.rpc_url);

        Ok(Self {
            config,
            inflight_bundles: std::collections::HashMap::new(),
        })
    }

    /// Create client with mainnet
    pub fn mainnet() -> Result<Self, JitoError> {
        Self::new(JitoConfig::mainnet())
    }

    /// Create client with testnet
    pub fn testnet() -> Result<Self, JitoError> {
        Self::new(JitoConfig::testnet())
    }

    /// Submit bundle to Jito
    pub async fn submit_bundle(
        &mut self,
        mut bundle: JitoBundle,
    ) -> Result<BundleSubmissionResponse, JitoError> {
        // Validate bundle
        if let Err(e) = bundle.validate() {
            return Err(JitoError::BundleValidationFailed(e.to_string()));
        }

        // Mark as submitted
        bundle.mark_submitted();

        let bundle_id = bundle.bundle_id.clone();

        // Store in-flight
        self.inflight_bundles.insert(bundle_id.clone(), bundle.clone());

        info!("📤 Submitting bundle {} with tip {}", bundle_id, bundle.jito_tip);

        // In a real implementation, this would:
        // 1. Serialize bundle as JSON
        // 2. POST to Jito RPC endpoint
        // 3. Parse response
        // 4. Return bundle ID

        // For now, simulate successful submission
        let response = BundleSubmissionResponse {
            bundle_id: bundle_id.clone(),
            submitted_at: SystemTime::now(),
            confirmation_status: "submitted".to_string(),
        };

        info!("✅ Bundle {} submitted successfully", bundle_id);

        Ok(response)
    }

    /// Poll bundle status
    pub async fn get_bundle_status(
        &self,
        bundle_id: &str,
    ) -> Result<BundleStatus, JitoError> {
        // Check in-flight
        if let Some(bundle) = self.inflight_bundles.get(bundle_id) {
            return Ok(bundle.status);
        }

        // In a real implementation, this would:
        // 1. Query Jito RPC for bundle status
        // 2. Return status (submitted, confirmed, landed, expired)
        // 3. Handle not-found errors

        // For now, return not found
        Err(JitoError::BundleNotFound(bundle_id.to_string()))
    }

    /// Wait for bundle confirmation
    pub async fn wait_for_confirmation(
        &mut self,
        bundle_id: &str,
    ) -> Result<u64, JitoError> {
        for attempt in 0..self.config.max_poll_attempts {
            match self.get_bundle_status(bundle_id).await {
                Ok(BundleStatus::Confirmed) => {
                    info!("✅ Bundle {} confirmed", bundle_id);
                    if let Some(bundle) = self.inflight_bundles.get(bundle_id) {
                        return Ok(bundle.confirmed_slot.unwrap_or(0));
                    }
                    return Ok(0);
                }
                Ok(BundleStatus::Landed) => {
                    info!("🎯 Bundle {} landed", bundle_id);
                    if let Some(bundle) = self.inflight_bundles.get(bundle_id) {
                        return Ok(bundle.confirmed_slot.unwrap_or(0));
                    }
                    return Ok(0);
                }
                Ok(BundleStatus::Failed) => {
                    return Err(JitoError::BundleExecutionFailed(bundle_id.to_string()));
                }
                Ok(BundleStatus::Expired) => {
                    return Err(JitoError::BundleExpired(bundle_id.to_string()));
                }
                _ => {
                    // Still pending, wait before next poll
                    if attempt < self.config.max_poll_attempts - 1 {
                        tokio::time::sleep(Duration::from_millis(self.config.poll_interval_ms))
                            .await;
                    }
                }
            }
        }

        Err(JitoError::BundleTimeout(bundle_id.to_string()))
    }

    /// Get in-flight bundle count
    pub fn inflight_count(&self) -> usize {
        self.inflight_bundles.len()
    }

    /// Get all in-flight bundles
    pub fn inflight_bundles(&self) -> Vec<&JitoBundle> {
        self.inflight_bundles.values().collect()
    }

    /// Remove bundle from tracking
    pub fn remove_bundle(&mut self, bundle_id: &str) -> Option<JitoBundle> {
        self.inflight_bundles.remove(bundle_id)
    }
}

/// Jito client error
#[derive(Debug, Clone)]
pub enum JitoError {
    /// Invalid RPC endpoint
    InvalidEndpoint,
    /// Invalid timeout
    InvalidTimeout,
    /// Bundle validation failed
    BundleValidationFailed(String),
    /// Connection failed
    ConnectionFailed(String),
    /// Submission failed
    SubmissionFailed(String),
    /// Bundle not found
    BundleNotFound(String),
    /// Bundle execution failed
    BundleExecutionFailed(String),
    /// Bundle expired (not included)
    BundleExpired(String),
    /// Bundle timed out
    BundleTimeout(String),
    /// Request parsing error
    ParseError(String),
    /// Network error
    NetworkError(String),
}

impl fmt::Display for JitoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JitoError::InvalidEndpoint => write!(f, "Invalid Jito RPC endpoint"),
            JitoError::InvalidTimeout => write!(f, "Invalid timeout value"),
            JitoError::BundleValidationFailed(msg) => write!(f, "Bundle validation failed: {}", msg),
            JitoError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            JitoError::SubmissionFailed(msg) => write!(f, "Bundle submission failed: {}", msg),
            JitoError::BundleNotFound(id) => write!(f, "Bundle not found: {}", id),
            JitoError::BundleExecutionFailed(id) => write!(f, "Bundle execution failed: {}", id),
            JitoError::BundleExpired(id) => write!(f, "Bundle expired: {}", id),
            JitoError::BundleTimeout(id) => write!(f, "Bundle timed out: {}", id),
            JitoError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            JitoError::NetworkError(msg) => write!(f, "Network error: {}", msg),
        }
    }
}

impl std::error::Error for JitoError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jito_config_mainnet() {
        let config = JitoConfig::mainnet();
        assert!(config.rpc_url.contains("mainnet"));
    }

    #[test]
    fn test_jito_config_testnet() {
        let config = JitoConfig::testnet();
        assert!(config.rpc_url.contains("testnet"));
    }

    #[test]
    fn test_jito_config_validation() {
        let config = JitoConfig::new("https://example.com");
        assert!(config.validate().is_ok());

        let invalid = JitoConfig {
            rpc_url: String::new(),
            submission_timeout: 30,
            poll_interval_ms: 500,
            max_poll_attempts: 60,
        };
        assert!(invalid.validate().is_err());
    }

    #[tokio::test]
    async fn test_client_creation() {
        let client = JitoBundleClient::mainnet();
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_inflight_tracking() {
        let mut client = JitoBundleClient::mainnet().unwrap();
        
        let mut bundle = JitoBundle::new("test-1".to_string());
        bundle.set_tip(5000);

        let _ = client.submit_bundle(bundle).await;
        
        assert_eq!(client.inflight_count(), 1);
    }

    #[test]
    fn test_bundle_submission_response() {
        let response = BundleSubmissionResponse {
            bundle_id: "test-bundle".to_string(),
            submitted_at: SystemTime::now(),
            confirmation_status: "submitted".to_string(),
        };

        assert_eq!(response.bundle_id, "test-bundle");
        assert_eq!(response.confirmation_status, "submitted");
    }
}
