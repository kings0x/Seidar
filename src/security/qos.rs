//! Connection tracking and QoS enforcement for long-lived connections.

use std::collections::HashMap;
use std::sync::Mutex;
use alloy::primitives::Address;
use crate::config::QosConfig;

/// State for tracking active long-lived connections.
pub struct ConnectionTracker {
    /// active connections per user: address -> count
    counts: Mutex<HashMap<Address, usize>>,
    _config: QosConfig,
}

impl ConnectionTracker {
    pub fn new(config: QosConfig) -> Self {
        Self {
            counts: Mutex::new(HashMap::new()),
            _config: config,
        }
    }

    /// Try to increment connection count for a user.
    /// Returns true if allowed, false if limit reached.
    pub fn try_increment(&self, address: Address, tier_id: u8) -> bool {
        let limit = match tier_id {
            1 => self._config.tier_1_max_conns,
            2 => self._config.tier_2_max_conns,
            3 => self._config.tier_3_max_conns,
            _ => self._config.tier_1_max_conns, // Default to lowest
        };

        let mut counts = self.counts.lock().expect("connection tracker mutex poisoned");
        let current = counts.entry(address).or_insert(0);
        
        if *current < limit {
            *current += 1;
            true
        } else {
            false
        }
    }

    /// Decrement connection count for a user.
    pub fn decrement(&self, address: Address) {
        let mut counts = self.counts.lock().expect("connection tracker mutex poisoned");
        if let Some(count) = counts.get_mut(&address) {
            if *count > 0 {
                *count -= 1;
            }
        }
    }
}
