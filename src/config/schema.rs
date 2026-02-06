//! Configuration schema definitions.
//!
//! This module defines the complete configuration structure for the proxy.
//! All types derive Serde traits for deserialization from config files.

use serde::{Deserialize, Serialize};

/// Root configuration for the reverse proxy.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
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

    /// Retry configuration.
    pub retries: RetryConfig,

    /// Observability settings.
    pub observability: ObservabilityConfig,

    /// Blockchain integration settings.
    pub blockchain: BlockchainConfig,

    #[serde(default)]
    pub payments: PaymentConfig,

    #[serde(default)]
    pub qos: QosConfig,

    #[serde(default)]
    pub admin: AdminConfig,

    #[serde(default)]
    pub security: SecurityConfig,
}

/// Listener configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
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
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TlsConfig {
    /// Path to certificate file (PEM).
    pub cert_path: String,

    /// Path to private key file (PEM).
    pub key_path: String,
}

/// Route configuration mapping requests to backend groups.
#[derive(Debug, Clone, Deserialize, Serialize)]
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
#[derive(Debug, Clone, Deserialize, Serialize)]
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
#[derive(Debug, Clone, Deserialize, Serialize)]
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
#[derive(Debug, Clone, Deserialize, Serialize)]
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
#[derive(Debug, Clone, Deserialize, Serialize)]
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

/// Retry configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct RetryConfig {
    /// Enable retries.
    pub enabled: bool,

    /// Maximum number of retry attempts.
    pub max_attempts: u32,

    /// Base delay for exponential backoff in milliseconds.
    pub base_delay_ms: u64,

    /// Maximum delay for exponential backoff in milliseconds.
    pub max_delay_ms: u64,

    /// Percentage of requests that can be retries (retry budget).
    /// e.g., 0.1 for 10% budget.
    pub budget_ratio: f32,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_attempts: 3,
            base_delay_ms: 100,
            max_delay_ms: 2000,
            budget_ratio: 0.1,
        }
    }
}

/// Observability configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
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

/// Admin dashboard configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct AdminConfig {
    /// Enable admin dashboard.
    pub enabled: bool,

    /// API key for authentication (Bearer token).
    pub api_key: String,

    /// Admin dashboard bind address.
    pub bind_address: String,
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            // WARNING: This is a placeholder! Change this in production.
            api_key: "CHANGE_ME_IN_PRODUCTION".to_string(),
            bind_address: "127.0.0.1:8081".to_string(),
        }
    }
}

/// Blockchain integration configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct BlockchainConfig {
    /// Enable blockchain integration.
    pub enabled: bool,

    /// JSON-RPC endpoint URL.
    pub rpc_url: String,

    /// Failover JSON-RPC endpoint URLs.
    #[serde(default)]
    pub failover_urls: Vec<String>,

    /// Chain ID (e.g., 1 for Ethereum mainnet, 31337 for local Anvil).
    pub chain_id: u64,

    /// RPC request timeout in seconds.
    pub rpc_timeout_secs: u64,

    /// Number of block confirmations required for finality.
    pub confirmation_blocks: u32,

    /// Gas price multiplier (1.0 = estimated, 1.2 = 20% buffer).
    pub gas_price_multiplier: f64,

    /// Maximum gas price in gwei (protection against spikes).
    pub max_gas_price_gwei: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct PaymentConfig {
    /// Enable payment monitoring.
    pub enabled: bool,

    /// Address of the PaymentProcessor contract.
    pub contract_address: String,

    /// Polling interval in milliseconds.
    pub monitor_interval_ms: u64,

    /// Grace period for expired subscriptions in seconds.
    #[serde(default)]
    pub grace_period_secs: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct QosConfig {
    pub tier_1_rps: u64,
    pub tier_2_rps: u64,
    pub tier_3_rps: u64,
    pub tier_1_max_conns: usize,
    pub tier_2_max_conns: usize,
    pub tier_3_max_conns: usize,
}

impl Default for QosConfig {
    fn default() -> Self {
        Self {
            tier_1_rps: 10,
            tier_2_rps: 100,
            tier_3_rps: 1000,
            tier_1_max_conns: 1,
            tier_2_max_conns: 10,
            tier_3_max_conns: 1000,
        }
    }
}

impl Default for PaymentConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            contract_address: String::new(),
            monitor_interval_ms: 10000,
            grace_period_secs: 300, // 5 minutes default grace
        }
    }
}

impl Default for BlockchainConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            rpc_url: "http://localhost:8545".to_string(),
            failover_urls: Vec::new(),
            chain_id: 1,
            rpc_timeout_secs: 10,
            confirmation_blocks: 3,
            gas_price_multiplier: 1.2,
            max_gas_price_gwei: 500,
        }
    }
}

/// Security hardening configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct SecurityConfig {
    /// Enable security headers.
    pub enable_headers: bool,
    /// Maximum body size in bytes.
    pub max_body_size: usize,
    /// Enable strict input validation.
    pub strict_validation: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_headers: true,
            max_body_size: 2 * 1024 * 1024, // 2MB
            strict_validation: true,
        }
    }
}
