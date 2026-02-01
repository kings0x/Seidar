//! Payment monitoring types.

use alloy::primitives::{Address, U256};
use serde::{Deserialize, Serialize};

/// Represents a detected payment event on the blockchain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentEvent {
    /// The transaction hash.
    pub tx_hash: String,
    /// block number where event occurred.
    pub block_number: u64,
    /// User who made the payment.
    pub user: Address,
    /// Amount paid.
    pub amount: U256,
    /// Tier ID purchased.
    pub tier_id: u8,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_payment_event_serde() {
        let event = PaymentEvent {
            tx_hash: "0x123".to_string(),
            block_number: 100,
            user: Address::ZERO,
            amount: U256::from(1000),
            tier_id: 1,
        };
        let json = serde_json::to_string(&event).unwrap();
        let decoded: PaymentEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.amount, U256::from(1000));
        assert_eq!(decoded.user, Address::ZERO);
    }
}
