//! Rate limiting.
//!
//! # Responsibilities
//! - Track request rate per client IP
//! - Enforce requests-per-second limit
//! - Return 429 Too Many Requests when exceeded
//!
//! # Design Decisions
//! - Token bucket algorithm with burst support
//! - Per-IP tracking (not global)
//! - Efficient: O(1) check and update
//! - Memory bounded: evict old entries
