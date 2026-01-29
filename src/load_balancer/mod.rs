//! Load balancing subsystem.
//!
//! # Data Flow
//! ```text
//! Route matched → backend_group identified
//!     → pool.rs (get available backends)
//!     → Apply load balancing algorithm:
//!         - round_robin.rs (rotate through backends)
//!         - least_conn.rs (pick backend with fewest connections)
//!     → backend.rs (acquire connection from pool)
//!     → Return backend connection or error
//! ```

pub mod backend;
pub mod least_conn;
#[allow(dead_code)] // To be implemented/used
pub mod pool;
pub mod round_robin;

use std::fmt::Debug;
use std::sync::Arc;
use backend::Backend;

/// Interface for load balancing algorithms.
pub trait LoadBalancer: Send + Sync + Debug {
    /// Select the next backend from the list.
    fn next_server(&self, backends: &[Arc<Backend>]) -> Option<Arc<Backend>>;
}

/// Available load balancing algorithms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadBalancerAlgo {
    RoundRobin,
    LeastConnections,
}
