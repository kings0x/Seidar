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

pub mod matcher;
pub mod router;

pub use router::Router;
pub use matcher::Matcher;
