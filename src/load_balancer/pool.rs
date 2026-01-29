//! Connection pool for backend servers.
//!
//! # Responsibilities
//! - Maintain pool of reusable connections per backend
//! - Enforce per-backend connection limits
//! - Clean up idle connections
//! - Provide connections to load balancer
//!
//! # Design Decisions
//! - Bounded pool prevents resource exhaustion
//! - Idle connections reaped after timeout
//! - LIFO reuse (most recently used = most likely valid)
//! - Connection acquisition is async with bounded wait
