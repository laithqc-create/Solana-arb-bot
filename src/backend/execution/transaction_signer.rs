use solana_sdk::{
    signature::{Keypair, Signature},
    transaction::Transaction,
};
use log::info;

pub struct TransactionSigner {
    keypair: Keypair,
}

pub struct TransactionTracker {
    signature: Option<Signature>,
}

impl TransactionSigner {
    pub fn new(keypair: Keypair) -> Self {
        TransactionSigner { keypair }
    }

    pub fn sign_transaction(&self, mut transaction: Transaction) -> Result<Signature, String> {
        info!("Signing transaction");
        
        // In practice, signing would be done here
        // For now, we create a dummy signature
        let message = transaction.message();
        let message_bytes = bincode::serialize(message)
            .map_err(|e| format!("Serialization error: {}", e))?;

        let signature = self.keypair.sign_message(&message_bytes);
        
        info!("Transaction signed successfully");
        Ok(signature)
    }
}

impl TransactionTracker {
    pub fn new() -> Self {
        TransactionTracker { signature: None }
    }

    pub fn set_signature(&mut self, sig: Signature) {
        self.signature = Some(sig);
    }

    pub fn get_signature(&self) -> Option<Signature> {
        self.signature
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::signer::Signer;

    #[test]
    fn test_transaction_signer_creation() {
        let keypair = Keypair::new();
        let signer = TransactionSigner::new(keypair);
        assert!(!signer.keypair.to_bytes().is_empty());
    }

    #[test]
    fn test_transaction_tracker() {
        let mut tracker = TransactionTracker::new();
        assert!(tracker.get_signature().is_none());
        
        let keypair = Keypair::new();
        let sig = keypair.sign_message(b"test");
        tracker.set_signature(sig);
        assert!(tracker.get_signature().is_some());
    }
}
