//! Routing subsystem.
//!
//! # Data Flow
//! ```text
//! Incoming Request (host, path, headers)
//!     → router.rs (route lookup)
//!     → matcher.rs (evaluate match conditions)
//!     → Return: matched Route or NoMatch
//!
//! Route Compilation (at startup):
//!     RouteConfig[]
//!     → Sort by priority
//!     → Compile matchers (prefix trees, exact maps)
//!     → Freeze as immutable Router
//! ```
//!
//! # Design Decisions
//! - Routes compiled at startup, immutable at runtime
//! - No regex in hot path (prefix matching only)
//! - Deterministic: same input always matches same route
//! - First match wins (ordered by priority)

pub mod matcher;
pub mod router;
