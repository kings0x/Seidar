//! Retry logic.
//!
//! # Responsibilities
//! - Determine if request is retryable (idempotent methods only)
//! - Execute retries with exponential backoff + jitter
//! - Enforce retry budget (max retries per time window)
//!
//! # Design Decisions
//! - Never retry POST/PUT/DELETE/PATCH (non-idempotent)
//! - Jittered backoff prevents thundering herd
//! - Retry budget prevents retry storms under load
//! - Connection errors always retryable; 5xx configurable
