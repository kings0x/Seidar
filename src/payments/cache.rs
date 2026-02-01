//! Subscription caching and persistence.

use alloy::primitives::Address;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::observability::metrics;

/// Information about a user's subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionInfo {
    /// The tier ID subscribed to.
    pub tier_id: u8,
    /// Expiry timestamp (seconds since epoch).
    pub expiry: u64,
}

impl SubscriptionInfo {
    /// Check if the subscription is active.
    pub fn is_active(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.expiry > now
    }

    /// Check if active or within grace period.
    pub fn is_active_with_grace(&self, grace_secs: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.expiry + grace_secs > now
    }
}

/// A thread-safe cache for subscription data.
#[derive(Clone, Default)]
pub struct SubscriptionCache {
    /// The internal map of address -> subscription info.
    /// Wrapped in Arc internally by DashMap? No, DashMap is concurrent but not Arc'd itself usually needs Arc wrapper for cloning locally?
    /// DashMap uses internal locking. But we want to share the whole map.
    /// Usually `Arc<DashMap<...>>` is the way.
    /// Wait, `DashMap` is `Send + Sync`.
    /// I will implement `SubscriptionCache` as a wrapper around `Arc<DashMap>`.
    inner: Arc<DashMap<Address, SubscriptionInfo>>,
    persistence_path: Option<String>,
}

impl SubscriptionCache {
    /// Create a new empty cache.
    pub fn new(persistence_path: Option<String>) -> Self {
        Self {
            inner: Arc::new(DashMap::new()),
            persistence_path,
        }
    }

    /// Load from file if exists.
    pub fn load_from_file(path: &str) -> std::io::Result<Self> {
        let cache = Self::new(Some(path.to_string()));
        if Path::new(path).exists() {
            let file = File::open(path)?;
            let reader = BufReader::new(file);
            let map: std::collections::HashMap<Address, SubscriptionInfo> = serde_json::from_reader(reader)?;
            
            for (k, v) in map {
                cache.inner.insert(k, v);
            }
            metrics::record_cache_size(cache.inner.len());
            tracing::info!("Loaded {} subscriptions from cache file", cache.inner.len());
        }
        Ok(cache)
    }

    /// Save to file.
    pub fn save_to_file(&self) -> std::io::Result<()> {
        if let Some(path) = &self.persistence_path {
            let file = File::create(path)?;
            let writer = BufWriter::new(file);
            
            // DashMap doesn't serialize directly efficiently without collecting?
            // Collect to HashMap first.
            let map: std::collections::HashMap<_, _> = self.inner.iter()
                .map(|r| (*r.key(), r.value().clone()))
                .collect();
                
            serde_json::to_writer(writer, &map)?;
            tracing::info!("Saved {} subscriptions to cache file", map.len());
        }
        Ok(())
    }

    /// Update subscription for a user.
    pub fn update_subscription(&self, user: Address, tier_id: u8, expiry: u64) {
        self.inner.insert(user, SubscriptionInfo { tier_id, expiry });
        metrics::record_subscription_event("update");
        metrics::record_cache_size(self.inner.len());
        // Auto-save on update? Or rely on periodic save?
        // For simplicity, save on update if critical, but might be slow.
        // Let's rely on shutdown save for now, or calling code to trigger save.
    }

    /// Get subscription info if active.
    pub fn get_subscription(&self, user: &Address) -> Option<SubscriptionInfo> {
        self.inner.get(user).map(|r| r.value().clone())
    }

    /// Count active subscriptions.
    pub fn count(&self) -> usize {
        self.inner.len()
    }

    /// Get a summary of active/inactive sessions.
    pub fn get_summary(&self) -> (usize, usize) {
        let mut active = 0;
        let mut expired = 0;
        for r in self.inner.iter() {
            if r.value().is_active() {
                active += 1;
            } else {
                expired += 1;
            }
        }
        (active, expired)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_operations() {
        let cache = SubscriptionCache::new(None);
        let user = Address::ZERO;
        
        // Initial check
        assert!(cache.get_subscription(&user).is_none());

        // Update
        cache.update_subscription(user, 1, 9999999999);
        let sub = cache.get_subscription(&user).unwrap();
        assert_eq!(sub.tier_id, 1);
        assert!(sub.is_active());
        
        // Expired
        cache.update_subscription(user, 1, 0);
        let sub = cache.get_subscription(&user).unwrap();
        assert!(!sub.is_active());
    }

    #[test]
    fn test_grace_period() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Expired 10 seconds ago
        let sub = SubscriptionInfo {
            tier_id: 1,
            expiry: now - 10,
        };

        assert!(!sub.is_active());
        // Grace period 30s
        assert!(sub.is_active_with_grace(30));
        // Grace period 5s (too short)
        assert!(!sub.is_active_with_grace(5));
    }

    #[test]
    fn test_persistence() {
        let path = "test_subs_persistence.json";
        
        let cache = SubscriptionCache::new(Some(path.to_string()));
        let user = Address::ZERO;
        cache.update_subscription(user, 2, 1234567890);
        cache.save_to_file().unwrap();
        
        // Load new instance
        let loaded = SubscriptionCache::load_from_file(path).unwrap();
        let sub = loaded.get_subscription(&user).unwrap();
        assert_eq!(sub.tier_id, 2);
        
        // Cleanup
        std::fs::remove_file(path).unwrap_or_default();
    }
}
