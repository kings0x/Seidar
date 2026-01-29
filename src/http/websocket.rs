//! WebSocket proxy handling.
//!
//! # Responsibilities
//! - Detect WebSocket upgrade requests
//! - Complete upgrade handshake with client
//! - Establish WebSocket connection to backend
//! - Bidirectional frame forwarding
//!
//! # Data Flow
//! ```text
//! Client ←──── WebSocket frames ────→ Proxy ←──── WebSocket frames ────→ Backend
//! ```
//!
//! # Design Decisions
//! - WebSocket handled separately from HTTP request/response
//! - Frame-level forwarding (no message buffering)
//! - Close frames propagated in both directions
//! - Ping/pong handled transparently
