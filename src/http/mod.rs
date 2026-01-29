//! HTTP protocol handling subsystem.
//!
//! # Data Flow
//! ```text
//! TCP/TLS connection
//!     → server.rs (Axum setup, protocol detection)
//!     → request.rs (parse, add request ID, validate headers)
//!     → [routing layer decides backend]
//!     → [load balancer picks server]
//!     → response.rs (transform, add headers)
//!     → Send to client
//!
//! WebSocket upgrade:
//!     → request.rs detects Upgrade header
//!     → websocket.rs handles bidirectional proxy
//! ```
//!
//! # Design Decisions
//! - HTTP/1.1 and HTTP/2 supported via Axum/hyper
//! - Request size limits enforced before full parse
//! - Request ID generated for every request (correlation)

pub mod request;
pub mod response;
pub mod server;
pub mod websocket;
