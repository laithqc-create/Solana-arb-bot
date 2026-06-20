/// Jito Bundle Builder
///
/// Composes multiple instructions into an atomic bundle for MEV protection:
/// 1. Flash loan borrow instruction
/// 2. Swap 1 instruction (borrow → intermediate)
/// 3. Swap 2 instruction (intermediate → borrow)
/// 4. Flash loan repayment instruction
///
/// All execute atomically or none at all (no partial execution)

use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    transaction::Transaction,
};
use log::{info, warn, error};
use std::fmt;

/// Bundle status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundleStatus {
    /// Bundle created, not yet submitted
    Created,
    /// Submitted to Jito
    Submitted,
    /// Included in a block
    Confirmed,
    /// Failed to execute
    Failed,
    /// Landed in a block
    Landed,
    /// Expired (not included in time)
    Expired,
}

impl fmt::Display for BundleStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BundleStatus::Created => write!(f, "Created"),
            BundleStatus::Submitted => write!(f, "Submitted"),
            BundleStatus::Confirmed => write!(f, "Confirmed"),
            BundleStatus::Failed => write!(f, "Failed"),
            BundleStatus::Landed => write!(f, "Landed"),
            BundleStatus::Expired => write!(f, "Expired"),
        }
    }
}

/// A single transaction in a bundle
#[derive(Debug, Clone)]
pub struct BundleTransaction {
    /// The transaction bytes
    pub transaction: Vec<u8>,
    /// Optional: skip if this transaction fails
    pub skip_preflight: bool,
}

impl BundleTransaction {
    pub fn new(transaction_bytes: Vec<u8>) -> Self {
        Self {
            transaction: transaction_bytes,
            skip_preflight: false,
        }
    }

    pub fn with_skip_preflight(mut self, skip: bool) -> Self {
        self.skip_preflight = skip;
        self
    }
}

/// Jito Bundle - atomic group of transactions
#[derive(Debug, Clone)]
pub struct JitoBundle {
    /// Bundle ID (UUID-like)
    pub bundle_id: String,
    /// Transactions in the bundle
    pub transactions: Vec<BundleTransaction>,
    /// Jito tip (incentive for inclusion)
    pub jito_tip: u64,
    /// Status
    pub status: BundleStatus,
    /// Created timestamp
    pub created_at: i64,
    /// Submitted timestamp
    pub submitted_at: Option<i64>,
    /// Block confirmation slot
    pub confirmed_slot: Option<u64>,
}

impl JitoBundle {
    /// Create new bundle
    pub fn new(bundle_id: String) -> Self {
        Self {
            bundle_id,
            transactions: Vec::new(),
            jito_tip: 0,
            status: BundleStatus::Created,
            created_at: chrono::Local::now().timestamp(),
            submitted_at: None,
            confirmed_slot: None,
        }
    }

    /// Add a transaction to the bundle
    pub fn add_transaction(&mut self, tx: BundleTransaction) -> &mut Self {
        self.transactions.push(tx);
        self
    }

    /// Set Jito tip
    pub fn set_tip(&mut self, tip: u64) -> &mut Self {
        self.jito_tip = tip;
        self
    }

    /// Get bundle size in bytes
    pub fn bundle_size(&self) -> usize {
        self.transactions.iter().map(|tx| tx.transaction.len()).sum()
    }

    /// Get transaction count
    pub fn transaction_count(&self) -> usize {
        self.transactions.len()
    }

    /// Validate bundle integrity
    pub fn validate(&self) -> Result<(), BundleError> {
        // Must have at least 2 transactions (swap + repay)
        if self.transactions.is_empty() {
            return Err(BundleError::EmptyBundle);
        }

        // Tip must be set
        if self.jito_tip == 0 {
            return Err(BundleError::NoTip);
        }

        // Bundle shouldn't be too large (reasonable limit)
        const MAX_BUNDLE_SIZE: usize = 1024 * 100; // 100KB
        if self.bundle_size() > MAX_BUNDLE_SIZE {
            return Err(BundleError::BundleTooLarge {
                actual: self.bundle_size(),
                max: MAX_BUNDLE_SIZE,
            });
        }

        Ok(())
    }

