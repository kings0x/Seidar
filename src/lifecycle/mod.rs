//! Lifecycle management subsystem.
//!
//! # Data Flow
//! ```text
//! Startup (startup.rs):
//!     Load config → Validate → Initialize subsystems → Start listeners
//!
//! Shutdown (shutdown.rs):
//!     Signal received → Stop accepting → Drain connections → Exit
//!
//! Signals (signals.rs):
//!     SIGTERM/SIGINT → Trigger graceful shutdown
//!     SIGHUP → Trigger config reload
//! ```
//!
//! # Design Decisions
//! - Ordered startup: config first, then core, then listeners
//! - Ordered shutdown: stop accept, drain, close
//! - Shutdown has timeout: forced exit after deadline

pub mod shutdown;
pub mod signals;
pub mod startup;
