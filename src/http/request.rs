//! Request handling and transformation.
//!
//! # Responsibilities
//! - Generate unique request ID (UUID or snowflake)
//! - Validate request headers and size limits
//! - Extract routing-relevant information (host, path)
//! - Prepare request for forwarding to backend
//!
//! # Design Decisions
//! - Request ID added as early as possible for tracing
//! - Header size limits enforced before full body read
//! - Original request preserved for logging; modified copy forwarded
