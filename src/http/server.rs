//! HTTP server setup and configuration.
//!
//! # Responsibilities
//! - Create Axum Router with all handlers
//! - Configure HTTP/1.1 and HTTP/2 support
//! - Wire up middleware (tracing, limits, request ID)
//! - Bind server to listener
//! - Dispatch requests to routing engine

use axum::{
    body::Body,
    extract::{ConnectInfo, Extension, State},
    http::{Method, Request, StatusCode},
    response::IntoResponse,
    routing::any,
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tower_http::{
    timeout::TimeoutLayer,
    trace::TraceLayer,
};

use crate::config::ProxyConfig;
use crate::http::request::RequestIdLayer;
use crate::routing::Router as ProxyRouter;

/// HTTP server for the reverse proxy.
pub struct HttpServer {
    router: Router,
    config: ProxyConfig,
}

impl HttpServer {
    /// Create a new HTTP server with the given configuration.
    pub fn new(config: ProxyConfig) -> Self {
        let proxy_router = Arc::new(ProxyRouter::from_config(config.routes.clone()));
        let router = Self::build_router(&config, proxy_router);
        Self { router, config }
    }

    /// Build the Axum router with all middleware layers.
    #[allow(deprecated)]
    fn build_router(config: &ProxyConfig, proxy_router: Arc<ProxyRouter>) -> Router {
        // Build router with middleware
        // Note: Layer order is bottom-up (last added = outermost)
        Router::new()
            // Catch-all handler for Phase 2: Dispatch to routing engine
            .route("/{*path}", any(proxy_handler))
            .route("/", any(proxy_handler))
            // Inject compiled router
            .with_state(proxy_router)
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
/// Looks up route and returns status.
async fn proxy_handler(
    State(router): State<Arc<ProxyRouter>>,
    method: Method,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request<Body>,
) -> impl IntoResponse {
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    let path = request.uri().path().to_string();

    tracing::debug!(
        request_id = %request_id,
        method = %method,
        path = %path,
        "Routing request"
    );

    if let Some(route) = router.match_request(&request) {
        tracing::info!(
            request_id = %request_id,
            route_id = %route.id,
            backend_group = %route.backend_group,
            "Route matched"
        );
        
        // Phase 2: Just acknowledge match
        (StatusCode::OK, format!("Forwarding to backend group: {}", route.backend_group))
    } else {
        tracing::warn!(
            request_id = %request_id,
            path = %path,
            "No route matched"
        );
        (StatusCode::NOT_FOUND, "No matching route found".to_string())
    }
}

/// Wait for shutdown signal (Ctrl+C).
async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install Ctrl+C handler");
    tracing::info!("Shutdown signal received");
}
