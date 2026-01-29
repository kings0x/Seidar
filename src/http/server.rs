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

use axum::{
    body::Body,
    extract::{ConnectInfo, Extension, State},
    http::{Method, Request, StatusCode, Uri},
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
use std::time::Duration;
use tokio::net::TcpListener;
use tower_http::{
    timeout::TimeoutLayer,
    trace::TraceLayer,
};

use crate::config::{ProxyConfig, HealthCheckConfig};
use crate::http::request::RequestIdLayer;
use crate::routing::Router as ProxyRouter;
use crate::load_balancer::pool::BackendManager;
use crate::health::active::HealthMonitor;

/// Application state injected into handlers.
#[derive(Clone)]
pub struct AppState {
    pub router: Arc<ProxyRouter>,
    pub backends: Arc<BackendManager>,
    pub client: Client<HttpConnector, Body>,
    pub health_config: HealthCheckConfig,
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

        let state = AppState {
            router: proxy_router,
            backends: backend_manager.clone(),
            client,
            health_config: config.health_check.clone(),
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
    mut request: Request<Body>,
) -> impl IntoResponse {
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let path = request.uri().path().to_string();

    tracing::debug!(
        request_id = %request_id,
        path = %path,
        "Routing request"
    );

    // 1. Match Route
    if let Some(route) = state.router.match_request(&request) {
        // 2. Select Backend
        if let Some(backend_guard) = state.backends.get(&route.backend_group) {
            let backend_addr = backend_guard.addr;
            
            // 3. Rewrite URI
            let mut parts = request.uri().clone().into_parts();
            parts.scheme = Some(Scheme::HTTP); 
            
            if let Ok(authority) = Authority::from_str(&backend_addr.to_string()) {
                parts.authority = Some(authority);
            } else {
                return (StatusCode::BAD_GATEWAY, "Invalid backend address").into_response();
            }

            if parts.path_and_query.is_none() {
                 parts.path_and_query = Some(axum::http::uri::PathAndQuery::from_static("/"));
            }

            if let Ok(new_uri) = Uri::from_parts(parts) {
                *request.uri_mut() = new_uri;
            } else {
                return (StatusCode::INTERNAL_SERVER_ERROR, "URI rewrite failed").into_response();
            }

            // 4. Forward Request
            match state.client.request(request).await {
                Ok(response) => {
                    // Passive Health Check: Success
                    if response.status().is_server_error() {
                         match response.status() {
                             StatusCode::BAD_GATEWAY | StatusCode::SERVICE_UNAVAILABLE | StatusCode::GATEWAY_TIMEOUT => {
                                 backend_guard.mark_failure(state.health_config.unhealthy_threshold as usize);
                             },
                             _ => {
                                 backend_guard.mark_success(state.health_config.healthy_threshold as usize);
                             }
                         }
                    } else {
                        backend_guard.mark_success(state.health_config.healthy_threshold as usize);
                    }

                    let (parts, body) = response.into_parts();
                    let body = Body::new(body); 
                    Response::from_parts(parts, body)
                },
                Err(e) => {
                    tracing::error!(
                        request_id = %request_id,
                        error = %e,
                        "Upstream request failed"
                    );
                    // Passive Health Check: Failure
                    backend_guard.mark_failure(state.health_config.unhealthy_threshold as usize);
                    
                    (StatusCode::BAD_GATEWAY, "Upstream request failed").into_response()
                }
            }
        } else {
            tracing::warn!(
                request_id = %request_id,
                backend_group = %route.backend_group,
                "No available backends in group"
            );
            (StatusCode::BAD_GATEWAY, "No healthy backends available").into_response()
        }
    } else {
        tracing::warn!(
            request_id = %request_id,
            path = %path,
            "No route matched"
        );
        (StatusCode::NOT_FOUND, "No matching route found").into_response()
    }
}

/// Wait for shutdown signal (Ctrl+C).
async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install Ctrl+C handler");
    tracing::info!("Shutdown signal received");
}
