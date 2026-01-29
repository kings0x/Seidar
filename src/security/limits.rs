//! Request and connection limits.
//!
//! # Responsibilities
//! - Enforce maximum request body size
//! - Enforce maximum header size
//! - Enforce maximum header count
//! - Enforce maximum URI length
//!
//! # Design Decisions
//! - Limits checked before full parsing (early rejection)
//! - Configurable limits per route (optional)
//! - Return 413 Payload Too Large or 431 Request Header Fields Too Large
