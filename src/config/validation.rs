//! Configuration validation.
//!
//! # Responsibilities
//! - Semantic validation (serde handles syntactic)
//! - Check referential integrity (routes reference existing backends)
//! - Validate value ranges (timeouts > 0, ports valid)
//! - Detect conflicting routes
//!
//! # Design Decisions
//! - Returns all validation errors, not just first
//! - Validation is pure function: ProxyConfig → Result<(), Vec<ValidationError>>
//! - Runs before config is accepted into the system
