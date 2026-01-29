//! Metrics collection and exposition.
//!
//! # Responsibilities
//! - Define proxy metrics (RPS, latency, errors, connections)
//! - Expose Prometheus-compatible metrics endpoint
//! - Track per-backend and aggregate metrics
//!
//! # Metrics
//! - `proxy_requests_total` (counter): total requests by route, status
//! - `proxy_request_duration_seconds` (histogram): latency distribution
//! - `proxy_active_connections` (gauge): current connection count
//! - `proxy_backend_health` (gauge): 1=healthy, 0=unhealthy
//!
//! # Design Decisions
//! - Low-overhead metric updates (atomic operations)
//! - Labels for route, backend, status code
//! - Histogram buckets tuned for typical web latencies
