//! Startup orchestration.
//!
//! # Responsibilities
//! - Load and validate configuration
//! - Initialize all subsystems in dependency order
//! - Start background tasks (health checks, metrics)
//! - Bind listeners and begin accepting traffic
//!
//! # Design Decisions
//! - Fail fast: any startup error is fatal
//! - Subsystems initialize in order, not concurrently
//! - Listeners start last (traffic only when ready)
