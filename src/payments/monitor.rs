//! Payment monitoring service.

use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use alloy::sol;
use alloy::primitives::Address;
use alloy::rpc::types::eth::Filter;
use alloy::sol_types::SolEvent;

use crate::blockchain::client::BlockchainClient;
use crate::config::PaymentConfig;
use crate::payments::cache::SubscriptionCache;
use crate::payments::processor::process_payment;
use crate::payments::types::PaymentEvent;

sol! {
    /// Emitted when a payment is received.
    #[derive(Debug)]
    event PaymentReceived(address indexed user, uint256 amount, uint8 tierId);
    
    /// Emitted when a subscription is created.
    #[derive(Debug)]
    event SubscriptionCreated(address indexed user, uint8 tier, uint256 expiry);
}

/// Service to monitor blockchain for payment events.
pub struct PaymentMonitor {
    client: BlockchainClient,
    config: PaymentConfig,
    contract_address: Address,
    last_block: u64,
    cache: Arc<SubscriptionCache>,
}

impl PaymentMonitor {
    /// Create a new payment monitor.
    pub fn new(
        client: BlockchainClient, 
        config: PaymentConfig,
        cache: Arc<SubscriptionCache>
    ) -> Result<Self, String> {
        let contract_address: Address = config.contract_address.parse()
            .map_err(|e| format!("Invalid contract address: {}", e))?;

        Ok(Self {
            client,
            config,
            contract_address,
            last_block: 0,
            cache,
        })
    }

    /// Run the monitor loop.
    pub async fn run(mut self) {
        if !self.config.enabled {
            tracing::info!("Payment monitor disabled");
            return;
        }

        tracing::info!("Starting payment monitor for contract {}", self.contract_address);

        // Initialize last_block to current block if 0
        if self.last_block == 0 {
            if let Ok(block) = self.client.get_block_number().await {
                self.last_block = block;
                tracing::info!("Initialized payment monitor at block {}", block);
            }
        }

        loop {
            if let Err(e) = self.poll_events().await {
                tracing::error!("Error polling payment events: {}", e);
            }

            sleep(Duration::from_millis(self.config.monitor_interval_ms)).await;
        }
    }

    async fn poll_events(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let current_block = self.client.get_block_number().await?;
        
        // Wait for confirmations
        let target_block = current_block.saturating_sub(self.client.confirmation_blocks() as u64);

        if target_block <= self.last_block {
            return Ok(());
        }

        let filter = Filter::new()
            .address(self.contract_address)
            .from_block(self.last_block + 1)
            .to_block(target_block)
            .event(PaymentReceived::SIGNATURE); // For now filtering specific event

        let logs = self.client.provider().get_logs(&filter).await?;

        for log in logs {
            // Try decoding PaymentReceived
            if let Ok(decoded) = log.log_decode::<PaymentReceived>() {
                let event = decoded.inner;
                let user = event.user;
                let amount = event.amount;
                let tier_id = event.tierId;
                
                let payment_event = PaymentEvent {
                    tx_hash: log.transaction_hash.map(|h| h.to_string()).unwrap_or_default(),
                    block_number: log.block_number.unwrap_or_default(),
                    user,
                    amount,
                    tier_id,
                };

                process_payment(payment_event, &self.cache).await;
            }
        }

        self.last_block = target_block;
        Ok(())
    }
}
