//! Route matching logic.
//!
//! # Responsibilities
//! - Match host header (exact match)
//! - Match path prefix
//! - Combine conditions with AND semantics
//!
//! # Design Decisions
//! - Host matching is case-insensitive (per HTTP spec)
//! - Path matching is case-sensitive
//! - Empty condition = always matches (wildcard)
//! - No regex to guarantee O(n) matching