    /// Mark as submitted
    pub fn mark_submitted(&mut self) {
        self.status = BundleStatus::Submitted;
        self.submitted_at = Some(chrono::Local::now().timestamp());
        info!("📤 Bundle {} submitted", self.bundle_id);
    }

    /// Mark as confirmed
    pub fn mark_confirmed(&mut self, slot: u64) {
        self.status = BundleStatus::Confirmed;
        self.confirmed_slot = Some(slot);
        info!("✅ Bundle {} confirmed in slot {}", self.bundle_id, slot);
    }

    /// Mark as failed
    pub fn mark_failed(&mut self) {
        self.status = BundleStatus::Failed;
        warn!("❌ Bundle {} failed", self.bundle_id);
    }

    /// Mark as expired
    pub fn mark_expired(&mut self) {
        self.status = BundleStatus::Expired;
        warn!("⏱️ Bundle {} expired", self.bundle_id);
    }
}

/// Jito Bundle Builder
pub struct JitoBundleBuilder {
    /// Bundle being built
    pub bundle: JitoBundle,
    /// Payer for Jito tip
    payer: Option<Pubkey>,
}

impl JitoBundleBuilder {
    /// Create new bundle builder
    pub fn new(bundle_id: String) -> Self {
        Self {
            bundle: JitoBundle::new(bundle_id),
            payer: None,
        }
    }

    /// Set the payer (signer)
    pub fn with_payer(mut self, payer: Pubkey) -> Self {
        self.payer = Some(payer);
        self
    }

    /// Add flash loan instruction
    pub fn add_flash_loan_instruction(
        mut self,
        instruction_bytes: Vec<u8>,
    ) -> Result<Self, BundleError> {
        if self.payer.is_none() {
            return Err(BundleError::NoPayer);
        }

        let tx = BundleTransaction::new(instruction_bytes);
        self.bundle.add_transaction(tx);
        info!("📥 Added flash loan instruction");
        Ok(self)
    }

    /// Add swap instruction
    pub fn add_swap_instruction(
        mut self,
        instruction_bytes: Vec<u8>,
        swap_number: u8,
    ) -> Result<Self, BundleError> {
        if swap_number < 1 || swap_number > 2 {
            return Err(BundleError::InvalidSwapNumber);
        }

        let tx = BundleTransaction::new(instruction_bytes);
        self.bundle.add_transaction(tx);
        info!("🔄 Added swap {} instruction", swap_number);
        Ok(self)
    }

    /// Add repayment instruction
    pub fn add_repayment_instruction(
        mut self,
        instruction_bytes: Vec<u8>,
    ) -> Result<Self, BundleError> {
        let tx = BundleTransaction::new(instruction_bytes);
        self.bundle.add_transaction(tx);
        info!("💰 Added repayment instruction");
        Ok(self)
    }

    /// Set Jito tip
    pub fn with_tip(mut self, tip: u64) -> Self {
        self.bundle.set_tip(tip);
        info!("💸 Set Jito tip: {} lamports", tip);
        self
    }

    /// Build the final bundle
    pub fn build(self) -> Result<JitoBundle, BundleError> {
        self.bundle.validate()?;
        info!("✅ Bundle built successfully with {} transactions", self.bundle.transaction_count());
        Ok(self.bundle)
    }
}

/// Bundle builder error
#[derive(Debug, Clone)]
pub enum BundleError {
    /// No payer specified
    NoPayer,
    /// Bundle is empty
    EmptyBundle,
    /// No tip set
    NoTip,
    /// Invalid swap number (must be 1 or 2)
    InvalidSwapNumber,
    /// Bundle too large
    BundleTooLarge { actual: usize, max: usize },
    /// Failed to serialize instruction
    SerializationFailed(String),
    /// Failed to sign bundle
    SigningFailed(String),
    /// Failed to submit bundle
    SubmissionFailed(String),
    /// Bundle not found
    BundleNotFound(String),
    /// Invalid bundle state
    InvalidState(String),
    /// Tip calculation error
    TipCalculationError(String),
}

