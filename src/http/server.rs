//! HTTP server setup and configuration.
//!
//! # Responsibilities
//! - Create Axum Router with all handlers
//! - Configure HTTP/1.1 and HTTP/2 support
//! - Wire up middleware (tracing, limits, request ID)
//! - Bind server to listener
//! - Dispatch requests to routing engine
//! - Forward requests to upstream backends
//! - Health monitoring (active & passive)
//! - Resilience (timeouts, retries)
//! - Observability (metrics, correlation IDs)

use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{Method, Request, StatusCode, Uri, header},
    response::{IntoResponse, Response},
    routing::any,
    Router,
};
use hyper_util::{
    client::legacy::{Client, connect::HttpConnector},
    rt::TokioExecutor,
};
use axum::http::uri::{Scheme, Authority};
use std::net::SocketAddr;
use std::sync::Arc;
use std::str::FromStr;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tower_http::{
    timeout::TimeoutLayer,
    trace::TraceLayer,
};

use crate::config::{ProxyConfig, HealthCheckConfig, RetryConfig, ObservabilityConfig};
use crate::http::request::RequestIdLayer;
use crate::routing::Router as ProxyRouter;
use crate::load_balancer::pool::BackendManager;
use crate::health::active::HealthMonitor;
use crate::resilience::retries::{RetryBudget, is_retryable};
use crate::resilience::backoff::calculate_backoff;
use crate::observability::metrics;

/// Application state injected into handlers.
#[derive(Clone)]
pub struct AppState {
    pub router: Arc<ProxyRouter>,
    pub backends: Arc<BackendManager>,
    pub client: Client<HttpConnector, Body>,
    pub health_config: HealthCheckConfig,
    pub retry_config: RetryConfig,
    pub retry_budget: Arc<RetryBudget>,
    pub observability_config: ObservabilityConfig,
}

/// HTTP server for the reverse proxy.
pub struct HttpServer {
    router: Router,
    config: ProxyConfig,
    backend_manager: Arc<BackendManager>,
}

impl HttpServer {
    /// Create a new HTTP server with the given configuration.
    pub fn new(config: ProxyConfig) -> Self {
        // Initialize subsystems
        let proxy_router = Arc::new(ProxyRouter::from_config(config.routes.clone()));
        let backend_manager = Arc::new(BackendManager::new(config.backends.clone()));
        
        // Initialize HTTP Client
        let client = Client::builder(TokioExecutor::new())
            .build(HttpConnector::new());

        // Initialize Retry Budget (ratio from config)
        let retry_budget = Arc::new(RetryBudget::new(config.retries.budget_ratio, 100));

        let state = AppState {
            router: proxy_router,
            backends: backend_manager.clone(),
            client,
            health_config: config.health_check.clone(),
            retry_config: config.retries.clone(),
            retry_budget,
            observability_config: config.observability.clone(),
        };

        let router = Self::build_router(&config, state);
        Self { 
            router, 
            config,
            backend_manager,
        }
    }

    /// Build the Axum router with all middleware layers.
    #[allow(deprecated)]
    fn build_router(config: &ProxyConfig, state: AppState) -> Router {
        Router::new()
            .route("/{*path}", any(proxy_handler))
            .route("/", any(proxy_handler))
            .with_state(state)
            .layer(TimeoutLayer::new(Duration::from_secs(config.timeouts.request_secs)))
            .layer(RequestIdLayer)
            .layer(TraceLayer::new_for_http())
    }

    /// Run the server, accepting connections on the given listener.
    pub async fn run(self, listener: TcpListener) -> Result<(), std::io::Error> {
        let addr = listener.local_addr()?;
        tracing::info!(
            address = %addr,
            "HTTP server starting"
        );

        // Spawn Health Monitor (Phase 4)
        if self.config.health_check.enabled {
            let monitor = HealthMonitor::new(
                self.backend_manager.clone(), 
                self.config.health_check.clone()
            );
            tokio::spawn(async move {
                monitor.run().await;
            });
        }

        // Create the router
        let app = self.router.into_make_service_with_connect_info::<SocketAddr>();

        // Serve with graceful shutdown
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await?;

        tracing::info!("HTTP server stopped");
        Ok(())
    }

    /// Get a reference to the config.
    pub fn config(&self) -> &ProxyConfig {
        &self.config
    }
}

