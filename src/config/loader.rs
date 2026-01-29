//! Configuration loading from files.
//!
//! # Responsibilities
//! - Read configuration files from disk
//! - Parse YAML/TOML into `ProxyConfig`
//! - Handle file I/O errors gracefully
//!
//! # Future Implementation
//! - Support multiple config formats (YAML, TOML, JSON)
//! - Merge config from multiple sources (file + env vars)
//! - Validate file permissions for security
