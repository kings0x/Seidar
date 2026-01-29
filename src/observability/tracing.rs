//! Distributed tracing support.
//!
//! # Responsibilities
//! - Extract trace context from incoming requests
//! - Propagate trace context to backend requests
//! - Create spans for proxy operations
//!
//! # Design Decisions
//! - Optional: tracing disabled by default for performance
//! - Supports W3C Trace Context headers
//! - Integration with OpenTelemetry exporters
