//! Network layer subsystem.
//!
//! # Data Flow
//! ```text
//! Incoming TCP connection
//!     → listener.rs (accept loop, connection limits)
//!     → tls.rs (optional TLS handshake)
//!     → connection.rs (lifecycle tracking, state machine)
//!     → Hand off to HTTP layer
//! ```
//!

pub mod connection;
pub mod listener;
pub mod tls;

pub use connection::{ConnectionGuard, ConnectionId, ConnectionState, ConnectionTracker};
pub use listener::{ConnectionPermit, Listener, ListenerError};
