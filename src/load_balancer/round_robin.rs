//! Round-robin load balancing.
//!
//! # Algorithm
//! Rotate through healthy backends in order.
//! Weighted round-robin: backends with weight 2 get picked twice as often.
//!
//! # Design Decisions
//! - Uses atomic counter, no locks
//! - Skips unhealthy backends
//! - Wraps around on overflow
