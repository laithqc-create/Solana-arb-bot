/// Execution Coordinator
///
/// Orchestrates complete arbitrage execution pipeline:
/// 1. Validate opportunity
/// 2. Sign transaction
/// 3. Submit to Jito bundle
/// 4. Track confirmation
/// 5. Handle errors with recovery
///
/// Manages state across all execution steps

pub mod error_recovery;
pub mod transaction_signer;

pub use error_recovery::{ErrorRecoveryManager, ExecutionError, RecoveryAction};
pub use transaction_signer::{TransactionSigner, TransactionTracker, SubmissionStatus};

use log::{info, warn, error};
use std::fmt;

/// Execution state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionState {
    /// Waiting to execute
    Pending,
    /// Validating opportunity
    Validating,
    /// Signing transaction
    Signing,
    /// Submitting to bundle
    Submitting,
    /// Waiting for confirmation
    Confirming,
    /// Completed successfully
    Success,
    /// Failed with recovery attempt
    RecoveringFromError,
    /// Final failure
    Failed,
}

impl fmt::Display for ExecutionState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecutionState::Pending => write!(f, "Pending"),
            ExecutionState::Validating => write!(f, "Validating"),
            ExecutionState::Signing => write!(f, "Signing"),
            ExecutionState::Submitting => write!(f, "Submitting"),
            ExecutionState::Confirming => write!(f, "Confirming"),
            ExecutionState::Success => write!(f, "Success"),
            ExecutionState::RecoveringFromError => write!(f, "Recovering"),
            ExecutionState::Failed => write!(f, "Failed"),
        }
    }
}

/// Execution result summary
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Final state
    pub state: ExecutionState,
    /// Transaction signature
    pub signature: Option<String>,
    /// Profit achieved (if successful)
    pub profit: Option<u64>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Recovery action taken (if any)
    pub recovery_action: Option<RecoveryAction>,
    /// Total attempts
    pub attempts: u32,
    /// Execution time (ms)
    pub execution_time_ms: u64,
}

/// Execution Coordinator
pub struct ExecutionCoordinator {
    /// Current execution state
    pub state: ExecutionState,
    /// Error recovery manager
    recovery_manager: ErrorRecoveryManager,
    /// Transaction signer
    signer: Option<TransactionSigner>,
    /// Active transaction tracker
    tracker: Option<TransactionTracker>,
    /// Attempt count
    attempts: u32,
    /// Start time (ms since epoch)
    start_time: i64,
}

impl ExecutionCoordinator {
    /// Create new coordinator
    pub fn new() -> Self {
        Self {
            state: ExecutionState::Pending,
            recovery_manager: ErrorRecoveryManager::new(3), // Max 3 retries
            signer: None,
            tracker: None,
            attempts: 0,
            start_time: chrono::Local::now().timestamp_millis(),
        }
    }

    /// Set transaction signer
    pub fn set_signer(&mut self, signer: TransactionSigner) {
        self.signer = Some(signer);
        info!("📝 Signer set: {}", signer.public_key());
    }

    /// Validate arbitrage opportunity
    pub fn validate_opportunity(
        &mut self,
        profit_lamports: u64,
        slippage_bps: u64,
    ) -> Result<(), ExecutionError> {
        self.state = ExecutionState::Validating;
        info!("✅ Validating opportunity: profit={}, slippage={}bps", profit_lamports, slippage_bps);

        // Minimum profit check
        if profit_lamports < 1_000 {
            return Err(ExecutionError::SimulationFailed(
                "Profit too low (< 1000 lamports)".to_string(),
            ));
        }

        // Slippage check (max 50 bps = 0.5%)
        if slippage_bps > 50 {
            return Err(ExecutionError::ExcessiveSlippage {
                expected: profit_lamports,
                actual: profit_lamports.saturating_mul(10000 - slippage_bps) / 10000,
            });
        }

        info!("✅ Opportunity validated");
        Ok(())
    }

    /// Sign and prepare transaction
    pub fn sign_transaction(&mut self) -> Result<String, ExecutionError> {
        self.state = ExecutionState::Signing;
        self.attempts += 1;

        let _signer = self.signer
            .as_ref()
            .ok_or_else(|| ExecutionError::SigningFailed("No signer configured".to_string()))?;

        info!("📝 Signing transaction (attempt {})", self.attempts);

        // Placeholder: In real implementation, would sign actual transaction
        let signature = format!("sig_{}", chrono::Local::now().timestamp_millis());
        
        let tracker = TransactionTracker::new(signature.clone());
        self.tracker = Some(tracker);

        info!("✅ Transaction signed: {}", signature);
        Ok(signature)
    }

    /// Submit to Jito bundle
    pub fn submit_to_bundle(&mut self, _bundle_id: String) -> Result<(), ExecutionError> {
        self.state = ExecutionState::Submitting;

        let tracker = self.tracker
            .as_ref()
            .ok_or_else(|| ExecutionError::BundleSubmissionFailed("No transaction to submit".to_string()))?;

        info!("📤 Submitting bundle with {}", tracker.signature);

        // Placeholder: In real implementation, would submit to Jito
        info!("✅ Bundle submitted");
        Ok(())
    }

