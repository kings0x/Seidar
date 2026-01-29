//! Active health checking.
//!
//! # Responsibilities
//! - Periodically probe backends
//! - Update backend health state based on results

use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use crate::config::HealthCheckConfig;
use crate::load_balancer::pool::BackendManager;
use hyper_util::{
    client::legacy::{Client, connect::HttpConnector},
    rt::TokioExecutor,
};
use axum::http::Request;
use axum::body::Body;

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

    pub async fn run(self) {
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
        
        // Consume the first immediate tick so we wait 'interval' before first check
        // Or do we want immediate check? 
        // Let's do immediate check first to discover initial status.
        // So we just loop.
        
        loop {
            ticker.tick().await; 
            self.check_all().await;
        }
    }

    async fn check_all(&self) {
        let all_backends = self.backends.all_backends(); 
        
        for backend in all_backends {
            let addr = backend.addr; 
            let check_path = &self.config.path;
            
            // Construct probe URI
            // Assume HTTP for now
            let uri_string = format!("http://{}{}", addr, check_path);
            
            let request = match Request::builder()
                .method("GET") 
                .uri(uri_string)
                .body(Body::empty()) {
                    Ok(req) => req,
                    Err(e) => {
                        tracing::error!("Failed to build health check request: {}", e);
                        continue;
                    }
                };

            // Enforce timeout
            let timeout = Duration::from_secs(self.config.timeout_secs);
            let response_future = self.client.request(request);
            
            match time::timeout(timeout, response_future).await {
                Ok(Ok(response)) => {
                    if response.status().is_success() {
                        backend.mark_success(self.config.healthy_threshold as usize);
                    } else {
                        backend.mark_failure(self.config.unhealthy_threshold as usize);
                    }
                },
                Ok(Err(_e)) => {
                    backend.mark_failure(self.config.unhealthy_threshold as usize);
                },
                Err(_elapsed) => {
                    backend.mark_failure(self.config.unhealthy_threshold as usize);
                }
            }
        }
    }
}
