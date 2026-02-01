//! Quote generation system types.

use alloy::primitives::{Address, B256};
use alloy::signers::Signature;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Supported service types for quoting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceType {
    /// Standard subscription tier.
    SubscriptionTier1,
    /// Premium subscription tier.
    SubscriptionTier2,
    /// Single proof generation request.
    ProofGeneration,
}

/// Request payload for generating a quote.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteRequest {
    /// The type of service requested.
    pub service_type: ServiceType,
    /// The user's wallet address (who the quote is for).
    pub user_address: Address,
    /// Optional duration in seconds (for subscriptions).
    pub duration_seconds: Option<u64>,
}

/// A pricing quote for a service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    /// Unique identifier for the quote.
    pub id: Uuid,
    /// The service being quoted.
    pub service_type: ServiceType,
    /// Amount to be paid (in Wei usually, or native token units).
    pub amount: String, // String to handle large numbers safely in JSON
    /// Currency symbol (e.g., "ETH", "LIT").
    pub currency: String,
    /// Unix timestamp when this quote expires.
    pub expiry: u64,
    /// Random nonce to prevent replay attacks.
    pub nonce: u64,
    /// Address who requested the quote.
    pub user_address: Address,
}

/// A quote signed by the service provider (Reverse Proxy).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedQuote {
    /// The original quote data.
    pub quote: Quote,
    /// Cryptographic signature of the quote hash.
    pub signature: Signature,
    /// The hash that was signed.
    pub hash: B256,
}
