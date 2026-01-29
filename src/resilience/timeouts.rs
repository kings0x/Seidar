//! Timeout enforcement.
//!
//! # Responsibilities
//! - Wrap backend calls with timeout
//! - Enforce connect timeout, request timeout, idle timeout
//! - Cancel operations cleanly on timeout
//!
//! # Design Decisions
//! - Uses Tokio's timeout facilities
//! - Timeout errors are distinct from other errors
//! - Timed-out requests return 504 Gateway Timeout
