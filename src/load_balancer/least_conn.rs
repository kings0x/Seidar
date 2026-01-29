//! Least-connections load balancing.
//!
//! # Algorithm
//! Pick the healthy backend with the fewest active connections.
//! Tie-breaker: first in list wins.
//!
//! # Design Decisions
//! - O(n) scan of backends (acceptable for typical pool sizes)
//! - Connection count from backend.rs is authoritative
//! - Skips unhealthy backends
