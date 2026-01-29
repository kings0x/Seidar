//! Connection state machine and lifecycle tracking.
//!
//! # Responsibilities
//! - Track connection state (Accepting → Active → Draining → Closed)
//! - Generate unique connection IDs for tracing
//! - Coordinate graceful shutdown per-connection
//! - Collect per-connection metrics
//!
//! # State Machine
//! ```text
//! ┌──────────┐     ┌────────────┐     ┌────────┐
//! │ Accepting│────▶│  Active    │────▶│Draining│
//! └──────────┘     └────────────┘     └────────┘
//!                        │                 │
//!                        ▼                 ▼
//!                   ┌────────┐        ┌────────┐
//!                   │ Closed │◀───────│ Closed │
//!                   └────────┘        └────────┘
//! ```
//!
//! # Design Decisions
//! - Connection ID is u64 for speed; collision risk acceptable
//! - State transitions are explicit, not implicit
//! - Draining state allows in-flight requests to complete
