//! HTTP server setup and configuration.
//!
//! # Responsibilities
//! - Create Axum Router with all handlers
//! - Configure HTTP/1.1 and HTTP/2 support
//! - Wire up middleware (tracing, limits, request ID)
//! - Bind server to listener
//! - Dispatch requests to routing engine
//! - Forward requests to upstream backends

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

use crate::config::ProxyConfig;
use crate::http::request::RequestIdLayer;
use crate::routing::Router as ProxyRouter;
use crate::load_balancer::pool::BackendManager;

/// Application state injected into handlers.
#[derive(Clone)]
pub struct AppState {
    pub router: Arc<ProxyRouter>,
    pub backends: Arc<BackendManager>,
    pub client: Client<HttpConnector, Body>,
}

/// HTTP server for the reverse proxy.
pub struct HttpServer {
    router: Router,
    config: ProxyConfig,
}

impl HttpServer {
    /// Create a new HTTP server with the given configuration.
    pub fn new(config: ProxyConfig) -> Self {
        // Initialize subsystems
        let proxy_router = Arc::new(ProxyRouter::from_config(config.routes.clone()));
        let backend_manager = Arc::new(BackendManager::new(config.backends.clone()));
        
        // Initialize HTTP Client (pooling is handled internally)
        let client = Client::builder(TokioExecutor::new())
            .build(HttpConnector::new());

        let state = AppState {
            router: proxy_router,
            backends: backend_manager,
            client,
        };

        let router = Self::build_router(&config, state);
        Self { router, config }
    }

    /// Build the Axum router with all middleware layers.
    #[allow(deprecated)]
    fn build_router(config: &ProxyConfig, state: AppState) -> Router {
        // Build router with middleware
        // Note: Layer order is bottom-up (last added = outermost)
        Router::new()
            // Catch-all handler for Phase 3: Dispatch and Proxy
            .route("/{*path}", any(proxy_handler))
            .route("/", any(proxy_handler))
            // Inject state
            .with_state(state)
            // Request timeout
            .layer(TimeoutLayer::new(Duration::from_secs(config.timeouts.request_secs)))
            // Request ID generation
            .layer(RequestIdLayer)
            // Request tracing
            .layer(TraceLayer::new_for_http())
    }

    /// Run the server, accepting connections on the given listener.
    pub async fn run(self, listener: TcpListener) -> Result<(), std::io::Error> {
        let addr = listener.local_addr()?;
        tracing::info!(
            address = %addr,
            "HTTP server starting"
        );

        // Create the router that captures connection info
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
        tracing::debug!(
            request_id = %request_id,
            route_id = %route.id,
            backend_group = %route.backend_group,
            "Route matched"
        );
        
        // 2. Select Backend
        // The guard ensures connection count is decremented when dropped
        if let Some(backend_guard) = state.backends.get(&route.backend_group) {
            let backend_addr = backend_guard.addr; // Deref to Backend
            
            tracing::info!(
                request_id = %request_id,
                route_id = %route.id,
                backend = %backend_addr,
                "Forwarding request"
            );

            // 3. Rewrite URI
            // incoming request usually has relative URI (e.g. /api/users)
            // we need absolute URI for hyper client (http://ip:port/api/users)
            let mut parts = request.uri().clone().into_parts();
            parts.scheme = Some(Scheme::HTTP); // Phase 3: assume HTTP
            
            if let Ok(authority) = Authority::from_str(&backend_addr.to_string()) {
                parts.authority = Some(authority);
            } else {
                tracing::error!("Invalid backend address format: {}", backend_addr);
                return (StatusCode::BAD_GATEWAY, "Invalid backend address").into_response();
            }

            // Path and query are preserved in 'parts' typically, but clarify:
            // if parts.path_and_query is None, use "/"
            if parts.path_and_query.is_none() {
                 parts.path_and_query = Some(axum::http::uri::PathAndQuery::from_static("/"));
            }

            if let Ok(new_uri) = Uri::from_parts(parts) {
                *request.uri_mut() = new_uri;
            } else {
                return (StatusCode::INTERNAL_SERVER_ERROR, "URI rewrite failed").into_response();
            }

            // 4. Forward Request
            // Hyper client handles connection pooling automatically
            match state.client.request(request).await {
                Ok(response) => {
                    // Convert hyper::Response<Incoming> to axum Response
                    let (parts, body) = response.into_parts();
                    let body = Body::new(body); // Convert Incoming to Body
                    Response::from_parts(parts, body)
                },
                Err(e) => {
                    tracing::error!(
                        request_id = %request_id,
                        error = %e,
                        "Upstream request failed"
                    );
                    (StatusCode::BAD_GATEWAY, "Upstream request failed").into_response()
                }
            }
            // guard is dropped here, decrementing connection count
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
