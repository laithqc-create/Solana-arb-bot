/// Execution Manager for Live Arbitrage
/// 
/// Orchestrates the complete execution pipeline:
/// 1. Detect opportunity (from Geyser stream)
/// 2. Validate profitability
/// 3. Build swap sequence
/// 4. Simulate before submit
/// 5. Submit to Jito bundle (Phase 2.3)
/// 6. Monitor transaction status
/// 7. Log results to journal

use super::atomic_swap::{AtomicSwapExecutor, ArbitrageOpportunity, SwapConfig};
use log::{info, warn, error};
use std::time::{SystemTime, UNIX_EPOCH};

/// Execution status
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionStatus {
    Pending,
    Validated,
    ReadyToSubmit,
    Submitted,
    Confirmed,
    Failed(String),
}

/// Complete execution record
#[derive(Debug, Clone)]
pub struct ExecutionRecord {
    /// Unique execution ID
    pub id: String,
    /// Timestamp when execution was created
    pub timestamp: u64,
    /// Arbitrage opportunity executed
    pub opportunity: ArbitrageOpportunity,
    /// Execution status
    pub status: ExecutionStatus,
    /// Gross profit before fees
    pub gross_profit: u64,
    /// Net profit after all fees
    pub net_profit: u64,
    /// Transaction signature (if submitted)
    pub tx_signature: Option<String>,
    /// Error message (if failed)
    pub error_message: Option<String>,
}

impl ExecutionRecord {
    /// Create new execution record
    pub fn new(opp: ArbitrageOpportunity) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            id: format!("exec_{}", timestamp),
            timestamp,
            opportunity: opp,
            status: ExecutionStatus::Pending,
            gross_profit: 0,
            net_profit: 0,
            tx_signature: None,
            error_message: None,
        }
    }

    /// Mark execution as validated
    pub fn mark_validated(&mut self, gross_profit: u64, net_profit: u64) {
        self.status = ExecutionStatus::Validated;
        self.gross_profit = gross_profit;
        self.net_profit = net_profit;
        info!("✅ Execution {} validated: profit {} lamports", self.id, net_profit);
    }

    /// Mark as ready for submission
    pub fn mark_ready(&mut self) {
        self.status = ExecutionStatus::ReadyToSubmit;
        info!("🎯 Execution {} ready for submission", self.id);
    }

    /// Mark as submitted
    pub fn mark_submitted(&mut self, tx_sig: String) {
        self.status = ExecutionStatus::Submitted;
        self.tx_signature = Some(tx_sig.clone());
        info!("📤 Execution {} submitted: {}", self.id, tx_sig);
    }

    /// Mark as confirmed
    pub fn mark_confirmed(&mut self) {
        self.status = ExecutionStatus::Confirmed;
        info!("✅ Execution {} confirmed!", self.id);
    }

    /// Mark as failed
    pub fn mark_failed(&mut self, error: String) {
        self.status = ExecutionStatus::Failed(error.clone());
        self.error_message = Some(error.clone());
        error!("❌ Execution {} failed: {}", self.id, error);
    }

    /// Format for trade journal
    pub fn format_for_journal(&self) -> String {
        format!(
            "ID: {} | Status: {:?} | Input: {} | Gross: {} | Net: {} | Sig: {} | Error: {}",
            self.id,
            self.status,
            self.opportunity.input_amount,
            self.gross_profit,
            self.net_profit,
            self.tx_signature.as_ref().unwrap_or(&"N/A".to_string()),
            self.error_message.as_ref().unwrap_or(&"None".to_string()),
        )
    }
}

/// Execution Manager
pub struct ExecutionManager {
    swap_executor: AtomicSwapExecutor,
    /// History of all executions
    execution_history: Vec<ExecutionRecord>,
    /// Total profit across all executions
    total_profit: u64,
    /// Total executions attempted
    total_attempts: u32,
    /// Successful executions
    successful_executions: u32,
}

impl ExecutionManager {
    /// Create new execution manager
    pub fn new(swap_config: SwapConfig) -> Self {
        Self {
            swap_executor: AtomicSwapExecutor::new(swap_config),
            execution_history: Vec::new(),
            total_profit: 0,
            total_attempts: 0,
            successful_executions: 0,
        }
    }

    /// Execute an arbitrage opportunity
    /// 
    /// Pipeline:
    /// 1. Validate profitability
    /// 2. Build swap instructions
    /// 3. Simulate execution
    /// 4. Prepare for submission (Phase 2.3)
    pub async fn execute_opportunity(
        &mut self,
        opp: ArbitrageOpportunity,
    ) -> Result<ExecutionRecord, String> {
        let mut record = ExecutionRecord::new(opp.clone());
        self.total_attempts += 1;

        info!("🚀 Starting execution pipeline for opportunity...");
        info!("   Input: {} lamports", opp.input_amount);

        // Step 1: Validate profitability
        match self.swap_executor.validate_opportunity(&opp) {
            Ok(validated) => {
                record.mark_validated(validated.gross_profit, validated.net_profit);

                // Step 2: Build swap instructions
                match self.swap_executor.build_swap_instructions(&validated) {
                    Ok(instructions) => {
                        info!("✅ Built {} instruction(s)", instructions.len());

                        // Step 3: Simulate before submit
                        // TODO: Integrate with RPC manager to actually simulate
                        match self.swap_executor.simulate_swap(&unimplemented!()).await {
                            Ok(sim_result) => {
                                if sim_result.successful {
                                    info!("✅ Simulation passed: {} compute units", sim_result.compute_units_used);
                                    record.mark_ready();

                                    // Update statistics
                                    self.total_profit = self.total_profit.saturating_add(record.net_profit);
                                    self.successful_executions += 1;

                                    self.execution_history.push(record.clone());
                                    Ok(record)
                                } else {
                                    let error = format!("Simulation failed: {:?}", sim_result.error);
                                    record.mark_failed(error.clone());
                                    self.execution_history.push(record.clone());
                                    Err(error)
                                }
                            }
                            Err(e) => {
                                record.mark_failed(e.clone());
                                self.execution_history.push(record.clone());
                                Err(e)
                            }
                        }
                    }
                    Err(e) => {
                        record.mark_failed(e.clone());
                        self.execution_history.push(record.clone());
                        Err(e)
                    }
                }
            }
            Err(e) => {
                record.mark_failed(e.clone());
                self.execution_history.push(record.clone());
                Err(e)
            }
        }
    }