    /// Confirm transaction
    pub fn confirm_transaction(&mut self) -> Result<(), ExecutionError> {
        self.state = ExecutionState::Confirming;

        if let Some(tracker) = &mut self.tracker {
            tracker.mark_confirmed();
            info!("✅ Transaction confirmed after {:?}ms", tracker.confirmation_time_ms());
            Ok(())
        } else {
            Err(ExecutionError::ConfirmationTimeout)
        }
    }

    /// Mark execution as successful
    pub fn mark_success(&mut self, profit: u64) {
        self.state = ExecutionState::Success;
        
        if let Some(tracker) = &mut self.tracker {
            tracker.mark_finalized();
        }

        let elapsed = chrono::Local::now().timestamp_millis() - self.start_time;
        info!(
            "🎉 Arbitrage successful: {} lamports profit in {}ms",
            profit, elapsed
        );
    }

    /// Handle execution error
    pub fn handle_error(&mut self, error: ExecutionError) -> Result<RecoveryAction, ExecutionError> {
        error!("❌ Execution error: {}", error);
        
        self.state = ExecutionState::RecoveringFromError;

        let recovery_result = self.recovery_manager.recover(&error);
        
        match recovery_result.action {
            RecoveryAction::Retry => {
                warn!("🔄 Will retry after delay");
                Ok(RecoveryAction::Retry)
            }
            RecoveryAction::Skip => {
                warn!("⏭️ Skipping this opportunity");
                self.state = ExecutionState::Failed;
                Ok(RecoveryAction::Skip)
            }
            RecoveryAction::Alert => {
                error!("🚨 Critical error, alerting user");
                self.state = ExecutionState::Failed;
                Err(error)
            }
            RecoveryAction::Rollback => {
                warn!("↩️ Attempting rollback");
                Ok(RecoveryAction::Rollback)
            }
        }
    }

    /// Get execution summary
    pub fn get_summary(&self) -> ExecutionResult {
        let elapsed = chrono::Local::now().timestamp_millis() - self.start_time;

        ExecutionResult {
            state: self.state,
            signature: self.tracker.as_ref().map(|t| t.signature.clone()),
            profit: None, // Set by caller
            error: self.tracker
                .as_ref()
                .and_then(|t| t.last_error.clone()),
            recovery_action: None, // Set by caller
            attempts: self.attempts,
            execution_time_ms: elapsed as u64,
        }
    }

    /// Can retry execution?
    pub fn can_retry(&self) -> bool {
        self.recovery_manager.can_retry()
    }

    /// Get retry delay (ms)
    pub fn get_retry_delay_ms(&self) -> u64 {
        self.recovery_manager.get_retry_delay_ms()
    }

    /// Reset for retry
    pub fn reset_for_retry(&mut self) {
        self.state = ExecutionState::Pending;
        self.tracker = None;
        info!("🔄 Reset for retry");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinator_creation() {
        let coordinator = ExecutionCoordinator::new();
        
        assert_eq!(coordinator.state, ExecutionState::Pending);
        assert_eq!(coordinator.attempts, 0);
    }

    #[test]
    fn test_validate_opportunity_good() {
        let mut coordinator = ExecutionCoordinator::new();
        
        let result = coordinator.validate_opportunity(10_000, 25);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_opportunity_low_profit() {
        let mut coordinator = ExecutionCoordinator::new();
        
        let result = coordinator.validate_opportunity(500, 25);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_opportunity_high_slippage() {
        let mut coordinator = ExecutionCoordinator::new();
        
        let result = coordinator.validate_opportunity(10_000, 100); // 1% slippage
        assert!(result.is_err());
    }

    #[test]
    fn test_sign_transaction_no_signer() {
        let mut coordinator = ExecutionCoordinator::new();
        
        let result = coordinator.sign_transaction();
        assert!(result.is_err());
    }

    #[test]
    fn test_execution_flow() {
        let mut coordinator = ExecutionCoordinator::new();
        
        // Validate
        assert!(coordinator.validate_opportunity(10_000, 25).is_ok());
        
        // Sign (without actual signer, just placeholder)
        let sig = coordinator.sign_transaction();
        assert!(sig.is_ok());
        
        // Confirm
        assert!(coordinator.confirm_transaction().is_ok());
        
        // Mark success
        coordinator.mark_success(10_000);
        assert_eq!(coordinator.state, ExecutionState::Success);
    }

    #[test]
    fn test_retry_logic() {
        let mut coordinator = ExecutionCoordinator::new();
        
        assert!(coordinator.can_retry());
        
        let error = ExecutionError::NetworkError("Connection lost".to_string());
        let recovery = coordinator.handle_error(error);
        
        assert!(recovery.is_ok());
        assert_eq!(recovery.unwrap(), RecoveryAction::Retry);
    }

    #[test]
    fn test_get_summary() {
        let coordinator = ExecutionCoordinator::new();
        let summary = coordinator.get_summary();
        
        assert_eq!(summary.state, ExecutionState::Pending);
        assert_eq!(summary.attempts, 0);
    }
}
