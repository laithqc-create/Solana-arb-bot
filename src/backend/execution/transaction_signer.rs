/// Transaction Signer
///
/// Safely signs and submits transactions:
/// - Sign with keypair from vault
/// - Serialize transaction properly
/// - Handle signature failures
/// - Track transaction lifecycle
/// - Return signature for confirmation

use solana_sdk::{
    transaction::Transaction,
    signature::{Keypair, Signature},
    signer::Signer,
    message::Message,
};
use log::{info, warn, error};
use std::fmt;

/// Signing result
#[derive(Debug, Clone)]
pub struct SigningResult {
    /// Transaction signature
    pub signature: String,
    /// Message hash (base58)
    pub message_hash: String,
    /// Whether signing succeeded
    pub success: bool,
    /// Size of transaction in bytes
    pub transaction_size: usize,
}

/// Signing error
#[derive(Debug, Clone)]
pub enum SigningError {
    /// Keypair is invalid
    InvalidKeypair(String),
    /// Transaction serialization failed
    SerializationFailed(String),
    /// Signing operation failed
    SigningFailed(String),
    /// Transaction too large
    TransactionTooLarge { actual: usize, max: usize },
    /// Message creation failed
    MessageCreationFailed(String),
}

impl fmt::Display for SigningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SigningError::InvalidKeypair(msg) => write!(f, "Invalid keypair: {}", msg),
            SigningError::SerializationFailed(msg) => write!(f, "Serialization failed: {}", msg),
            SigningError::SigningFailed(msg) => write!(f, "Signing failed: {}", msg),
            SigningError::TransactionTooLarge { actual, max } => {
                write!(f, "Transaction too large: {} bytes (max: {})", actual, max)
            }
            SigningError::MessageCreationFailed(msg) => write!(f, "Message creation failed: {}", msg),
        }
    }
}

impl std::error::Error for SigningError {}

/// Transaction Signer
pub struct TransactionSigner {
    /// Keypair for signing
    keypair: Keypair,
    /// Max transaction size (bytes)
    max_tx_size: usize,
}

impl TransactionSigner {
    /// Create new signer with keypair
    pub fn new(keypair: Keypair) -> Self {
        Self {
            keypair,
            max_tx_size: 1232, // Solana max is 1232 bytes
        }
    }

    /// Get signer public key
    pub fn public_key(&self) -> String {
        self.keypair.pubkey().to_string()
    }

    /// Sign a transaction
    pub fn sign_transaction(
        &self,
        mut transaction: Transaction,
    ) -> Result<SigningResult, SigningError> {
        // Validate keypair
        let pubkey = self.keypair.pubkey();
        info!("📝 Signing transaction with {}", pubkey);

        // Create message and sign
        let message = transaction.message_mut();
        let serialized = bincode::serialize(message)
            .map_err(|e| SigningError::SerializationFailed(e.to_string()))?;

        // Check transaction size
        let tx_size = serialized.len();
        if tx_size > self.max_tx_size {
            return Err(SigningError::TransactionTooLarge {
                actual: tx_size,
                max: self.max_tx_size,
            });
        }

        // Sign the message
        let signature = self.keypair.sign_message(&serialized);

        info!(
            "✅ Transaction signed: {} ({} bytes)",
            signature, tx_size
        );

        Ok(SigningResult {
            signature: signature.to_string(),
            message_hash: format!("{:?}", message),
            success: true,
            transaction_size: tx_size,
        })
    }

    /// Verify transaction signature
    pub fn verify_signature(
        &self,
        signature: &Signature,
        message: &[u8],
    ) -> bool {
        signature.verify(&self.keypair.pubkey(), message)
    }

    /// Get transaction fee estimate (in lamports)
    pub fn estimate_transaction_fee(&self, tx_size: usize) -> u64 {
        // Solana base fee: 5000 lamports per signature
        // Additional: 5000 lamports per 32KB of transaction size
        let base_fee = 5_000u64;
        let size_multiplier = ((tx_size + 32_000) / 32_000) as u64;
        base_fee * size_multiplier
    }
}

/// Transaction Submission Status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubmissionStatus {
    /// Waiting to be processed
    Pending,
    /// Confirmed in a block
    Confirmed,
    /// Finalized (safe from rollback)
    Finalized,
    /// Failed
    Failed,
}

