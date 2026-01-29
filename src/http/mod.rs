//! HTTP protocol handling subsystem.
//!
//! # Data Flow
//! ```text
//! TCP/TLS connection
//!     → server.rs (Axum setup, protocol detection)
//!     → request.rs (parse, add request ID, validate headers)
//!     → [routing layer decides backend] (Phase 2+)
//!     → [load balancer picks server] (Phase 3+)
//!     → response.rs (transform, add headers)
//!     → Send to client
//! ```

pub mod request;
pub mod response;
pub mod server;
pub mod websocket;

pub use request::{RequestId, RequestIdExt, RequestIdLayer, X_REQUEST_ID};
pub use server::HttpServer;
