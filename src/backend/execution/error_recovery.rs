/// Error Recovery System
///
/// Handles failed arbitrage executions:
/// - Retry failed transactions with exponential backoff
/// - Recover from partial execution
/// - Handle flash loan repayment failures
/// - Track failure reasons for analysis
///
/// Strategy:
/// 1. Detect failure reason
/// 2. Attempt recovery (if possible)
/// 3. Log event for analysis
/// 4. Alert user if unrecoverable

use log::{info, warn, error};
use std::fmt;

/// Execution failure reason
#[derive(Debug, Clone)]
pub enum ExecutionError {
    /// Transaction simulation failed
    SimulationFailed(String),
    /// Insufficient balance for execution
    InsufficientBalance { required: u64, available: u64 },
    /// Slippage exceeded limits
    ExcessiveSlippage { expected: u64, actual: u64 },
    /// Flash loan repayment failed
    RepaymentFailed(String),
    /// Swap failed on DEX
    SwapFailed { dex: String, reason: String },
    /// Network error
    NetworkError(String),
    /// Timeout waiting for confirmation
    ConfirmationTimeout,
    /// Keypair signing failed
    SigningFailed(String),
    /// RPC endpoint failure
    RpcError(String),
    /// Bundle submission failed
    BundleSubmissionFailed(String),
    /// Partial execution (some swaps succeeded, others failed)
    PartialExecution { succeeded: u32, failed: u32 },
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecutionError::SimulationFailed(msg) => write!(f, "Simulation failed: {}", msg),
            ExecutionError::InsufficientBalance { required, available } => {
                write!(f, "Insufficient balance: need {} but have {}", required, available)
            }
            ExecutionError::ExcessiveSlippage { expected, actual } => {
                write!(f, "Slippage exceeded: expected {}, got {}", expected, actual)
            }
            ExecutionError::RepaymentFailed(msg) => write!(f, "Flash loan repayment failed: {}", msg),
            ExecutionError::SwapFailed { dex, reason } => {
                write!(f, "{} swap failed: {}", dex, reason)
            }
            ExecutionError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            ExecutionError::ConfirmationTimeout => write!(f, "Timeout waiting for transaction confirmation"),
            ExecutionError::SigningFailed(msg) => write!(f, "Transaction signing failed: {}", msg),
            ExecutionError::RpcError(msg) => write!(f, "RPC error: {}", msg),
            ExecutionError::BundleSubmissionFailed(msg) => write!(f, "Bundle submission failed: {}", msg),
            ExecutionError::PartialExecution { succeeded, failed } => {
                write!(f, "Partial execution: {} succeeded, {} failed", succeeded, failed)
            }
        }
    }
}

impl std::error::Error for ExecutionError {}

/// Recovery action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryAction {
    /// Retry the entire transaction
    Retry,
    /// Skip this opportunity and move on
    Skip,
    /// Alert user and stop
    Alert,
    /// Attempt to rollback (repay flash loan only)
    Rollback,
}

/// Error recovery result
#[derive(Debug, Clone)]
pub struct RecoveryResult {
    /// What action was taken
    pub action: RecoveryAction,
    /// Whether recovery succeeded
    pub success: bool,
    /// Message for user
    pub message: String,
    /// Retry count
    pub retry_count: u32,
}

/// Error Recovery Manager
pub struct ErrorRecoveryManager {
    /// Max retries per transaction
    pub max_retries: u32,
    /// Current retry count
    pub retry_count: u32,
    /// Enable automatic retry
    pub auto_retry: bool,
    /// Minimum balance buffer (lamports)
    pub balance_buffer: u64,
}

impl ErrorRecoveryManager {
    /// Create new recovery manager
    pub fn new(max_retries: u32) -> Self {
        Self {
            max_retries,
            retry_count: 0,
            auto_retry: true,
            balance_buffer: 10_000, // 0.01 lamports buffer
        }
    }

    /// Determine recovery action for an error
    pub fn determine_recovery(&self, error: &ExecutionError) -> RecoveryAction {
        match error {
            // Retryable errors
            ExecutionError::NetworkError(_) |
            ExecutionError::RpcError(_) |
            ExecutionError::ConfirmationTimeout => {
                if self.retry_count < self.max_retries {
                    RecoveryAction::Retry
                } else {
                    RecoveryAction::Alert
                }
            }

            // Might be retryable
            ExecutionError::SimulationFailed(_) => RecoveryAction::Retry,

            // Balance issue - skip this opportunity
            ExecutionError::InsufficientBalance { .. } => RecoveryAction::Skip,

            // Slippage exceeded - alert user
            ExecutionError::ExcessiveSlippage { .. } => RecoveryAction::Alert,

            // Swap failed - try to rollback (repay flash loan)
            ExecutionError::SwapFailed { .. } => RecoveryAction::Rollback,

            // Flash loan repayment failure - critical alert
            ExecutionError::RepaymentFailed(_) => RecoveryAction::Alert,

            // Bundle submission failed - retry
            ExecutionError::BundleSubmissionFailed(_) => RecoveryAction::Retry,

            // Signing failed - skip
            ExecutionError::SigningFailed(_) => RecoveryAction::Skip,

            // Partial execution - critical alert (funds might be stuck)
            ExecutionError::PartialExecution { .. } => RecoveryAction::Alert,
        }
    }

