//! Least Connections load balancing strategy.

use std::sync::Arc;
use std::sync::atomic::Ordering;
use crate::load_balancer::{LoadBalancer, backend::Backend};

/// Least connections selector.
/// Selects the backend with the minimum number of active connections.
#[derive(Debug, Default)]
pub struct LeastConnections;

impl LeastConnections {
    pub fn new() -> Self {
        Self::default()
    }
}

impl LoadBalancer for LeastConnections {
    fn next_server(&self, backends: &[Arc<Backend>]) -> Option<Arc<Backend>> {
        if backends.is_empty() {
            return None;
        }

        // Find backend with minimum connections
        // In case of tie, the first one is selected (stability)
        backends
            .iter()
            .min_by_key(|b| b.active_connections.load(Ordering::Relaxed))
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_least_conn() {
        let lb = LeastConnections::new();
        let b1 = Arc::new(Backend::new("127.0.0.1:8080".parse().unwrap(), 100));
        let b2 = Arc::new(Backend::new("127.0.0.1:8081".parse().unwrap(), 100));
        
        // artificially increase connections on b1
        b1.inc_connections();

        let backends = vec![b1.clone(), b2.clone()];

        // Should pick b2 (0 connections)
        let s1 = lb.next_server(&backends).unwrap();
        assert_eq!(s1.addr, b2.addr);

        // increase b2
        b2.inc_connections();
        b2.inc_connections(); // now b2 has 2, b1 has 1

        // Should pick b1 (1 connection)
        let s2 = lb.next_server(&backends).unwrap();
        assert_eq!(s2.addr, b1.addr);
    }
}
