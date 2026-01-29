//! Passive health checking (failure detection).
//!
//! # Responsibilities
//! - Observe request outcomes
//! - Track consecutive failures
//! - Trigger state transition on threshold breach
//!
//! # Design Decisions
//! - Only connection errors and 5xx count as failures
//! - Timeouts are failures
//! - 4xx are NOT failures (client error, not backend)
//! - Thread-safe counters for concurrent request tracking