    /// Attempt recovery for an error
    pub fn recover(&mut self, error: &ExecutionError) -> RecoveryResult {
        let action = self.determine_recovery(error);

        match action {
            RecoveryAction::Retry => {
                self.retry_count += 1;
                info!(
                    "🔄 Attempting retry {} of {}",
                    self.retry_count, self.max_retries
                );
                RecoveryResult {
                    action,
                    success: true,
                    message: format!("Retrying... (attempt {}/{})", self.retry_count, self.max_retries),
                    retry_count: self.retry_count,
                }
            }

            RecoveryAction::Skip => {
                warn!("⏭️ Skipping this opportunity: {}", error);
                RecoveryResult {
                    action,
                    success: true,
                    message: "Skipped this opportunity".to_string(),
                    retry_count: self.retry_count,
                }
            }

            RecoveryAction::Alert => {
                error!("🚨 Alert: {}", error);
                RecoveryResult {
                    action,
                    success: false,
                    message: format!("Critical error: {}", error),
                    retry_count: self.retry_count,
                }
            }

            RecoveryAction::Rollback => {
                warn!("↩️ Attempting rollback for: {}", error);
                RecoveryResult {
                    action,
                    success: true,
                    message: "Attempting to rollback (repay flash loan)".to_string(),
                    retry_count: self.retry_count,
                }
            }
        }
    }

    /// Reset retry counter
    pub fn reset(&mut self) {
        self.retry_count = 0;
    }

    /// Check if we can retry
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries && self.auto_retry
    }

    /// Get retry delay (exponential backoff)
    pub fn get_retry_delay_ms(&self) -> u64 {
        // 100ms * 2^(retry_count-1)
        // 100ms, 200ms, 400ms, 800ms, 1.6s...
        let base = 100u64;
        let multiplier = 2u64.pow(self.retry_count.saturating_sub(1));
        let delay = base.saturating_mul(multiplier);
        
        // Cap at 10 seconds
        std::cmp::min(delay, 10_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recovery_action_network_error() {
        let manager = ErrorRecoveryManager::new(3);
        let error = ExecutionError::NetworkError("Connection lost".to_string());
        
        let action = manager.determine_recovery(&error);
        assert_eq!(action, RecoveryAction::Retry);
    }

    #[test]
    fn test_recovery_action_insufficient_balance() {
        let manager = ErrorRecoveryManager::new(3);
        let error = ExecutionError::InsufficientBalance {
            required: 1_000_000,
            available: 500_000,
        };
        
        let action = manager.determine_recovery(&error);
        assert_eq!(action, RecoveryAction::Skip);
    }

    #[test]
    fn test_recovery_action_max_retries() {
        let mut manager = ErrorRecoveryManager::new(2);
        manager.retry_count = 2; // Already at max
        
        let error = ExecutionError::NetworkError("Connection lost".to_string());
        let action = manager.determine_recovery(&error);
        
        // Should alert instead of retry since max retries reached
        assert_eq!(action, RecoveryAction::Alert);
    }

    #[test]
    fn test_exponential_backoff() {
        let mut manager = ErrorRecoveryManager::new(5);
        
        // Retry 1: 100ms
        assert_eq!(manager.get_retry_delay_ms(), 100);
        
        manager.retry_count = 2;
        // Retry 2: 200ms
        assert_eq!(manager.get_retry_delay_ms(), 200);
        
        manager.retry_count = 3;
        // Retry 3: 400ms
        assert_eq!(manager.get_retry_delay_ms(), 400);
        
        manager.retry_count = 4;
        // Retry 4: 800ms
        assert_eq!(manager.get_retry_delay_ms(), 800);
        
        manager.retry_count = 8;
        // Capped at 10000ms
        assert_eq!(manager.get_retry_delay_ms(), 10_000);
    }

    #[test]
    fn test_recover_flow() {
        let mut manager = ErrorRecoveryManager::new(3);
        let error = ExecutionError::NetworkError("Connection lost".to_string());
        
        let result = manager.recover(&error);
        
        assert_eq!(result.action, RecoveryAction::Retry);
        assert!(result.success);
        assert_eq!(result.retry_count, 1);
    }

    #[test]
    fn test_reset() {
        let mut manager = ErrorRecoveryManager::new(3);
        manager.retry_count = 3;
        
        manager.reset();
        assert_eq!(manager.retry_count, 0);
    }
}
