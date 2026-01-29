//! Resilience subsystem.
//!
//! # Data Flow
//! ```text
//! Request to backend:
//!     → timeouts.rs (enforce connect/request timeout)
//!     → On failure: retries.rs (check if retryable, retry with backoff)
//!     → circuit_breaker.rs (track failures, open circuit if threshold exceeded)
//! ```
//!
//! # Design Decisions
//! - Timeouts are non-negotiable; every external call has a deadline
//! - Retries only for idempotent requests (GET, HEAD, etc.)
//! - Circuit breaker prevents cascading failures
//! - All resilience logic is composable middleware

pub mod circuit_breaker;
pub mod retries;
pub mod timeouts;