/// Main proxy handler.
/// Looks up route, selects backend, and forwards request.
async fn proxy_handler(
    State(state): State<AppState>,
    ConnectInfo(_addr): ConnectInfo<SocketAddr>,
    request: Request<Body>,
) -> impl IntoResponse {
    let start_time = Instant::now();
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let path = request.uri().path().to_string();
    let method = request.method().clone();
    let method_str = method.to_string();

    tracing::debug!(
        request_id = %request_id,
        method = %method,
        path = %path,
        "Proxying request"
    );

    // 1. Match Route
    let route = match state.router.match_request(&request) {
        Some(r) => r,
        None => {
            tracing::warn!(request_id = %request_id, path = %path, "No route matched");
            metrics::record_request(&method_str, 404, "none", start_time);
            return (StatusCode::NOT_FOUND, "No matching route found").into_response();
        }
    };

    // 2. Buffer Request Body if retriable
    let (parts, body) = request.into_parts();
    let body_bytes = if state.retry_config.enabled && method.is_idempotent() {
        match axum::body::to_bytes(body, 1024 * 1024).await {
            Ok(bytes) => Some(bytes),
            Err(_) => None,
        }
    } else {
        None
    };

    state.retry_budget.record_request();

    // 3. Retry Loop
    let mut attempts = 0;
    let max_attempts = if state.retry_config.enabled && (body_bytes.is_some() || method == Method::GET || method == Method::HEAD) {
        state.retry_config.max_attempts
    } else {
        1
    };

    loop {
        attempts += 1;
        
        // Select Backend
        let backend_guard = match state.backends.get(&route.backend_group) {
            Some(g) => g,
            None => {
                tracing::warn!(request_id = %request_id, group = %route.backend_group, "No healthy backends");
                metrics::record_request(&method_str, 503, "none", start_time);
                return (StatusCode::SERVICE_UNAVAILABLE, "No healthy backends").into_response();
            }
        };

        // Construct Request for this attempt
        let mut req = Request::builder()
            .method(method.clone())
            .version(parts.version);
        
        // Copy headers and PROPRAGATE X-Request-ID (Phase 6)
        if let Some(headers) = req.headers_mut() {
            for (k, v) in parts.headers.iter() {
                headers.insert(k.clone(), v.clone());
            }
            // Ensure request ID is present (it should be due to layer, but let's be explicit)
            headers.insert("x-request-id", header::HeaderValue::from_str(&request_id).unwrap());
        }

        // URI rewrite
        let mut uri_parts = parts.uri.clone().into_parts();
        uri_parts.scheme = Some(Scheme::HTTP);
        if let Ok(authority) = Authority::from_str(&backend_guard.addr.to_string()) {
            uri_parts.authority = Some(authority);
        }
        let uri = Uri::from_parts(uri_parts).unwrap_or(parts.uri.clone());
        let backend_addr_str = backend_guard.addr.to_string();
        
        let req = req.uri(uri)
            .body(if let Some(ref bytes) = body_bytes {
                Body::from(bytes.clone())
            } else {
                Body::empty() 
            })
            .unwrap();

        // Forward
        match state.client.request(req).await {
            Ok(response) => {
                let status = response.status();
                
                // Check if we should retry
                if attempts < max_attempts 
                    && is_retryable(&method, Some(status), false)
                    && state.retry_budget.can_retry()
                {
                    let backoff = calculate_backoff(attempts, state.retry_config.base_delay_ms, state.retry_config.max_delay_ms);
                    tracing::info!(request_id = %request_id, attempt = attempts, delay = ?backoff, status = %status, "Retrying request");
                    tokio::time::sleep(backoff).await;
                    continue;
                }

                // Metrics (Phase 6)
                metrics::record_request(&method_str, status.as_u16(), &backend_addr_str, start_time);

                // Passive Health Check update
                if status.is_server_error() {
                    match status {
                        StatusCode::BAD_GATEWAY | StatusCode::SERVICE_UNAVAILABLE | StatusCode::GATEWAY_TIMEOUT => {
                            backend_guard.mark_failure(state.health_config.unhealthy_threshold as usize);
                        }
                        _ => {
                            backend_guard.mark_success(state.health_config.healthy_threshold as usize);
                        }
                    }
                } else {
                    backend_guard.mark_success(state.health_config.healthy_threshold as usize);
                }

                let (parts, body) = response.into_parts();
                return Response::from_parts(parts, Body::new(body)).into_response();
            }
            Err(e) => {
                tracing::error!(request_id = %request_id, attempt = attempts, error = %e, "Upstream error");
                
                if attempts < max_attempts 
                    && is_retryable(&method, None, true)
                    && state.retry_budget.can_retry()
                {
                    let backoff = calculate_backoff(attempts, state.retry_config.base_delay_ms, state.retry_config.max_delay_ms);
                    tracing::info!(request_id = %request_id, attempt = attempts, delay = ?backoff, "Retrying after network error");
                    tokio::time::sleep(backoff).await;
                    continue;
                }

                // Metrics (Phase 6)
                metrics::record_request(&method_str, 502, &backend_addr_str, start_time);

                backend_guard.mark_failure(state.health_config.unhealthy_threshold as usize);
                return (StatusCode::BAD_GATEWAY, "Upstream request failed").into_response();
            }
        }
    }
}

/// Wait for shutdown signal (Ctrl+C).
async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install Ctrl+C handler");
    tracing::info!("Shutdown signal received");
}
