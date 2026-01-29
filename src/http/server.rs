//! HTTP server setup and configuration.

use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{Method, Request, StatusCode, Uri, header},
    response::{IntoResponse, Response},
    routing::any,
    Router,
    middleware,
    extract::DefaultBodyLimit,
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
use tokio::sync::{mpsc, broadcast};
use tower_http::{
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use arc_swap::ArcSwap;
use tower::Service;
use axum_server::Handle;

use crate::config::ProxyConfig;
use crate::http::request::RequestIdLayer;
use crate::routing::Router as ProxyRouter;
use crate::load_balancer::pool::BackendManager;
use crate::health::active::HealthMonitor;
use crate::resilience::retries::{RetryBudget, is_retryable};
use crate::resilience::backoff::calculate_backoff;
use crate::observability::metrics;
use crate::security::rate_limit::{RateLimiterState, rate_limit_middleware};
use crate::net::tls::load_tls_config;

/// Internal state that can be swapped atomically.
pub struct InnerState {
    pub config: ProxyConfig,
    pub router: Arc<ProxyRouter>,
    pub backends: Arc<BackendManager>,
    pub retry_budget: Arc<RetryBudget>,
    pub rate_limiter: Option<Arc<RateLimiterState>>,
    pub axum_router: Router<InnerStateWrapper>, 
}

/// A wrapper to allow and inject State into the inner router
#[derive(Clone)]
pub struct InnerStateWrapper {
    pub client: Client<HttpConnector, Body>,
    pub inner: Arc<InnerState>,
}

/// Application state injected into the master (fallback) router.
#[derive(Clone)]
pub struct AppState {
    pub client: Client<HttpConnector, Body>,
    pub inner: Arc<ArcSwap<InnerState>>,
}

/// HTTP server for the reverse proxy.
pub struct HttpServer {
    config: ProxyConfig,
    inner_state: Arc<ArcSwap<InnerState>>,
    client: Client<HttpConnector, Body>,
}

impl HttpServer {
    /// Create a new HTTP server with the given configuration.
    pub fn new(config: ProxyConfig) -> Self {
        let client = Client::builder(TokioExecutor::new())
            .build(HttpConnector::new());

        let inner = Self::build_inner(&config, client.clone());
        let inner_state = Arc::new(ArcSwap::from_pointee(inner));

        Self { 
            config,
            inner_state,
            client,
        }
    }

    /// Build the internal state from a configuration.
    fn build_inner(config: &ProxyConfig, _client: Client<HttpConnector, Body>) -> InnerState {
        let proxy_router = Arc::new(ProxyRouter::from_config(config.routes.clone()));
        let backend_manager = Arc::new(BackendManager::new(config.backends.clone()));
        let retry_budget = Arc::new(RetryBudget::new(config.retries.budget_ratio, 100));
        
        let rate_limiter = if config.rate_limit.enabled {
            Some(Arc::new(RateLimiterState::new(
                config.rate_limit.requests_per_second,
                config.rate_limit.burst_size,
            )))
        } else {
            None
        };

        let mut axum_router = Router::new()
            .route("/{*path}", any(proxy_handler))
            .route("/", any(proxy_handler));

        if let Some(ref rl_state) = rate_limiter {
            axum_router = axum_router.layer(middleware::from_fn_with_state(
                rl_state.clone(),
                rate_limit_middleware,
            ));
        }

        axum_router = axum_router
            .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
            .layer(TimeoutLayer::new(Duration::from_secs(config.timeouts.request_secs)))
            .layer(RequestIdLayer)
            .layer(TraceLayer::new_for_http());

        InnerState {
            config: config.clone(),
            router: proxy_router,
            backends: backend_manager,
            retry_budget,
            rate_limiter,
            axum_router,
        }
    }

    /// Run the server, accepting connections on the given listener.
    pub async fn run(
        self, 
        listener: TcpListener, 
        mut config_updates: mpsc::UnboundedReceiver<ProxyConfig>,
        mut shutdown: broadcast::Receiver<()>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let addr = listener.local_addr()?;
        tracing::info!(
            address = %addr,
            "HTTP server starting"
        );

        let inner_state = self.inner_state.clone();
        let client = self.client.clone();
        
        // Spawn Reloader Task
        let reloader_inner = inner_state.clone();
        let reloader_client = client.clone();
        let mut reloader_shutdown = shutdown.resubscribe();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(new_config) = config_updates.recv() => {
                        tracing::info!("Applying new configuration...");
                        let new_inner = Self::build_inner(&new_config, reloader_client.clone());
                        reloader_inner.store(Arc::new(new_inner));
                        tracing::info!("Configuration reload complete");
                    }
                    _ = reloader_shutdown.recv() => {
                        tracing::info!("Config reloader received shutdown signal, exiting loop");
                        break;
                    }
                    else => break,
                }
            }
        });

        if self.config.health_check.enabled {
            let monitor = HealthMonitor::new(
                inner_state.load().backends.clone(), 
                self.config.health_check.clone()
            );
            let monitor_shutdown = shutdown.resubscribe();
            tokio::spawn(async move {
                monitor.run(monitor_shutdown).await;
            });
        }

        let app_state = AppState {
            client: client.clone(),
            inner: inner_state.clone(),
        };

        // The Master Router delegates every request to the latest inner router
        let app = Router::new()
            .fallback(|State(s): State<AppState>, req: Request<Body>| async move {
                let current_inner = s.inner.load_full();
                let inner_router = current_inner.axum_router.clone();
                let wrapper = InnerStateWrapper {
                    client: s.client.clone(),
                    inner: current_inner,
                };
                inner_router.with_state(wrapper).call(req).await.into_response()
            })
            .with_state(app_state)
            .into_make_service_with_connect_info::<SocketAddr>();

        if let Some(ref tls_config) = self.config.listener.tls {
            tracing::info!("TLS enabled, loading certificates");
            let cert_path = std::path::Path::new(&tls_config.cert_path);
            let key_path = std::path::Path::new(&tls_config.key_path);
            let tls_config = load_tls_config(cert_path, key_path).await?;
            
            let handle = Handle::new();
            let mut https_shutdown = shutdown.resubscribe();
            let h = handle.clone();
            
            tokio::spawn(async move {
                let _ = https_shutdown.recv().await;
                tracing::info!("HTTPS server initiating graceful shutdown");
                // Deadline for 10 seconds
                h.graceful_shutdown(Some(Duration::from_secs(10)));
            });

            axum_server::from_tcp_rustls(listener.into_std()?, tls_config)
                .handle(handle)
                .serve(app)
                .await?;
        } else {
            axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    let _ = shutdown.recv().await;
                    tracing::info!("HTTP server initiating graceful shutdown");
                })
                .await?;
        }

        tracing::info!("HTTP server stopped");
        Ok(())
    }
}

