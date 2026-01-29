//! TCP listener implementation.
//!
//! # Responsibilities
//! - Bind to configured address(es)
//! - Accept incoming TCP connections
//! - Enforce max_connections limit (backpressure)
//! - Delegate TLS handshake when configured
//!
//! # Design Decisions
//! - Uses Tokio's TcpListener for async accept
//! - Semaphore-based connection limiting
//! - Accept errors are logged but don't crash the listener
//! - Supports multiple listeners for different ports
