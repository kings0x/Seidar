//! Backend pool management.
//!
//! # Responsibilities
//! - Manage collections of backends grouped by name
//! - Apply load balancing algorithms to select backends
//! - Provide connection guards for tracking

use std::collections::HashMap;
use std::sync::Arc;
use crate::config::BackendConfig;
use crate::load_balancer::{
    LoadBalancer,
    backend::{Backend, BackendConnectionGuard},
    round_robin::RoundRobin,
};

/// Manages backend pools and load balancing.
#[derive(Debug)]
pub struct BackendManager {
    /// Map of backend_group name -> (Backends, LoadBalancerAlgo).
    groups: HashMap<String, (Vec<Arc<Backend>>, Box<dyn LoadBalancer>)>,
}

impl BackendManager {
    /// Create a new backend manager from configuration.
    pub fn new(configs: Vec<BackendConfig>) -> Self {
        let mut groups: HashMap<String, Vec<Arc<Backend>>> = HashMap::new();

        // 1. Group backends by group name
        for config in configs {
            if let Ok(addr) = config.address.parse() {
                // Phase 3: Pass max_connections
                let backend = Arc::new(Backend::new(addr, config.max_connections));
                groups.entry(config.group.clone()).or_default().push(backend);
            } else {
                tracing::warn!("Invalid backend address: {}", config.address);
            }
        }

        // 2. Create LoadBalancers for each group
        let mut managed_groups = HashMap::new();
        for (name, backends) in groups {
            // Default to RoundRobin for Phase 3
            let lb: Box<dyn LoadBalancer> = Box::new(RoundRobin::new());
            managed_groups.insert(name, (backends, lb));
        }

        Self {
            groups: managed_groups,
        }
    }

    /// Select a backend for the given group.
    /// Returns a guard that decrements the connection count on drop.
    pub fn get(&self, group_name: &str) -> Option<BackendConnectionGuard> {
        if let Some((backends, lb)) = self.groups.get(group_name) {
            if let Some(backend) = lb.next_server(backends) {
                // Phase 3: Enforce limits
                return backend.try_create_guard();
            }
        }
        None
    }

    /// Return a list of all backends (for health checking).
    pub fn all_backends(&self) -> Vec<Arc<Backend>> {
        self.groups.values()
            .flat_map(|(backends, _)| backends.iter())
            .cloned()
            .collect()
    }
}
