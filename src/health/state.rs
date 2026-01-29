//! Backend health state machine.
//!
//! # States
//! - Healthy: backend receives traffic
//! - Unhealthy: backend excluded from load balancing
//!
//! # State Transitions
//! ```text
//! Healthy → Unhealthy: consecutive failures >= unhealthy_threshold
//! Unhealthy → Healthy: consecutive successes >= healthy_threshold
//! ```
//!
//! # Design Decisions
//! - Hysteresis prevents flapping
//! - State changes logged for observability
//! - Counters reset on state transition
