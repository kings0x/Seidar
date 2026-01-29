//! Configuration management subsystem.
//!
//! # Data Flow
//! ```text
//! config file (YAML/TOML)
//!     → loader.rs (parse & deserialize)
//!     → validation.rs (semantic checks)
//!     → ProxyConfig (validated, immutable)
//!     → shared via Arc to all subsystems
//!
//! On reload signal:
//!     watcher.rs detects change
//!     → loader.rs loads new config
//!     → validation.rs validates
//!     → atomic swap of Arc<ProxyConfig>
//!     → subsystems observe new config
//! ```
//!
//! # Design Decisions
//! - Config is immutable once loaded; changes require full reload
//! - All fields have defaults to allow minimal configs
//! - Validation separates syntactic (serde) from semantic checks

pub mod loader;
pub mod schema;
pub mod validation;
pub mod watcher;

pub use schema::ProxyConfig;
pub use schema::ListenerConfig;
pub use schema::RouteConfig;
pub use schema::BackendConfig;

