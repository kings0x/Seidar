//! OS signal handling.
//!
//! # Responsibilities
//! - Register signal handlers (SIGTERM, SIGINT, SIGHUP)
//! - Translate signals to internal events
//! - Trigger appropriate actions (shutdown, reload)
//!
//! # Design Decisions
//! - Uses Tokio's signal handling (async-safe)
//! - Multiple SIGTERM/SIGINT triggers forced shutdown
//! - SIGHUP triggers config reload, not shutdown
