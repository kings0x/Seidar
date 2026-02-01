//! Wallet management and transaction signing.
//!
//! # Security
//! - Private keys are loaded ONLY from environment variables
//! - Keys are never logged or serialized
//! - Uses secure memory handling where possible

use alloy::primitives::{Address, B256};
use alloy::signers::local::PrivateKeySigner;
use alloy::signers::Signer;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::blockchain::types::{BlockchainError, BlockchainResult};

/// Environment variable name for the private key.
pub const PRIVATE_KEY_ENV_VAR: &str = "PROXY_BLOCKCHAIN_PRIVATE_KEY";

/// Wallet for transaction signing with nonce management.
#[derive(Debug)]
pub struct Wallet {
    /// The underlying signer (private key).
    signer: PrivateKeySigner,
    /// Current nonce for sequential transactions.
    nonce: Arc<AtomicU64>,
    /// Chain ID for EIP-155 replay protection.
    chain_id: u64,
}

impl Wallet {
    /// Create a wallet from a hex-encoded private key string.
    ///
    /// # Arguments
    /// * `private_key_hex` - Hex string (with or without 0x prefix)
    /// * `chain_id` - Chain ID for transaction signing
    ///
    /// # Security
    /// The private key is parsed and stored securely. It is never logged.
    pub fn from_private_key(private_key_hex: &str, chain_id: u64) -> BlockchainResult<Self> {
        // Strip 0x prefix if present
        let key_hex = private_key_hex.strip_prefix("0x").unwrap_or(private_key_hex);



        let signer: PrivateKeySigner = key_hex
            .parse()
            .map_err(|e| BlockchainError::Wallet(format!("Invalid private key format: {}", e)))?;

        tracing::info!(
            address = %signer.address(),
            chain_id = chain_id,
            "Wallet initialized"
        );

        Ok(Self {
            signer,
            nonce: Arc::new(AtomicU64::new(0)),
            chain_id,
        })
    }

    /// Load wallet from environment variable.
    ///
    /// Reads `PROXY_BLOCKCHAIN_PRIVATE_KEY` from environment.
    pub fn from_env(chain_id: u64) -> BlockchainResult<Self> {
        let private_key = std::env::var(PRIVATE_KEY_ENV_VAR).map_err(|_| {
            BlockchainError::Wallet(format!(
                "Environment variable {} not set",
                PRIVATE_KEY_ENV_VAR
            ))
        })?;

        Self::from_private_key(&private_key, chain_id)
    }

    /// Get the wallet's address.
    pub fn address(&self) -> Address {
        self.signer.address()
    }

    /// Get the chain ID this wallet is configured for.
    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }

    /// Get and increment the nonce atomically.
    ///
    /// This ensures sequential transactions don't collide.
    pub fn get_and_increment_nonce(&self) -> u64 {
        self.nonce.fetch_add(1, Ordering::SeqCst)
    }

    /// Set the nonce to a specific value (e.g., after querying from chain).
    pub fn set_nonce(&self, nonce: u64) {
        self.nonce.store(nonce, Ordering::SeqCst);
    }

    /// Get current nonce without incrementing.
    pub fn current_nonce(&self) -> u64 {
        self.nonce.load(Ordering::SeqCst)
    }

    /// Sign a message hash.
    ///
    /// # Arguments
    /// * `hash` - The 32-byte hash to sign
    ///
    /// # Returns
    /// The signature as bytes
    pub async fn sign_hash(&self, hash: B256) -> BlockchainResult<alloy::signers::Signature> {
        self.signer
            .sign_hash(&hash)
            .await
            .map_err(|e| BlockchainError::Wallet(format!("Signing failed: {}", e)))
    }

    /// Sign arbitrary message bytes (with Ethereum prefix).
    pub async fn sign_message(&self, message: &[u8]) -> BlockchainResult<alloy::signers::Signature> {
        self.signer
            .sign_message(message)
            .await
            .map_err(|e| BlockchainError::Wallet(format!("Message signing failed: {}", e)))
    }
}

impl Clone for Wallet {
    fn clone(&self) -> Self {
        Self {
            signer: self.signer.clone(),
            nonce: self.nonce.clone(),
            chain_id: self.chain_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Well-known test private key (Anvil's first account)
    const TEST_PRIVATE_KEY: &str = "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";

    #[test]
    fn test_wallet_from_private_key() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, 1).unwrap();
        // This is the corresponding address for the test key
        assert_eq!(
            wallet.address().to_string().to_lowercase(),
            "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266"
        );
    }

    #[test]
    fn test_wallet_with_0x_prefix() {
        let wallet = Wallet::from_private_key(&format!("0x{}", TEST_PRIVATE_KEY), 1).unwrap();
        assert_eq!(
            wallet.address().to_string().to_lowercase(),
            "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266"
        );
    }

    #[test]
    fn test_nonce_management() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, 1).unwrap();

        assert_eq!(wallet.current_nonce(), 0);
        assert_eq!(wallet.get_and_increment_nonce(), 0);
        assert_eq!(wallet.get_and_increment_nonce(), 1);
        assert_eq!(wallet.current_nonce(), 2);

        wallet.set_nonce(100);
        assert_eq!(wallet.current_nonce(), 100);
    }

    #[test]
    fn test_invalid_private_key() {
        let result = Wallet::from_private_key("invalid_key", 1);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid private key"));
    }

    #[tokio::test]
    async fn test_sign_message() {
        let wallet = Wallet::from_private_key(TEST_PRIVATE_KEY, 1).unwrap();
        let message = b"Hello, World!";
        let signature = wallet.sign_message(message).await.unwrap();
        // Signature should be 65 bytes (r, s, v)
        assert_eq!(signature.as_bytes().len(), 65);
    }
}
