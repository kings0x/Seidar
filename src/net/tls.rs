//! TLS termination.
//!
//! # Responsibilities
//! - Load certificates and private keys
//! - Perform TLS handshake on accepted connections
//! - Support certificate reloading without restart
//!
//! # Design Decisions
//! - Uses rustls (no OpenSSL dependency)
//! - Async handshake to avoid blocking runtime
//! - Handshake timeout prevents slowloris-style attacks
//! - Certificate errors logged with client IP for debugging
