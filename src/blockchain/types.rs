//! Chain-specific types and error definitions.

use thiserror::Error;

// Re-export BlockchainConfig from config module to avoid duplication
pub use crate::config::schema::BlockchainConfig;

/// Chain ID type for strong typing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChainId(pub u64);

impl From<u64> for ChainId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

impl From<ChainId> for u64 {
    fn from(id: ChainId) -> Self {
        id.0
    }
}

/// Errors that can occur during blockchain operations.
#[derive(Debug, Error)]
pub enum BlockchainError {
    /// RPC connection or request failed.
    #[error("RPC error: {0}")]
    Rpc(String),

    /// RPC request timed out.
    #[error("RPC timeout after {0} seconds")]
    Timeout(u64),

    /// Transaction was not confirmed within expected time.
    #[error("Transaction not confirmed after {0} blocks")]
    ConfirmationTimeout(u32),

    /// Transaction was reverted on-chain.
    #[error("Transaction reverted: {0}")]
    Reverted(String),

    /// Invalid private key format or derivation error.
    #[error("Wallet error: {0}")]
    Wallet(String),

    /// Gas price exceeded maximum allowed.
    #[error("Gas price {current_gwei} gwei exceeds maximum {max_gwei} gwei")]
    GasPriceTooHigh { current_gwei: u64, max_gwei: u64 },

    /// Nonce management error.
    #[error("Nonce error: {0}")]
    Nonce(String),

    /// Chain configuration mismatch.
    #[error("Chain ID mismatch: expected {expected}, got {actual}")]
    ChainMismatch { expected: u64, actual: u64 },

    /// Blockchain client not initialized or disabled.
    #[error("Blockchain not available: {0}")]
    NotAvailable(String),
}

/// Result type for blockchain operations.
pub type BlockchainResult<T> = Result<T, BlockchainError>;

/// Transaction confirmation status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmationStatus {
    /// Transaction is pending in mempool.
    Pending,
    /// Transaction has been mined but not enough confirmations.
    Confirming { current: u32, required: u32 },
    /// Transaction is confirmed with required block depth.
    Confirmed { block_number: u64 },
    /// Transaction failed or was dropped.
    Failed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_id_conversion() {
        let chain_id = ChainId::from(1u64);
        assert_eq!(chain_id.0, 1);
        assert_eq!(u64::from(chain_id), 1);
    }

    #[test]
    fn test_default_config() {
        let config = BlockchainConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.rpc_timeout_secs, 10);
        assert_eq!(config.confirmation_blocks, 3);
    }

    #[test]
    fn test_error_display() {
        let err = BlockchainError::Timeout(10);
        assert_eq!(err.to_string(), "RPC timeout after 10 seconds");

        let err = BlockchainError::GasPriceTooHigh {
            current_gwei: 600,
            max_gwei: 500,
        };
        assert!(err.to_string().contains("600"));
    }
}
