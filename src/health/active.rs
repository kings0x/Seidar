//! Active health checking.
//!
//! # Responsibilities
//! - Periodically probe backends
//! - Update backend health state based on results

use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tokio::sync::broadcast;
use crate::config::HealthCheckConfig;
use crate::load_balancer::pool::BackendManager;
use hyper_util::{
    client::legacy::{Client, connect::HttpConnector},
    rt::TokioExecutor,
};
use axum::http::Request;
use axum::body::Body;
use crate::observability::metrics;

pub struct HealthMonitor {
    backends: Arc<BackendManager>,
    config: HealthCheckConfig,
    client: Client<HttpConnector, Body>,
}

impl HealthMonitor {
    pub fn new(backends: Arc<BackendManager>, config: HealthCheckConfig) -> Self {
        let client = Client::builder(TokioExecutor::new())
            .build(HttpConnector::new());
        
        Self {
            backends,
            config,
            client,
        }
    }

    pub async fn run(self, mut shutdown: broadcast::Receiver<()>) {
        if !self.config.enabled {
            tracing::info!("Active health checks disabled");
            return;
        }

        tracing::info!(
            interval = self.config.interval_secs,
            path = %self.config.path,
            "Health monitor starting"
        );

        let interval = Duration::from_secs(self.config.interval_secs);
        let mut ticker = time::interval(interval);
        
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    self.check_all().await;
                }
                _ = shutdown.recv() => {
                    tracing::info!("Health monitor received shutdown signal, exiting loop");
                    break;
                }
            }
        }
    }

    async fn check_all(&self) {
        let all_backends = self.backends.all_backends(); 
        
        for backend in all_backends {
            let addr = backend.addr; 
            let check_path = &self.config.path;
            
            let uri_string = format!("http://{}{}", addr, check_path);
            
            let request = match Request::builder()
                .method("GET") 
                .uri(uri_string)
                .header("user-agent", "rust-proxy-health-check")
                .body(Body::empty()) {
                    Ok(req) => req,
                    Err(e) => {
                        tracing::error!("Failed to build health check request: {}", e);
                        continue;
                    }
                };

            let timeout = Duration::from_secs(self.config.timeout_secs);
            let response_future = self.client.request(request);
            
            let healthy = match time::timeout(timeout, response_future).await {
                Ok(Ok(response)) => {
                    let success = response.status().is_success();
                    if !success {
                        tracing::warn!(addr = %addr, status = %response.status(), "Health check failed: non-success status");
                    }
                    success
                },
                Ok(Err(e)) => {
                    tracing::warn!(addr = %addr, error = %e, "Health check failed: connection error");
                    false
                }
                Err(_) => {
                    tracing::warn!(addr = %addr, "Health check failed: timeout");
                    false
                },
            };

            if healthy {
                backend.mark_success(self.config.healthy_threshold as usize);
            } else {
                backend.mark_failure(self.config.unhealthy_threshold as usize);
            }

            // Record health metric (Phase 6)
            metrics::record_backend_health(&addr.to_string(), backend.is_healthy());
        }
    }
}
