use solana_sdk::{
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};
use log::info;

pub struct TransactionSigner {
    keypair: Keypair,
}

pub struct TransactionTracker {
    pub signature: Option<Signature>,
    pub confirmed: bool,
    pub finalized: bool,
}

impl TransactionSigner {
    pub fn new(keypair: Keypair) -> Self {
        TransactionSigner { keypair }
    }

    pub fn sign_transaction(&self, transaction: Transaction) -> Result<Signature, String> {
        info!("Signing transaction");
        
        // Sign using the keypair's sign method
        let message_bytes = transaction.message_data();
        let signature = self.keypair.sign_message(&message_bytes);
        
        info!("Transaction signed successfully");
        Ok(signature)
    }
}

impl TransactionTracker {
    pub fn new() -> Self {
        TransactionTracker { 
            signature: None,
            confirmed: false,
            finalized: false,
        }
    }

    pub fn set_signature(&mut self, sig: Signature) {
        self.signature = Some(sig);
    }

    pub fn get_signature(&self) -> Option<Signature> {
        self.signature
    }

    pub fn mark_confirmed(&mut self) {
        self.confirmed = true;
    }

    pub fn mark_finalized(&mut self) {
        self.finalized = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_tracker() {
        let mut tracker = TransactionTracker::new();
        assert!(tracker.get_signature().is_none());
        tracker.mark_confirmed();
        assert!(tracker.confirmed);
    }
}
