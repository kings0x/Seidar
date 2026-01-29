//! Security subsystem.
//!
//! # Data Flow
//! ```text
//! Incoming request:
//!     → rate_limit.rs (check per-IP limits)
//!     → limits.rs (check request size, header count)
//!     → headers.rs (sanitize, add X-Forwarded-*)
//!     → Pass to routing
//! ```
//!
//! # Design Decisions
//! - Defense in depth: multiple layers of protection
//! - Fail closed: reject on any security check failure
//! - No trust in client input

pub mod headers;
pub mod limits;
pub mod rate_limit;
