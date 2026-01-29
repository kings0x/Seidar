//! Active health checking.
//!
//! # Responsibilities
//! - Periodically probe backend health endpoints
//! - Send HTTP requests to configured health path
//! - Report success/failure to state machine
//!
//! # Design Decisions
//! - Uses dedicated HTTP client (not connection pool)
//! - Timeout per health check (distinct from request timeout)
//! - Jittered intervals prevent thundering herd
//! - Runs as background task, doesn't block request path
