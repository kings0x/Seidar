//! Circuit breaker for backend protection.
//!
//! # States
//! - Closed: normal operation, requests pass through
//! - Open: backend assumed down, requests fail fast
//! - Half-Open: testing if backend recovered
//!
//! # State Transitions
//! ```text
//! Closed → Open: failure_count >= threshold within window
//! Open → Half-Open: after recovery timeout
//! Half-Open → Closed: probe request succeeds
//! Half-Open → Open: probe request fails
//! ```
//!
//! # Design Decisions
//! - Per-backend circuit breaker (not global)
//! - Fail fast in Open state (no waiting for timeout)
//! - Single probe in Half-Open (prevents hammering recovering backend)
