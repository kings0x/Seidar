//! Backend server abstraction.
//!
//! # Responsibilities
//! - Represent a single backend server
//! - Track connection count
//! - Track health state
//! - Provide address for connection
//!
//! # Design Decisions
//! - Backend is immutable config + mutable state
//! - State updates via interior mutability (atomic/RwLock)
//! - Health state set by health module, read by load balancer