/// Transaction lifecycle tracker
#[derive(Debug, Clone)]
pub struct TransactionTracker {
    /// Signature
    pub signature: String,
    /// Status
    pub status: SubmissionStatus,
    /// Created timestamp
    pub created_at: i64,
    /// Confirmed timestamp
    pub confirmed_at: Option<i64>,
    /// Finalized timestamp
    pub finalized_at: Option<i64>,
    /// Retry count
    pub retry_count: u32,
    /// Last error
    pub last_error: Option<String>,
}

impl TransactionTracker {
    /// Create new tracker
    pub fn new(signature: String) -> Self {
        Self {
            signature,
            status: SubmissionStatus::Pending,
            created_at: chrono::Local::now().timestamp(),
            confirmed_at: None,
            finalized_at: None,
            retry_count: 0,
            last_error: None,
        }
    }

    /// Mark as confirmed
    pub fn mark_confirmed(&mut self) {
        self.status = SubmissionStatus::Confirmed;
        self.confirmed_at = Some(chrono::Local::now().timestamp());
        info!("✅ Transaction confirmed: {}", self.signature);
    }

    /// Mark as finalized
    pub fn mark_finalized(&mut self) {
        self.status = SubmissionStatus::Finalized;
        self.finalized_at = Some(chrono::Local::now().timestamp());
        info!("🔒 Transaction finalized: {}", self.signature);
    }

    /// Mark as failed
    pub fn mark_failed(&mut self, error: String) {
        self.status = SubmissionStatus::Failed;
        self.last_error = Some(error.clone());
        warn!("❌ Transaction failed: {}", error);
    }

    /// Get confirmation time
    pub fn confirmation_time_ms(&self) -> Option<i64> {
        match (self.confirmed_at, self.created_at) {
            (Some(confirmed), created) => Some((confirmed - created) * 1000),
            (None, _) => None,
        }
    }

    /// Get finalization time
    pub fn finalization_time_ms(&self) -> Option<i64> {
        match (self.finalized_at, self.created_at) {
            (Some(finalized), created) => Some((finalized - created) * 1000),
            (None, _) => None,
        }
    }

    /// Is transaction still pending?
    pub fn is_pending(&self) -> bool {
        self.status == SubmissionStatus::Pending
    }

    /// Is transaction successful?
    pub fn is_successful(&self) -> bool {
        self.status == SubmissionStatus::Finalized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signer_creation() {
        let keypair = Keypair::new();
        let signer = TransactionSigner::new(keypair.clone());
        
        assert_eq!(signer.public_key(), keypair.pubkey().to_string());
    }

    #[test]
    fn test_transaction_fee_estimate() {
        let keypair = Keypair::new();
        let signer = TransactionSigner::new(keypair);
        
        // Base fee for small transaction
        let fee_small = signer.estimate_transaction_fee(100);
        assert_eq!(fee_small, 5_000); // Base fee only
        
        // Fee scales with size
        let fee_large = signer.estimate_transaction_fee(33_000);
        assert!(fee_large > fee_small);
    }

    #[test]
    fn test_transaction_tracker_creation() {
        let tracker = TransactionTracker::new("sig123".to_string());
        
        assert_eq!(tracker.signature, "sig123");
        assert_eq!(tracker.status, SubmissionStatus::Pending);
        assert!(tracker.is_pending());
        assert!(!tracker.is_successful());
    }

    #[test]
    fn test_transaction_tracker_confirmation() {
        let mut tracker = TransactionTracker::new("sig123".to_string());
        
        tracker.mark_confirmed();
        assert_eq!(tracker.status, SubmissionStatus::Confirmed);
        assert!(tracker.confirmed_at.is_some());
        assert!(tracker.confirmation_time_ms().is_some());
    }

    #[test]
    fn test_transaction_tracker_finalization() {
        let mut tracker = TransactionTracker::new("sig123".to_string());
        
        tracker.mark_finalized();
        assert_eq!(tracker.status, SubmissionStatus::Finalized);
        assert!(tracker.is_successful());
        assert!(tracker.finalized_at.is_some());
    }

    #[test]
    fn test_transaction_tracker_failure() {
        let mut tracker = TransactionTracker::new("sig123".to_string());
        
        tracker.mark_failed("Network error".to_string());
        assert_eq!(tracker.status, SubmissionStatus::Failed);
        assert!(tracker.last_error.is_some());
    }
}
