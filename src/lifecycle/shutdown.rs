//! Graceful shutdown orchestration.
//!
//! # Shutdown Sequence
//! 1. Stop accepting new connections
//! 2. Wait for in-flight requests to complete (drain)
//! 3. Close idle connections
//! 4. Stop background tasks
//! 5. Exit
//!
//! # Design Decisions
//! - Drain has timeout: force close after deadline
//! - In-flight requests get full timeout to complete
//! - Shutdown progress logged for debugging stuck shutdowns
