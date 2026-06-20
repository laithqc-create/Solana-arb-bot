# Phase 2: Live Arbitrage Execution - Implementation Guide

**Status:** Phase 1 ✅ Complete | Phase 2 🚀 Starting Now

**Timeline:** 2-3 weeks full implementation (solo dev)

---

## Table of Contents
1. [Architecture Overview](#architecture-overview)
2. [Prerequisites & Setup](#prerequisites--setup)
3. [Flash Loan Integration](#flash-loan-integration)
4. [Jito Bundle Execution](#jito-bundle-execution)
5. [Safety & Risk Management](#safety--risk-management)
6. [Testing Strategy](#testing-strategy)
7. [Deployment Checklist](#deployment-checklist)

---

## Architecture Overview

### Data Flow (Phase 2)
```
Geyser Stream (Phase 1)
    ↓
Pool Data Parser
    ↓
Arbitrage Detection Engine
    ↓
Flash Loan Contract Call
    ├─ Borrow from Protocol A
    ├─ Swap to Token B
    ├─ Swap back to Token A
    ├─ Repay + Fee
    └─ Profit to Wallet
    ↓
Jito Bundle Submission
    ├─ User Swap (Your Arb)
    ├─ MEV-protected execution
    └─ Atomic guarantee
    ↓
Blockchain Execution
    ↓
Trade Journal Log
```

### Components to Add

| Component | Language | Purpose | Existing? |
|-----------|----------|---------|-----------|
| Flash Loan Manager | Rust | Interact with Orca/Raydium flash loans | ❌ New |
| Bundle Builder | Rust | Create Jito bundles | ❌ New |
| Keypair Manager | Rust | Sign transactions securely | ✅ Partial |
| Transaction Signer | Rust | Sign & submit transactions | ❌ New |
| Jito Client | Rust | Submit to Jito block engine | ❌ New |
| Vault Integration | Rust | Secure keypair storage | ✅ Ready |
| Error Recovery | Rust | Handle failed executions | ❌ New |

---

## Prerequisites & Setup

### 1. **Obtain Required API Keys & Endpoints**

#### **A. Helius RPC (Ultra-low latency Geyser + Jito relay)**
```bash
# Cost: FREE tier available (100K requests/day)
# https://www.helius.dev

Steps:
1. Sign up at helius.dev
2. Create project → get API key
3. Copy your WebSocket endpoint:
   wss://mainnet.helius-rpc.com/?api-key=YOUR_KEY

4. Copy your HTTP endpoint:
   https://mainnet.helius-rpc.com/?api-key=YOUR_KEY
```

**Store in config:**
```json
{
  "helius_api_key": "YOUR_HELIUS_KEY",
  "helius_ws": "wss://mainnet.helius-rpc.com/?api-key=YOUR_HELIUS_KEY",
  "helius_http": "https://mainnet.helius-rpc.com/?api-key=YOUR_HELIUS_KEY"
}
```

#### **B. Jito Block Engine (Bundle submission)**
```bash
# Cost: FREE (no signup needed)
# Public endpoint: https://mainnet.block-engine.jito.wtf/api/v1/bundles

# Jito MainRPC for submission:
# https://mainnet.block-engine.jito.wtf/api/v1/bundles

# Jito SearchRPC (for bundle status):
# https://mainnet.block-engine.jito.wtf/api/v1/bundles
```

**Store in config:**
```json
{
  "jito_bundle_endpoint": "https://mainnet.block-engine.jito.wtf/api/v1/bundles",
  "jito_block_engine_url": "https://mainnet.block-engine.jito.wtf"
}
```

#### **C. Generate Solana Keypair (Your Wallet)**
```bash
# On your machine (NOT in code):
solana-keygen new --outfile ~/.config/solana/id.json

# This creates a keypair file. KEEP IT SAFE!
# You'll need ~0.1-1 SOL for:
#   - Transaction fees (per execution)
#   - Jito tips (MEV protection)
#   - Flashloan fees (varies by protocol)
```

**CRITICAL SECURITY:**
```
⚠️ NEVER commit keypair to GitHub
⚠️ NEVER hardcode private keys
⚠️ Use environment variables or encrypted vault
⚠️ Consider cold storage for mainnet keys
```

---

## Flash Loan Integration

### **What are Flash Loans?**

Flash loans are **uncollateralized loans** that must be repaid **in the same transaction**:

```
1. Borrow 1000 USDC from Protocol A
2. Swap to 1.05 ETH on DEX 1
3. Swap back to 1050 USDC on DEX 2
4. Repay 1000 USDC + 0.01% fee = 1000.1 USDC
5. Profit = 1050 - 1000.1 = 49.9 USDC per execution
```

### **Supported Protocols**

| Protocol | Flash Loan Fee | Min Amount | Max Amount | Notes |
|----------|----------------|-----------|-----------|-------|
| **Orca** | 0.0275% | $1 | Unlimited | Recommended (low fee) |
| **Raydium** | 0.05% | $1 | Unlimited | Good liquidity |
| **Solend** | 0.09% | $1 | Unlimited | Higher fee |
| **Marinade** | 0.01% | $1 | Unlimited | Best fee (stSOL only) |

**For this implementation, we'll use Orca (best balance of fee + liquidity)**

---

### **Step 1: Add Flash Loan Manager**

**Create file:** `src/backend/flash_loan/mod.rs`

```rust
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    program_pack::Pack,
    pubkey::Pubkey,
    signer::Signer,
    transaction::Transaction,
};
use solana_client::rpc_client::RpcClient;
use std::str::FromStr;

pub struct FlashLoanManager {
    rpc_client: RpcClient,
    payer: Pubkey,
}

impl FlashLoanManager {
    pub fn new(rpc_url: &str, payer: Pubkey) -> Self {
        Self {
            rpc_client: RpcClient::new(rpc_url),
            payer,
        }
    }

    /// Orca Flash Loan instruction
    pub fn create_orca_flash_loan(
        &self,
        pool_address: &str,
        token_mint: &str,
        amount: u64,
        instruction_data: Vec<u8>,
    ) -> Result<Instruction, Box<dyn std::error::Error>> {
        let pool = Pubkey::from_str(pool_address)?;
        let token = Pubkey::from_str(token_mint)?;

        // Orca Flash Loan Program
        let flash_loan_program = Pubkey::from_str(
            "9W957QEUQMax4GSLCxDLXpTK63gbLosLvmWXNrWgAg7"
        )?;

        // Build instruction accounts
        let accounts = vec![
            AccountMeta::new(pool, false),              // Pool account
            AccountMeta::new_readonly(token, false),    // Token mint
            AccountMeta::new(self.payer, false),        // Receiver (your wallet)
            AccountMeta::new_readonly(flash_loan_program, false),
        ];

        Ok(Instruction {
            program_id: flash_loan_program,
            accounts,
            data: instruction_data,
        })
    }

    /// Simulate flash loan execution (devnet safe)
    pub async fn simulate_flash_loan(
        &self,
        tx: &Transaction,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let result = self.rpc_client.simulate_transaction(tx)?;
        Ok(result.value.err.is_none())
    }

    /// Calculate required fee for flash loan
    pub fn calculate_flash_loan_fee(amount: u64, fee_bps: u64) -> u64 {
        // fee_bps = basis points (0.01% = 1 bps)
        (amount * fee_bps) / 10000
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fee_calculation() {
        let amount = 1_000_000; // 1M lamports
        let orca_fee = 275; // 0.0275% = 275 bps
        let fee = FlashLoanManager::calculate_flash_loan_fee(amount, orca_fee);
        assert_eq!(fee, 275); // 0.0275% of 1M = 275
    }
}
```

**Update:** `src/backend/main.rs`

```rust
mod flash_loan;

use flash_loan::FlashLoanManager;

#[tauri::command]
async fn initialize_flash_loan(
    helius_url: String,
    payer_pubkey: String,
) -> Result<String, String> {
    let manager = FlashLoanManager::new(&helius_url, 
        payer_pubkey.parse().map_err(|e| format!("Invalid pubkey: {}", e))?
    );
    
    Ok("Flash loan manager initialized".to_string())
}
```

---

### **Step 2: Build the Atomic Swap Logic**

**Create file:** `src/backend/flash_loan/atomic_swap.rs`

```rust
use solana_sdk::pubkey::Pubkey;
use spl_token::instruction as token_instruction;
use std::str::FromStr;

pub struct AtomicSwapExecution {
    pub flash_loan_amount: u64,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub dex_1_program: Pubkey,  // Where we swap A→B
    pub dex_2_program: Pubkey,  // Where we swap B→A
    pub expected_profit: u64,
}

impl AtomicSwapExecution {
    /// Verify swap execution is profitable BEFORE submitting
    pub fn validate_profitability(
        &self,
        flash_loan_fee: u64,
        swap_1_output: u64,
        swap_2_output: u64,
        slippage_tolerance: u64, // in basis points (e.g., 50 = 0.5%)
    ) -> Result<bool, String> {
        // Calculate maximum acceptable slippage
        let max_slippage = (self.flash_loan_amount * slippage_tolerance) / 10000;
        
        // Minimum output after slippage
        let min_required = self.flash_loan_amount + flash_loan_fee + max_slippage;

        if swap_2_output < min_required {
            return Err(format!(
                "Insufficient output. Got {}, need {}",
                swap_2_output, min_required
            ));
        }

        let profit = swap_2_output - self.flash_loan_amount - flash_loan_fee;
        
        if profit < 1000 {
            // Minimum 1000 lamports profit (0.000001 SOL)
            return Err("Profit below minimum threshold".to_string());
        }

        Ok(true)
    }

    /// Build instruction sequence
    pub fn build_instruction_sequence(
        &self,
        your_wallet: Pubkey,
    ) -> Vec<solana_sdk::instruction::Instruction> {
        vec![
            // 1. Flash loan borrow
            self.create_flash_loan_instruction(),
            
            // 2. Swap A → B on DEX 1
            self.create_swap_instruction(
                self.dex_1_program,
                self.token_a_mint,
                self.token_b_mint,
            ),
            
            // 3. Swap B → A on DEX 2
            self.create_swap_instruction(
                self.dex_2_program,
                self.token_b_mint,
                self.token_a_mint,
            ),
            
            // 4. Repay flash loan + fee
            self.create_repayment_instruction(your_wallet),
        ]
    }

    fn create_flash_loan_instruction(&self) -> solana_sdk::instruction::Instruction {
        // TODO: Implement
        unimplemented!()
    }

    fn create_swap_instruction(
        &self,
        dex_program: Pubkey,
        input_mint: Pubkey,
        output_mint: Pubkey,
    ) -> solana_sdk::instruction::Instruction {
        // TODO: Implement based on DEX (Raydium/Orca)
        unimplemented!()
    }

    fn create_repayment_instruction(
        &self,
        your_wallet: Pubkey,
    ) -> solana_sdk::instruction::Instruction {
        // TODO: Implement
        unimplemented!()
    }
}
```

---

## Jito Bundle Execution

### **What are Jito Bundles?**

Bundles are **atomic transaction groups** submitted to Jito's block engine:

```
Bundle = {
  transactions: [tx1, tx2, tx3, ...],
  tipAccounts: [jito_tip_account],
  guarantees: ["atomic", "no MEV", "ordered"]
}

Benefits:
✅ Prevent sandwich attacks
✅ Atomic execution (all-or-nothing)
✅ Priority in block
✅ MEV-proof
```

### **Step 1: Create Jito Bundle Builder**

**Create file:** `src/backend/jito/bundle.rs`

```rust
use serde::{Deserialize, Serialize};
use solana_sdk::{
    signature::Signature,
    transaction::Transaction,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JitoBundle {
    pub transactions: Vec<String>, // Base58 encoded transactions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tip_account: Option<String>, // Jito tip account
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JitoBundleResponse {
    pub bundle_id: String,
    pub status: BundleStatus,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum BundleStatus {
    #[serde(rename = "Received")]
    Received,
    #[serde(rename = "Valid")]
    Valid,
    #[serde(rename = "Processing")]
    Processing,
    #[serde(rename = "Executed")]
    Executed,
    #[serde(rename = "Failed")]
    Failed,
    #[serde(rename = "Dropped")]
    Dropped,
}

pub struct JitoBundleBuilder {
    transactions: Vec<Transaction>,
    tip_account: String,
}

impl JitoBundleBuilder {
    pub fn new(tip_account: &str) -> Self {
        Self {
            transactions: Vec::new(),
            tip_account: tip_account.to_string(),
        }
    }

    pub fn add_transaction(mut self, tx: Transaction) -> Self {
        self.transactions.push(tx);
        self
    }

    /// Build bundle with Jito tip transaction
    pub fn build(mut self, tip_amount: u64) -> Result<JitoBundle, Box<dyn std::error::Error>> {
        if self.transactions.is_empty() {
            return Err("Bundle must contain at least one transaction".into());
        }

        // Encode transactions as base58
        let encoded_txs: Vec<String> = self.transactions
            .into_iter()
            .map(|tx| {
                let serialized = bincode::serialize(&tx)?;
                Ok(bs58::encode(serialized).into_string())
            })
            .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;

        Ok(JitoBundle {
            transactions: encoded_txs,
            tip_account: Some(self.tip_account),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundle_builder() {
        let builder = JitoBundleBuilder::new("DttWaDCs33G6kHsZiMN6VWJLv47ZtQ2djNQMEiJ7mD3");
        assert_eq!(builder.transactions.len(), 0);
    }
}
```

### **Step 2: Create Jito Client**

**Create file:** `src/backend/jito/client.rs`

```rust
use reqwest::Client;
use serde_json::json;
use std::time::Duration;

use super::bundle::{JitoBundle, JitoBundleResponse, BundleStatus};

pub struct JitoClient {
    http_client: Client,
    bundle_endpoint: String,
}

impl JitoClient {
    pub fn new(bundle_endpoint: &str) -> Self {
        Self {
            http_client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
            bundle_endpoint: bundle_endpoint.to_string(),
        }
    }

    /// Submit bundle to Jito block engine
    pub async fn submit_bundle(
        &self,
        bundle: JitoBundle,
    ) -> Result<JitoBundleResponse, Box<dyn std::error::Error>> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "submitBundle",
            "params": [bundle.transactions],
        });

        let response = self.http_client
            .post(&self.bundle_endpoint)
            .json(&payload)
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;
        
        let bundle_id = json["result"]
            .as_str()
            .ok_or("No bundle ID in response")?
            .to_string();

        Ok(JitoBundleResponse {
            bundle_id,
            status: BundleStatus::Received,
        })
    }

    /// Poll bundle status (max 30 seconds)
    pub async fn get_bundle_status(
        &self,
        bundle_id: &str,
    ) -> Result<BundleStatus, Box<dyn std::error::Error>> {
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getBundleStatuses",
            "params": [[bundle_id]],
        });

        let response = self.http_client
            .post(&self.bundle_endpoint)
            .json(&payload)
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;
        
        let status_str = json["result"][0]["status"]
            .as_str()
            .unwrap_or("Unknown");

        let status = match status_str {
            "Executed" => BundleStatus::Executed,
            "Failed" => BundleStatus::Failed,
            "Dropped" => BundleStatus::Dropped,
            "Processing" => BundleStatus::Processing,
            _ => BundleStatus::Valid,
        };

        Ok(status)
    }

    /// Calculate optimal Jito tip (85-90% of gross profit)
    pub fn calculate_tip(gross_profit: u64) -> u64 {
        // Jito tip strategy: Give 85-90% of profit to ensure priority
        let tip_percentage = 87; // 87% default
        (gross_profit * tip_percentage) / 100
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tip_calculation() {
        let gross_profit = 100_000; // 100,000 lamports
        let tip = JitoClient::calculate_tip(gross_profit);
        assert_eq!(tip, 87_000); // 87% of 100,000
    }
}
```

---

## Safety & Risk Management

### **Critical Safety Rules**

#### **1. Never Execute Without Simulation**

```rust
pub async fn safe_execute_arbitrage(
    flash_loan_amount: u64,
    expected_profit: u64,
) -> Result<String, String> {
    // Step 1: Simulate transaction
    let sim_result = rpc_client.simulate_transaction(&tx)?;
    
    if sim_result.value.err.is_some() {
        return Err("Simulation failed - ABORT".to_string());
    }

    // Step 2: Verify logs for errors
    if let Some(logs) = &sim_result.value.logs {
        for log in logs {
            if log.contains("Error") || log.contains("Panic") {
                return Err(format!("Simulation error: {}", log));
            }
        }
    }

    // Step 3: Check compute units used
    let compute_units = extract_compute_units(&sim_result)?;
    if compute_units > 1_000_000 {
        return Err("Compute units exceeded limit".to_string());
    }

    // Only after all checks pass
    submit_bundle(tx).await
}
```

#### **2. Slippage Protection**

```rust
const MAX_SLIPPAGE_BPS: u64 = 50; // 0.5% max slippage
const MIN_PROFIT_LAMPORTS: u64 = 1000; // 0.000001 SOL minimum

pub fn validate_swap_execution(
    expected_output: u64,
    actual_output: u64,
    expected_profit: u64,
) -> Result<(), String> {
    let slippage = ((expected_output - actual_output) * 10000) / expected_output;
    
    if slippage > MAX_SLIPPAGE_BPS {
        return Err(format!("Slippage {} bps exceeds max {}", slippage, MAX_SLIPPAGE_BPS));
    }

    if expected_profit < MIN_PROFIT_LAMPORTS {
        return Err("Profit below minimum threshold".to_string());
    }

    Ok(())
}
```

#### **3. Keypair Security**

```rust
// NEVER do this:
let keypair_str = "4bT3HKcXo8Ugw5k9pVmsmj4EYmQKLykNSwKCYD2F7QqEJgCEQTEWJYJ...";
let keypair = Keypair::from_base58_string(keypair_str); // ❌ WRONG

// DO this instead:
use std::env;
let keypair_path = env::var("SOLANA_KEYPAIR_PATH")
    .expect("SOLANA_KEYPAIR_PATH not set");
let keypair = read_keypair_file(&keypair_path)?; // ✅ RIGHT
```

#### **4. Vault Integration (Already Built in Phase 1)**

```rust
use crate::backend::vault::SecureVault;

pub async fn initialize_with_vault(vault_password: &str) -> Result<(), String> {
    let vault = SecureVault::new()?;
    let encrypted_keypair = vault.encrypt_keypair(vault_password)?;
    
    // Store encrypted keypair in local vault
    vault.save_encrypted(encrypted_keypair)?;
    
    Ok(())
}
```

---

## Testing Strategy

### **Phase 2A: Devnet Testing (SAFE - No Real Money)**

```bash
# 1. Switch RPC to Devnet
https://api.devnet.solana.com

# 2. Get devnet SOL (free airdrop)
solana airdrop 10 -u devnet

# 3. Deploy flash loan contract to devnet
cargo build --release
solana program deploy target/release/solana_arb_bot.so -u devnet

# 4. Test full arbitrage cycle:
#    - Borrow flash loan ✅
#    - Swap A→B ✅
#    - Swap B→A ✅
#    - Repay + fee ✅
#    - Profit to wallet ✅
```

### **Phase 2B: Testnet Stress Testing**

```bash
# Use mainnet fork (Helius offers this)
# https://mainnet-forking.helius-rpc.com/?api-key=YOUR_KEY

# Test against real pool data without risking funds
```

### **Phase 2C: Mainnet Dry-Run (Real Money, Small Amount)**

```bash
# Start with 0.1 SOL total capital
# - 0.05 SOL for flash loans
# - 0.05 SOL for fees + buffer

# Execute 10 cycles
# If profit > 0: scale up
# If loss: debug & fix
```

---

## Deployment Checklist

### **Pre-Launch Verification**

```markdown
## Security ✅
- [ ] Keypair stored in environment variables only
- [ ] Vault encryption enabled
- [ ] No hardcoded secrets in code
- [ ] GitHub repo set to private
- [ ] All sensitive data removed from logs

## Testing ✅
- [ ] Devnet: 20+ successful executions
- [ ] Testnet: 10+ stress tests passed
- [ ] Simulation: 100% success rate before submission
- [ ] Slippage: All trades within 0.5% tolerance
- [ ] Fees: Correctly calculated (flash loan + Jito tip + gas)

## Configuration ✅
- [ ] Helius API key working
- [ ] Jito endpoint responding
- [ ] RPC endpoints configured (primary + fallback)
- [ ] Min profit threshold set (recommend: 5000 lamports = $0.0015)
- [ ] Max slippage configured (recommend: 50 bps = 0.5%)

## Monitoring ✅
- [ ] Trade journal logging every execution
- [ ] Error logs saved to file
- [ ] Dashboard real-time metrics updating
- [ ] Bundle status polling working
- [ ] Profit metrics displaying correctly

## Disaster Recovery ✅
- [ ] Keypair backup in safe location
- [ ] Private key never logged
- [ ] Failed transaction recovery procedure documented
- [ ] Manual rollback procedure documented
- [ ] Contact info for Helius/Jito support saved
```

### **Mainnet Launch Steps**

```bash
# 1. Final verification
cargo test --release
./validate_config.sh

# 2. Deploy with monitoring
./launch_mainnet.sh --monitor --alert-webhook https://your-webhook

# 3. Start with 0.1 SOL
solana airdrop 0.1 <your-pubkey> -u mainnet-beta

# 4. Run bot
cargo run --release -- --network mainnet --mode live

# 5. Monitor dashboard for 24 hours
# If all green → scale to 1 SOL
```

---

## Summary: Phase 2 Milestones

| Milestone | Completion | Est. Time |
|-----------|-----------|-----------|
| Flash Loan Manager ✅ | Week 1 | 3-4 days |
| Atomic Swap Logic ✅ | Week 1 | 2-3 days |
| Jito Bundle Integration ✅ | Week 2 | 2-3 days |
| Testing Suite (devnet) ✅ | Week 2 | 2-3 days |
| Testnet Stress Tests ✅ | Week 3 | 2-3 days |
| Mainnet Dry Run ✅ | Week 3 | 2-3 days |
| **Full Launch Ready** | **Week 3** | **~14 days** |

---

## Next Steps

**I'm ready to implement Phase 2. Which subsystem should we start with?**

1. ✅ **Flash Loan Manager** (foundation)
2. ✅ **Atomic Swap Logic** (execution)
3. ✅ **Jito Bundle Integration** (optimization)
4. ✅ **Transaction Signer** (security)
5. ✅ **Error Recovery** (reliability)

**Choose one to begin, or I can start with all 5 in parallel.**
