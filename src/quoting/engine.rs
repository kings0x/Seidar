//! Core logic for calculating prices and generating signed quotes.

use alloy::primitives::{keccak256, U256};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::blockchain::types::BlockchainResult;
use crate::blockchain::wallet::Wallet;
use crate::quoting::types::{Quote, QuoteRequest, ServiceType, SignedQuote};

use dashmap::DashMap;
use std::sync::Arc;

/// Engine for generating and signing quotes.
#[derive(Clone)]
pub struct QuoteEngine {
    wallet: Wallet,
    quotes: Arc<DashMap<Uuid, SignedQuote>>,
}

impl QuoteEngine {
    /// Create a new quote engine.
    pub fn new(wallet: Wallet) -> Self {
        Self {
            wallet,
            quotes: Arc::new(DashMap::new()),
        }
    }

    /// Generate a signed quote for a request.
    pub async fn generate_quote(&self, request: QuoteRequest) -> BlockchainResult<SignedQuote> {
        let (amount, currency) = self.calculate_price(&request);
        let expiry = self.calculate_expiry(&request);
        let nonce = fastrand::u64(..);
        let id = Uuid::new_v4();

        let quote = Quote {
            id,
            service_type: request.service_type,
            amount: amount.to_string(),
            currency,
            expiry,
            nonce,
            user_address: request.user_address,
        };

        let signed = self.sign_quote(quote).await?;
        
        // Store quote
        self.quotes.insert(id, signed.clone());

        Ok(signed)
    }

    /// Get a quote by ID.
    pub fn get_quote(&self, id: Uuid) -> Option<SignedQuote> {
        self.quotes.get(&id).map(|r| r.value().clone())
    }

    /// Calculate price based on service type.
    ///
    /// In a real system, this would look up dynamic pricing or query an oracle.
    fn calculate_price(&self, request: &QuoteRequest) -> (U256, String) {
        match request.service_type {
            ServiceType::SubscriptionTier1 => (
                U256::from(10_000_000_000_000_000u64), // 0.01 ETH
                "ETH".to_string(),
            ),
            ServiceType::SubscriptionTier2 => (
                U256::from(50_000_000_000_000_000u64), // 0.05 ETH
                "ETH".to_string(),
            ),
            ServiceType::ProofGeneration => (
                U256::from(1_000_000_000_000_000u64), // 0.001 ETH
                "ETH".to_string(),
            ),
        }
    }

    /// Calculate quote expiration time.
    fn calculate_expiry(&self, _request: &QuoteRequest) -> u64 {
        // Quotes valid for 1 hour by default
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now + 3600
    }

    /// Sign the quote using the wallet.
    async fn sign_quote(&self, quote: Quote) -> BlockchainResult<SignedQuote> {
        // EIP-712 style hashing would be better, but for now simple hash of fields
        // Serialize relevant fields for hashing
        // This is a simplified hashing scheme for demonstration
        let mut data = Vec::new();
        data.extend_from_slice(quote.id.as_bytes());
        data.extend_from_slice(&U256::from_str_radix(&quote.amount, 10).unwrap_or_default().to_be_bytes::<32>());
        data.extend_from_slice(&quote.nonce.to_be_bytes());
        data.extend_from_slice(quote.user_address.as_slice());

        let hash = keccak256(&data);

        // Sign the hash
        let signature = self.wallet.sign_hash(hash).await?;

        Ok(SignedQuote {
            quote,
            signature,
            hash,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::Address;

    fn test_wallet() -> Wallet {
        // Use Anvil's well-known test account #0 for deterministic testing
        // This key is publicly known and should NEVER be used for real funds
        const TEST_KEY: &str = "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
        Wallet::from_private_key(TEST_KEY, 31337).expect("Failed to create test wallet")
    }

    #[tokio::test]
    async fn test_quote_generation() {
        let wallet = test_wallet();
        let engine = QuoteEngine::new(wallet);

        let request = QuoteRequest {
            service_type: ServiceType::SubscriptionTier1,
            user_address: Address::ZERO,
            duration_seconds: None,
        };

        let signed_quote = engine.generate_quote(request.clone()).await.expect("Failed to generate quote");

        assert_eq!(signed_quote.quote.service_type, ServiceType::SubscriptionTier1);
        assert_eq!(signed_quote.quote.currency, "ETH");
        assert!(signed_quote.quote.expiry > 0);
        
        // Verify storage
        let retrieved = engine.get_quote(signed_quote.quote.id).expect("Quote not found");
        assert_eq!(retrieved.quote.id, signed_quote.quote.id);
    }

    #[tokio::test]
    async fn test_price_calculation() {
        let wallet = test_wallet();
        let engine = QuoteEngine::new(wallet);
        
        let request = QuoteRequest {
            service_type: ServiceType::SubscriptionTier2,
            user_address: Address::ZERO,
            duration_seconds: None,
        };
        
        let (price, currency) = engine.calculate_price(&request);
        assert_eq!(price, U256::from(50_000_000_000_000_000u64));
        assert_eq!(currency, "ETH");
    }
}