/// Proxy handler using InnerStateWrapper
async fn proxy_handler(
    State(wrapper): State<InnerStateWrapper>,
    ConnectInfo(client_addr): ConnectInfo<SocketAddr>,
    request: Request<Body>,
) -> impl IntoResponse {
    let start_time = Instant::now();
    let inner = &wrapper.inner;
    let config = &inner.config;
    let router = &inner.router;
    let backends = &inner.backends;
    let retry_budget = &inner.retry_budget;
    let retry_config = &config.retries;
    let health_config = &config.health_check;

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

    let route = match router.match_request(&request) {
        Some(r) => r,
        None => {
            tracing::warn!(request_id = %request_id, path = %path, "No route matched");
            metrics::record_request(&method_str, 404, "none", start_time);
            return (StatusCode::NOT_FOUND, "No matching route found").into_response();
        }
    };

    let (parts, body) = request.into_parts();
    let body_bytes = if retry_config.enabled && method.is_idempotent() {
        match axum::body::to_bytes(body, 1024 * 1024).await {
            Ok(bytes) => Some(bytes),
            Err(_) => None,
        }
    } else {
        None
    };

    retry_budget.record_request();

    let mut attempts = 0;
    let max_attempts = if retry_config.enabled && (body_bytes.is_some() || method == Method::GET || method == Method::HEAD) {
        retry_config.max_attempts
    } else {
        1
    };

    loop {
        attempts += 1;
        
        let backend_guard = match backends.get(&route.backend_group) {
            Some(g) => g,
            None => {
                tracing::warn!(request_id = %request_id, group = %route.backend_group, "No healthy backends");
                metrics::record_request(&method_str, 503, "none", start_time);
                return (StatusCode::SERVICE_UNAVAILABLE, "No healthy backends").into_response();
            }
        };

        let mut req = Request::builder()
            .method(method.clone())
            .version(parts.version);
        
        if let Some(headers) = req.headers_mut() {
            for (k, v) in parts.headers.iter() {
                headers.insert(k.clone(), v.clone());
            }
            headers.insert("x-request-id", header::HeaderValue::from_str(&request_id).unwrap());
            
            let client_ip = client_addr.ip().to_string();
            if let Some(existing) = headers.get("x-forwarded-for") {
                if let Ok(s) = existing.to_str() {
                    let new_val = format!("{}, {}", s, client_ip);
                    if let Ok(hv) = header::HeaderValue::from_str(&new_val) {
                        headers.insert("x-forwarded-for", hv);
                    }
                }
            } else {
                headers.insert("x-forwarded-for", header::HeaderValue::from_str(&client_ip).unwrap());
            }
            headers.insert("x-forwarded-proto", header::HeaderValue::from_static("http"));
            if let Some(host) = parts.headers.get("host") {
                headers.insert("x-forwarded-host", host.clone());
            }
        }

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

        match wrapper.client.request(req).await {
            Ok(response) => {
                let status = response.status();
                
                if attempts < max_attempts 
                    && is_retryable(&method, Some(status), false)
                    && retry_budget.can_retry()
                {
                    let backoff = calculate_backoff(attempts, retry_config.base_delay_ms, retry_config.max_delay_ms);
                    tracing::info!(request_id = %request_id, attempt = attempts, delay = ?backoff, status = %status, "Retrying request");
                    tokio::time::sleep(backoff).await;
                    continue;
                }

                metrics::record_request(&method_str, status.as_u16(), &backend_addr_str, start_time);

                if status.is_server_error() {
                    match status {
                        StatusCode::BAD_GATEWAY | StatusCode::SERVICE_UNAVAILABLE | StatusCode::GATEWAY_TIMEOUT => {
                            backend_guard.mark_failure(health_config.unhealthy_threshold as usize);
                        }
                        _ => {
                            backend_guard.mark_success(health_config.healthy_threshold as usize);
                        }
                    }
                } else {
                    backend_guard.mark_success(health_config.healthy_threshold as usize);
                }

                let (parts, body) = response.into_parts();
                return Response::from_parts(parts, Body::new(body)).into_response();
            }
            Err(e) => {
                tracing::error!(request_id = %request_id, attempt = attempts, error = %e, "Upstream error");
                
                if attempts < max_attempts 
                    && is_retryable(&method, None, true)
                    && retry_budget.can_retry()
                {
                    let backoff = calculate_backoff(attempts, retry_config.base_delay_ms, retry_config.max_delay_ms);
                    tracing::info!(request_id = %request_id, attempt = attempts, delay = ?backoff, "Retrying after network error");
                    tokio::time::sleep(backoff).await;
                    continue;
                }

                metrics::record_request(&method_str, 502, &backend_addr_str, start_time);

                backend_guard.mark_failure(health_config.unhealthy_threshold as usize);
                return (StatusCode::BAD_GATEWAY, "Upstream request failed").into_response();
            }
        }
    }
}
