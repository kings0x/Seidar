//! Observability subsystem.
//!
//! # Data Flow
//! ```text
//! All subsystems produce:
//!     → logging.rs (structured log events)
//!     → metrics.rs (counters, gauges, histograms)
//!     → tracing.rs (spans with correlation IDs)
//!
//! Consumers:
//!     → Log aggregation (stdout, file, remote)
//!     → Metrics endpoint (Prometheus scrape)
//!     → Distributed tracing (optional, e.g., Jaeger)
//! ```
//!
//! # Design Decisions
//! - Structured logging (JSON) for machine parsing
//! - Request ID flows through all subsystems
//! - Metrics are cheap (atomic increments)
//! - Tracing is optional to reduce overhead when not needed

pub mod logging;
pub mod metrics;
pub mod tracing;
