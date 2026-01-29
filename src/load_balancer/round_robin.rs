//! Round-robin load balancing strategy.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use crate::load_balancer::{LoadBalancer, backend::Backend};

/// Round-robin selector.
/// Stores an internal counter to rotate through backends.
#[derive(Debug, Default)]
pub struct RoundRobin {
    counter: AtomicUsize,
}

impl RoundRobin {
    pub fn new() -> Self {
        Self::default()
    }
}

impl LoadBalancer for RoundRobin {
    fn next_server(&self, backends: &[Arc<Backend>]) -> Option<Arc<Backend>> {
        if backends.is_empty() {
            return None;
        }

        // Phase 4: Filter healthy
        // Note: Simple loop detection to avoid infinite loop if all unhealthy
        let start_count = self.counter.fetch_add(1, Ordering::Relaxed);
        let len = backends.len();

        for i in 0..len {
            let index = (start_count + i) % len;
            let backend = &backends[index];
            if backend.is_healthy() {
                return Some(backend.clone());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_robin() {
        let lb = RoundRobin::new();
        let b1 = Arc::new(Backend::new("127.0.0.1:8080".parse().unwrap(), 100));
        let b2 = Arc::new(Backend::new("127.0.0.1:8081".parse().unwrap(), 100));
        let backends = vec![b1.clone(), b2.clone()];

        let s1 = lb.next_server(&backends).unwrap();
        assert_eq!(s1.addr, b1.addr);

        let s2 = lb.next_server(&backends).unwrap();
        assert_eq!(s2.addr, b2.addr);

        let s3 = lb.next_server(&backends).unwrap();
        assert_eq!(s3.addr, b1.addr);
    }
}
