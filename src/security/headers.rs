//! Header manipulation and security headers.
//!
//! # Responsibilities
//! - Add X-Forwarded-For, X-Forwarded-Proto, X-Forwarded-Host
//! - Strip hop-by-hop headers
//! - Add security response headers (optional)
//!
//! # Design Decisions
//! - Preserve original client IP in X-Forwarded-For
//! - Never trust existing X-Forwarded-* from untrusted sources
//! - Configurable trusted proxy list for header trust
