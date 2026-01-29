//! HTTP server setup and configuration.
//!
//! # Responsibilities
//! - Create Axum Router with all handlers
//! - Configure HTTP/1.1 and HTTP/2 support
//! - Wire up middleware (tracing, metrics, limits)
//! - Bind server to listener
//!
//! # Design Decisions
//! - Uses Axum for ergonomic async HTTP handling
//! - Server configuration separate from business logic
//! - Graceful shutdown integrated with lifecycle module
