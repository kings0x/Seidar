//! Transaction building, signing, and confirmation monitoring.
//!
//! # Responsibilities
//! - Build transactions with proper gas estimation
//! - Sign and broadcast transactions
//! - Monitor confirmations
//! - Handle retry logic for failed broadcasts

use alloy::network::TransactionBuilder;
use alloy::primitives::{Address, Bytes, TxHash, U256};
use alloy::rpc::types::TransactionRequest;
use std::time::Duration;
use tokio::time::{interval, timeout};

use crate::blockchain::client::BlockchainClient;
use crate::blockchain::types::{BlockchainError, BlockchainResult, ConfirmationStatus};
use crate::blockchain::wallet::Wallet;

/// Transaction builder for common operations.
pub struct TxBuilder {
    client: BlockchainClient,
    wallet: Wallet,
}

impl TxBuilder {
    /// Create a new transaction builder.
    pub fn new(client: BlockchainClient, wallet: Wallet) -> Self {
        Self { client, wallet }
    }

    /// Build a transaction request with gas estimation.
    ///
    /// # Arguments
    /// * `to` - Destination address
    /// * `value` - Amount of native token to send
    /// * `data` - Call data (empty for simple transfers)
    pub async fn build(
        &self,
        to: Address,
        value: U256,
        data: Bytes,
    ) -> BlockchainResult<TransactionRequest> {
        // Get current nonce from chain and sync wallet
        let chain_nonce = self.client.get_transaction_count(self.wallet.address()).await?;
        self.wallet.set_nonce(chain_nonce);

        // Get gas price
        let gas_price = self.client.get_gas_price().await?;
        let gas_price_gwei = gas_price / 1_000_000_000;

        // Check against max gas price
        let config = self.client.config();
        if gas_price_gwei > config.max_gas_price_gwei as u128 {
            return Err(BlockchainError::GasPriceTooHigh {
                current_gwei: gas_price_gwei as u64,
                max_gwei: config.max_gas_price_gwei,
            });
        }

        // Apply multiplier for safety margin
        let adjusted_gas_price =
            (gas_price as f64 * config.gas_price_multiplier) as u128;

        let nonce = self.wallet.get_and_increment_nonce();

        // Calculate gas limit before consuming data
        // Base gas + data cost (16 gas per non-zero byte, simplified)
        let gas_limit = 21000u64 + (data.len() as u64 * 16);

        let tx = TransactionRequest::default()
            .with_to(to)
            .with_value(value)
            .with_input(data)
            .with_nonce(nonce)
            .with_gas_price(adjusted_gas_price)
            .with_chain_id(self.wallet.chain_id())
            .with_gas_limit(gas_limit);

        Ok(tx)
    }

    /// Wait for a transaction to be confirmed.
    ///
    /// # Arguments
    /// * `tx_hash` - Transaction hash to monitor
    /// * `timeout_secs` - Maximum time to wait for confirmation
    pub async fn wait_for_confirmation(
        &self,
        tx_hash: TxHash,
        timeout_secs: u64,
    ) -> BlockchainResult<ConfirmationStatus> {
        let required_confirmations = self.client.confirmation_blocks();
        let timeout_duration = Duration::from_secs(timeout_secs);
        let poll_interval = Duration::from_secs(2);

        let result = timeout(timeout_duration, async {
            let mut ticker = interval(poll_interval);

            loop {
                ticker.tick().await;

                // Get the receipt
                let receipt = match self.client.get_transaction_receipt(tx_hash).await? {
                    Some(r) => r,
                    None => {
                        tracing::debug!(tx_hash = %tx_hash, "Transaction pending");
                        continue;
                    }
                };

                // Check if transaction succeeded
                if !receipt.status() {
                    return Ok(ConfirmationStatus::Failed(
                        "Transaction reverted".to_string(),
                    ));
                }

                // Get current block number
                let current_block = self.client.get_block_number().await?;
                let tx_block = receipt.block_number.unwrap_or(current_block);
                let confirmations = current_block.saturating_sub(tx_block) as u32;

                if confirmations >= required_confirmations {
                    return Ok(ConfirmationStatus::Confirmed {
                        block_number: tx_block,
                    });
                }

                tracing::debug!(
                    tx_hash = %tx_hash,
                    confirmations = confirmations,
                    required = required_confirmations,
                    "Waiting for confirmations"
                );
            }
        })
        .await;

        match result {
            Ok(status) => status,
            Err(_) => Err(BlockchainError::ConfirmationTimeout(required_confirmations)),
        }
    }

    /// Get the wallet address.
    pub fn address(&self) -> Address {
        self.wallet.address()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confirmation_status() {
        let status = ConfirmationStatus::Confirming {
            current: 2,
            required: 3,
        };
        assert!(matches!(status, ConfirmationStatus::Confirming { .. }));

        let status = ConfirmationStatus::Confirmed { block_number: 100 };
        assert!(matches!(status, ConfirmationStatus::Confirmed { .. }));
    }
}
