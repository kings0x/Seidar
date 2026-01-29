//! Metrics collection and exposition.

use std::net::SocketAddr;
use metrics_exporter_prometheus::PrometheusBuilder;
use std::time::Instant;
use metrics::{counter, histogram, gauge};

/// Initialize metrics exporter and server.
pub fn init_metrics(addr: SocketAddr) {
    PrometheusBuilder::new()
        .with_http_listener(addr)
        .install()
        .expect("failed to install Prometheus recorder");
        
    tracing::info!("Metrics server listening on http://{}", addr);
}

/// Helper to record a proxy request.
pub fn record_request(method: &str, status: u16, backend: &str, duration: Instant) {
    let labels = [
        ("method", method.to_string()),
        ("status", status.to_string()),
        ("backend", backend.to_string()),
    ];
    
    counter!("proxy_requests_total", &labels).increment(1);
    histogram!("proxy_request_duration_seconds", &labels).record(duration.elapsed().as_secs_f64());
}

/// Helper to update backend health status for metrics.
pub fn record_backend_health(backend: &str, healthy: bool) {
    let val = if healthy { 1.0 } else { 0.0 };
    gauge!("proxy_backend_healthy", "backend" => backend.to_string()).set(val);
}