impl fmt::Display for BundleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BundleError::NoPayer => write!(f, "No payer specified for bundle"),
            BundleError::EmptyBundle => write!(f, "Bundle is empty"),
            BundleError::NoTip => write!(f, "No tip set for bundle"),
            BundleError::InvalidSwapNumber => write!(f, "Invalid swap number (must be 1 or 2)"),
            BundleError::BundleTooLarge { actual, max } => {
                write!(f, "Bundle too large: {} bytes (max: {} bytes)", actual, max)
            }
            BundleError::SerializationFailed(msg) => write!(f, "Serialization failed: {}", msg),
            BundleError::SigningFailed(msg) => write!(f, "Signing failed: {}", msg),
            BundleError::SubmissionFailed(msg) => write!(f, "Submission failed: {}", msg),
            BundleError::BundleNotFound(id) => write!(f, "Bundle not found: {}", id),
            BundleError::InvalidState(msg) => write!(f, "Invalid state: {}", msg),
            BundleError::TipCalculationError(msg) => write!(f, "Tip calculation error: {}", msg),
        }
    }
}

impl std::error::Error for BundleError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundle_creation() {
        let bundle = JitoBundle::new("bundle-1".to_string());
        assert_eq!(bundle.bundle_id, "bundle-1");
        assert_eq!(bundle.status, BundleStatus::Created);
        assert_eq!(bundle.transaction_count(), 0);
    }

    #[test]
    fn test_bundle_add_transactions() {
        let mut bundle = JitoBundle::new("bundle-2".to_string());
        
        bundle.add_transaction(BundleTransaction::new(vec![1, 2, 3]));
        bundle.add_transaction(BundleTransaction::new(vec![4, 5, 6]));
        
        assert_eq!(bundle.transaction_count(), 2);
        assert_eq!(bundle.bundle_size(), 6);
    }

    #[test]
    fn test_bundle_set_tip() {
        let mut bundle = JitoBundle::new("bundle-3".to_string());
        bundle.set_tip(5000);
        assert_eq!(bundle.jito_tip, 5000);
    }

    #[test]
    fn test_bundle_validation_empty() {
        let bundle = JitoBundle::new("bundle-4".to_string());
        assert!(matches!(bundle.validate(), Err(BundleError::EmptyBundle)));
    }

    #[test]
    fn test_bundle_validation_no_tip() {
        let mut bundle = JitoBundle::new("bundle-5".to_string());
        bundle.add_transaction(BundleTransaction::new(vec![1, 2]));
        assert!(matches!(bundle.validate(), Err(BundleError::NoTip)));
    }

    #[test]
    fn test_bundle_validation_success() {
        let mut bundle = JitoBundle::new("bundle-6".to_string());
        bundle.add_transaction(BundleTransaction::new(vec![1, 2]));
        bundle.set_tip(1000);
        assert!(bundle.validate().is_ok());
    }

    #[test]
    fn test_bundle_builder() {
        let payer = Pubkey::new_unique();
        let result = JitoBundleBuilder::new("bundle-7".to_string())
            .with_payer(payer)
            .with_tip(5000);
        
        assert_eq!(result.bundle.jito_tip, 5000);
    }

    #[test]
    fn test_bundle_status_transitions() {
        let mut bundle = JitoBundle::new("bundle-8".to_string());
        
        assert_eq!(bundle.status, BundleStatus::Created);
        
        bundle.mark_submitted();
        assert_eq!(bundle.status, BundleStatus::Submitted);
        
        bundle.mark_confirmed(12345);
        assert_eq!(bundle.status, BundleStatus::Confirmed);
        assert_eq!(bundle.confirmed_slot, Some(12345));
    }

    #[test]
    fn test_bundle_too_large() {
        let mut bundle = JitoBundle::new("bundle-9".to_string());
        bundle.add_transaction(BundleTransaction::new(vec![0; 101 * 1024])); // 101KB
        bundle.set_tip(1000);
        assert!(matches!(bundle.validate(), Err(BundleError::BundleTooLarge { .. })));
    }

    #[test]
    fn test_builder_no_payer() {
        let result = JitoBundleBuilder::new("bundle-10".to_string())
            .add_flash_loan_instruction(vec![1, 2, 3]);
        
        assert!(result.is_err());
    }
}