    /// Get execution history
    pub fn get_history(&self) -> &[ExecutionRecord] {
        &self.execution_history
    }

    /// Get statistics
    pub fn get_stats(&self) -> ExecutionStats {
        ExecutionStats {
            total_attempts: self.total_attempts,
            successful_executions: self.successful_executions,
            failed_executions: self.total_attempts.saturating_sub(self.successful_executions),
            success_rate: if self.total_attempts > 0 {
                (self.successful_executions as f64 / self.total_attempts as f64) * 100.0
            } else {
                0.0
            },
            total_profit: self.total_profit,
            average_profit_per_execution: if self.successful_executions > 0 {
                self.total_profit / self.successful_executions as u64
            } else {
                0
            },
        }
    }

    /// Format trade journal entry
    pub fn format_trade_journal(&self) -> String {
        let stats = self.get_stats();
        let mut journal = String::new();

        journal.push_str(&format!(
            "=== TRADE JOURNAL ===\n\
             Attempts: {} | Success: {} | Failed: {} | Success Rate: {:.1}%\n\
             Total Profit: {} lamports | Avg per Trade: {} lamports\n\n",
            stats.total_attempts,
            stats.successful_executions,
            stats.failed_executions,
            stats.success_rate,
            stats.total_profit,
            stats.average_profit_per_execution
        ));

        journal.push_str("Recent Executions:\n");
        for record in self.execution_history.iter().rev().take(10) {
            journal.push_str(&format!("{}\n", record.format_for_journal()));
        }

        journal
    }
}

/// Execution statistics
#[derive(Debug, Clone)]
pub struct ExecutionStats {
    pub total_attempts: u32,
    pub successful_executions: u32,
    pub failed_executions: u32,
    pub success_rate: f64,
    pub total_profit: u64,
    pub average_profit_per_execution: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_record_lifecycle() {
        let mut record = ExecutionRecord::new(create_test_opp());
        assert_eq!(record.status, ExecutionStatus::Pending);

        record.mark_validated(100_000, 50_000);
        assert_eq!(record.status, ExecutionStatus::Validated);

        record.mark_ready();
        assert_eq!(record.status, ExecutionStatus::ReadyToSubmit);

        record.mark_submitted("sig123".to_string());
        assert_eq!(record.status, ExecutionStatus::Submitted);
        assert_eq!(record.tx_signature, Some("sig123".to_string()));

        record.mark_confirmed();
        assert_eq!(record.status, ExecutionStatus::Confirmed);
    }

    #[test]
    fn test_execution_manager_stats() {
        let mut manager = ExecutionManager::new(SwapConfig::default());
        
        // Simulate an execution
        let mut record = ExecutionRecord::new(create_test_opp());
        record.mark_validated(100_000, 50_000);
        manager.execution_history.push(record);
        manager.successful_executions = 1;
        manager.total_attempts = 1;
        manager.total_profit = 50_000;

        let stats = manager.get_stats();
        assert_eq!(stats.total_attempts, 1);
        assert_eq!(stats.successful_executions, 1);
        assert_eq!(stats.success_rate, 100.0);
        assert_eq!(stats.total_profit, 50_000);
    }

    #[test]
    fn test_trade_journal_formatting() {
        let mut manager = ExecutionManager::new(SwapConfig::default());
        
        let mut record = ExecutionRecord::new(create_test_opp());
        record.mark_validated(100_000, 50_000);
        manager.execution_history.push(record);
        manager.successful_executions = 1;
        manager.total_attempts = 1;
        manager.total_profit = 50_000;

        let journal = manager.format_trade_journal();
        assert!(journal.contains("TRADE JOURNAL"));
        assert!(journal.contains("Success Rate"));
        assert!(journal.contains("Total Profit"));
    }

    fn create_test_opp() -> ArbitrageOpportunity {
        use solana_sdk::pubkey::Pubkey;

        ArbitrageOpportunity {
            input_token: Pubkey::new_unique(),
            input_amount: 1_000_000,
            expected_output: 1_050_000,
            token_a: Pubkey::new_unique(),
            token_b: Pubkey::new_unique(),
            dex_1_program: Pubkey::new_unique(),
            dex_2_program: Pubkey::new_unique(),
            dex_1_pool: Pubkey::new_unique(),
            dex_2_pool: Pubkey::new_unique(),
            flash_loan_program: Pubkey::new_unique(),
            user_wallet: Pubkey::new_unique(),
        }
    }
}
