//! Response handling and transformation.
//!
//! # Responsibilities
//! - Transform backend response for client
//! - Add/remove headers (X-Request-ID, Server, etc.)
//! - Handle response streaming efficiently
//! - Map backend errors to appropriate HTTP status codes
//!
//! # Design Decisions
//! - Streaming responses avoid buffering entire body
//! - Hop-by-hop headers stripped automatically
//! - Backend timeouts result in 504 Gateway Timeout
