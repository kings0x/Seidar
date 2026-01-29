//! Configuration schema definitions.
//!
//! This module defines the complete configuration structure for the proxy.
//! All types derive Serde traits for deserialization from config files.
//!
//! # Design Decisions
//! - All fields use `Option` or `Vec` with `#[serde(default)]` for flexibility
//! - Durations stored as seconds (u64) to avoid serde complexity
//! - No validation logic here; that belongs in `validation.rs`

use serde::Deserialize;

/// Root configuration for the reverse proxy.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct ProxyConfig {
    /// Listener configuration (bind address, TLS).
    pub listener: ListenerConfig,

    /// Route definitions mapping requests to backends.
    pub routes: Vec<RouteConfig>,

    /// Backend server definitions.
    pub backends: Vec<BackendConfig>,

    /// Health check settings.
    pub health_check: HealthCheckConfig,

    /// Timeout configuration.
    pub timeouts: TimeoutConfig,

    /// Rate limiting configuration.
    pub rate_limit: RateLimitConfig,

    /// Observability settings.
    pub observability: ObservabilityConfig,
}

/// Listener configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ListenerConfig {
    /// Bind address (e.g., "0.0.0.0:8080").
    pub bind_address: String,

    /// Optional TLS configuration.
    pub tls: Option<TlsConfig>,

    /// Maximum concurrent connections (backpressure).
    pub max_connections: usize,
}

impl Default for ListenerConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0:8080".to_string(),
            tls: None,
            max_connections: 10_000,
        }
    }
}

/// TLS configuration for the listener.
#[derive(Debug, Clone, Deserialize)]
pub struct TlsConfig {
    /// Path to certificate file (PEM).
    pub cert_path: String,

    /// Path to private key file (PEM).
    pub key_path: String,
}

/// Route configuration mapping requests to backend groups.
#[derive(Debug, Clone, Deserialize)]
pub struct RouteConfig {
    /// Route identifier for logging/metrics.
    pub name: String,

    /// Host header to match (exact match).
    pub host: Option<String>,

    /// Path prefix to match.
    pub path_prefix: Option<String>,

    /// Backend group name to forward to.
    pub backend_group: String,

    /// Route priority (higher = checked first).
    #[serde(default)]
    pub priority: u32,
}

/// Backend server configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct BackendConfig {
    /// Unique backend identifier.
    pub name: String,

    /// Backend group this server belongs to.
    pub group: String,

    /// Backend address (e.g., "127.0.0.1:3000").
    pub address: String,

    /// Weight for weighted load balancing (default: 1).
    #[serde(default = "default_weight")]
    pub weight: u32,

    /// Maximum concurrent connections to this backend.
    #[serde(default = "default_max_backend_conns")]
    pub max_connections: usize,
}

fn default_weight() -> u32 {
    1
}

fn default_max_backend_conns() -> usize {
    100
}

/// Health check configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct HealthCheckConfig {
    /// Enable active health checks.
    pub enabled: bool,

    /// Health check interval in seconds.
    pub interval_secs: u64,

    /// Health check timeout in seconds.
    pub timeout_secs: u64,

    /// Path to probe for HTTP health checks.
    pub path: String,

    /// Number of consecutive failures before marking unhealthy.
    pub unhealthy_threshold: u32,

    /// Number of consecutive successes before marking healthy.
    pub healthy_threshold: u32,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_secs: 10,
            timeout_secs: 5,
            path: "/health".to_string(),
            unhealthy_threshold: 3,
            healthy_threshold: 2,
        }
    }
}

/// Timeout configuration for various operations.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct TimeoutConfig {
    /// Connection establishment timeout in seconds.
    pub connect_secs: u64,

    /// Request timeout (total time for request/response) in seconds.
    pub request_secs: u64,

    /// Idle connection timeout in seconds.
    pub idle_secs: u64,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            connect_secs: 5,
            request_secs: 30,
            idle_secs: 60,
        }
    }
}

/// Rate limiting configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RateLimitConfig {
    /// Enable rate limiting.
    pub enabled: bool,

    /// Maximum requests per second per IP.
    pub requests_per_second: u32,

    /// Burst capacity.
    pub burst_size: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            requests_per_second: 100,
            burst_size: 50,
        }
    }
}

/// Observability configuration.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ObservabilityConfig {
    /// Log level (trace, debug, info, warn, error).
    pub log_level: String,

    /// Enable metrics endpoint.
    pub metrics_enabled: bool,

    /// Metrics endpoint bind address.
    pub metrics_address: String,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            metrics_enabled: true,
            metrics_address: "0.0.0.0:9090".to_string(),
        }
    }
}
