//! Blockchain RPC client with timeout and error handling.
//!
//! # Responsibilities
//! - Connect to JSON-RPC endpoint
//! - Query chain state (block number, balances, receipts)
//! - Handle timeouts and network errors gracefully
//! - Provide health check for blockchain connectivity

use alloy::primitives::{Address, TxHash, U256};
use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::types::TransactionReceipt;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

use crate::blockchain::types::{BlockchainConfig, BlockchainError, BlockchainResult, ChainId};
use crate::observability::metrics;

/// Blockchain RPC client wrapper with failover support.
#[derive(Clone)]
pub struct BlockchainClient {
    /// List of providers (primary + failovers).
    providers: Vec<Arc<dyn Provider + Send + Sync>>,
    /// Configuration.
    config: BlockchainConfig,
    /// Request timeout duration.
    timeout_duration: Duration,
}

impl BlockchainClient {
    /// Create a new blockchain client.
    ///
    /// # Arguments
    /// * `config` - Blockchain configuration
    ///
    /// # Returns
    /// A new client or error if connection fails
    pub async fn new(config: BlockchainConfig) -> BlockchainResult<Self> {
        let timeout_duration = Duration::from_secs(config.rpc_timeout_secs);
        let mut providers = Vec::new();

        // 1. Add primary provider
        let primary_url: url::Url = config.rpc_url.parse().map_err(|e| {
            BlockchainError::Rpc(format!("Invalid RPC URL '{}': {}", config.rpc_url, e))
        })?;
        providers.push(Arc::new(ProviderBuilder::new().connect_http(primary_url)) as Arc<dyn Provider + Send + Sync>);

        // 2. Add failover providers
        for url_str in &config.failover_urls {
            if let Ok(url) = url_str.parse() {
                providers.push(Arc::new(ProviderBuilder::new().connect_http(url)) as Arc<dyn Provider + Send + Sync>);
            } else {
                tracing::warn!(url = %url_str, "Ignoring invalid failover RPC URL");
            }
        }

        let client = Self {
            providers,
            config: config.clone(),
            timeout_duration,
        };

        // Verify chain ID matches configuration
        match client.verify_chain_id().await {
            Ok(()) => {
                tracing::info!(
                    rpc_url = %config.rpc_url,
                    chain_id = config.chain_id,
                    "Blockchain client initialized"
                );
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Blockchain client initialized but chain verification failed"
                );
                // Don't fail initialization - allow graceful degradation
            }
        }

