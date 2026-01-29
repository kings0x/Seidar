//! Route lookup and dispatch.
//!
//! # Responsibilities
//! - Store compiled routes
//! - Look up matching route for request
//! - Return matched route or explicit no-match
//!
//! # Design Decisions
//! - Immutable after construction (thread-safe without locks)
//! - O(1) host lookup via HashMap
//! - O(n) path prefix scan (acceptable for typical route counts)
//! - Explicit NoMatch rather than silent default
