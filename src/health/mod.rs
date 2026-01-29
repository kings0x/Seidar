//! Health checking subsystem.
//!
//! # Data Flow
//! ```text
//! Active health checks (active.rs):
//!     Periodic timer
//!     → Probe each backend
//!     → Update state.rs
//!
//! Passive health checks (passive.rs):
//!     Request failure observed
//!     → Increment failure count
//!     → Update state.rs if threshold exceeded
//!
//! State machine (state.rs):
//!     Healthy ←→ Unhealthy
//!     With thresholds to prevent flapping
//! ```
//!
//! # Design Decisions
//! - Active and passive checks are complementary
//! - State transitions require consecutive successes/failures
//! - Health state is per-backend, not per-pool

pub mod active;
pub mod passive;
pub mod state;