        Ok(client)
    }

    /// Verify the connected chain ID matches configuration.
    pub async fn verify_chain_id(&self) -> BlockchainResult<()> {
        let chain_id = self.get_chain_id().await?;
        if chain_id.0 != self.config.chain_id {
            return Err(BlockchainError::ChainMismatch {
                expected: self.config.chain_id,
                actual: chain_id.0,
            });
        }
        Ok(())
    }

    /// Get the chain ID from the RPC.
    pub async fn get_chain_id(&self) -> BlockchainResult<ChainId> {
        for (i, provider) in self.providers.iter().enumerate() {
            let fut = provider.get_chain_id();
            match timeout(self.timeout_duration, fut).await {
                Ok(Ok(result)) => return Ok(ChainId(result)),
                Ok(Err(e)) => {
                    tracing::warn!(provider_idx = i, error = %e, "RPC error, trying next provider");
                }
                Err(_) => {
                    tracing::warn!(provider_idx = i, "RPC timeout, trying next provider");
                }
            }
        }
        Err(BlockchainError::Rpc("All RPC providers failed".to_string()))
    }

    /// Get the latest block number.
    pub async fn get_block_number(&self) -> BlockchainResult<u64> {
        for (i, provider) in self.providers.iter().enumerate() {
            let fut = provider.get_block_number();
            match timeout(self.timeout_duration, fut).await {
                Ok(Ok(result)) => return Ok(result),
                Ok(Err(e)) => tracing::warn!(provider_idx = i, error = %e, "RPC error"),
                Err(_) => tracing::warn!(provider_idx = i, "RPC timeout"),
            }
        }
        Err(BlockchainError::Rpc("All providers failed to get block number".to_string()))
    }

    /// Get the balance of an address.
    pub async fn get_balance(&self, address: Address) -> BlockchainResult<U256> {
        for (i, provider) in self.providers.iter().enumerate() {
            let fut = provider.get_balance(address);
            match timeout(self.timeout_duration, fut).await {
                Ok(Ok(result)) => return Ok(result),
                Ok(Err(e)) => tracing::warn!(provider_idx = i, error = %e, "RPC error"),
                Err(_) => tracing::warn!(provider_idx = i, "RPC timeout"),
            }
        }
        Err(BlockchainError::Rpc("All providers failed to get balance".to_string()))
    }

    /// Get the transaction count (nonce) for an address.
    pub async fn get_transaction_count(&self, address: Address) -> BlockchainResult<u64> {
        for (i, provider) in self.providers.iter().enumerate() {
            let fut = provider.get_transaction_count(address);
            match timeout(self.timeout_duration, fut).await {
                Ok(Ok(result)) => return Ok(result),
                Ok(Err(e)) => tracing::warn!(provider_idx = i, error = %e, "RPC error"),
                Err(_) => tracing::warn!(provider_idx = i, "RPC timeout"),
            }
        }
        Err(BlockchainError::Rpc("All providers failed to get transaction count".to_string()))
    }

    /// Get a transaction receipt by hash.
    pub async fn get_transaction_receipt(
        &self,
        tx_hash: TxHash,
    ) -> BlockchainResult<Option<TransactionReceipt>> {
        for (i, provider) in self.providers.iter().enumerate() {
            let fut = provider.get_transaction_receipt(tx_hash);
            match timeout(self.timeout_duration, fut).await {
                Ok(Ok(result)) => return Ok(result),
                Ok(Err(e)) => tracing::warn!(provider_idx = i, error = %e, "RPC error"),
                Err(_) => tracing::warn!(provider_idx = i, "RPC timeout"),
            }
        }
        Err(BlockchainError::Rpc("All providers failed to get receipt".to_string()))
    }

    /// Get current gas price in wei.
    pub async fn get_gas_price(&self) -> BlockchainResult<u128> {
        for (i, provider) in self.providers.iter().enumerate() {
            let fut = provider.get_gas_price();
            match timeout(self.timeout_duration, fut).await {
                Ok(Ok(result)) => return Ok(result),
                Ok(Err(e)) => tracing::warn!(provider_idx = i, error = %e, "RPC error"),
                Err(_) => tracing::warn!(provider_idx = i, "RPC timeout"),
            }
        }
        Err(BlockchainError::Rpc("All providers failed to get gas price".to_string()))
    }

    /// Check if the blockchain is reachable and healthy.
    ///
    /// Returns true if we can query the block number.
    pub async fn is_healthy(&self) -> bool {
        let healthy = self.get_block_number().await.is_ok();
        // Record health metric
        metrics::record_backend_health("blockchain_rpc", healthy);
        healthy
    }

    /// Get the underlying primary provider.
    pub fn provider(&self) -> &(dyn Provider + Send + Sync) {
        self.providers[0].as_ref()
    }

    /// Get the configuration.
    pub fn config(&self) -> &BlockchainConfig {
        &self.config
    }

    /// Get the number of confirmation blocks required.
    pub fn confirmation_blocks(&self) -> u32 {
        self.config.confirmation_blocks
    }
}

impl std::fmt::Debug for BlockchainClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlockchainClient")
            .field("rpc_url", &self.config.rpc_url)
            .field("chain_id", &self.config.chain_id)
            .field("timeout_secs", &self.config.rpc_timeout_secs)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> BlockchainConfig {
        BlockchainConfig {
            enabled: true,
            rpc_url: "http://localhost:8545".to_string(),
            failover_urls: Vec::new(),
            chain_id: 31337, // Anvil default
            rpc_timeout_secs: 5,
            confirmation_blocks: 1,
            gas_price_multiplier: 1.0,
            max_gas_price_gwei: 100,
        }
    }

    #[tokio::test]
    async fn test_client_creation() {
        // This test will fail if Anvil is not running, but shouldn't panic
        let config = test_config();
        let result = BlockchainClient::new(config).await;
        // Client creation should succeed even if RPC is unreachable
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rpc_failover() {
        let mut config = test_config();
        // Add a secondary invalid URL
        config.failover_urls.push("http://invalid:8545".to_string());
        
        let client = BlockchainClient::new(config).await.unwrap();
        
        // This should fail because BOTH are invalid (localhost:8545 is empty and invalid:8545 is invalid)
        // But we want to see it iterate.
        let result = client.get_chain_id().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("All RPC providers failed"));
    }
}
