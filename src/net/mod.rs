//! Network layer subsystem.
//!
//! # Data Flow
//! ```text
//! Incoming TCP connection
//!     → listener.rs (accept loop, connection limits)
//!     → tls.rs (optional TLS handshake)
//!     → connection.rs (lifecycle tracking, state machine)
//!     → Hand off to HTTP layer
//!
//! Connection States:
//!     Accepting → Handshaking → Active → Draining → Closed
//! ```
//!
//! # Design Decisions
//! - Bounded accept queue prevents resource exhaustion
//! - Each connection tracked for graceful shutdown
//! - TLS is optional and handled transparently

pub mod connection;
pub mod listener;
pub mod tls;
