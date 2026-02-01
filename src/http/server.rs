//! HTTP server setup and configuration.

use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{Method, Request, StatusCode, header},
    response::{IntoResponse, Response},
    routing::any,
    Router,
    middleware,
    extract::{DefaultBodyLimit, Request as AxumRequest},
};
use alloy::primitives::Address;
use hyper_util::{
    client::legacy::{Client, connect::HttpConnector},
    rt::TokioExecutor,
};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, broadcast};
use tower_http::{
    timeout::TimeoutLayer,
    trace::TraceLayer,
    set_header::SetResponseHeaderLayer,
};
use arc_swap::ArcSwap;
use std::sync::atomic::Ordering;
use axum_server::Handle;

use crate::blockchain::wallet::Wallet;
use crate::blockchain::client::BlockchainClient;
use crate::payments::monitor::PaymentMonitor;
use crate::payments::cache::SubscriptionCache;
use crate::config::ProxyConfig;
use crate::http::request::RequestIdLayer;
use crate::quoting::QuoteEngine;
use crate::routing::Router as ProxyRouter;
use crate::load_balancer::pool::BackendManager;
use crate::health::active::HealthMonitor;
use crate::resilience::retries::{RetryBudget, is_retryable};
use crate::resilience::backoff::calculate_backoff;
use crate::observability::metrics;
use crate::security::rate_limit::{RateLimiterState, rate_limit_middleware};
use crate::security::access_control::{AccessControlState, access_control_middleware};
use crate::security::qos::ConnectionTracker;
use crate::net::tls::load_tls_config;
use crate::admin::setup_admin_router;

/// Internal state that can be swapped atomically.
pub struct InnerState {
    pub config: ProxyConfig,
    pub router: Arc<ProxyRouter>,
    pub backends: Arc<BackendManager>,
    pub retry_budget: Arc<RetryBudget>,
    pub rate_limiter: Option<Arc<RateLimiterState>>,
    pub quote_engine: Option<QuoteEngine>,
    pub subscription_cache: Arc<SubscriptionCache>,
    pub conn_tracker: Arc<ConnectionTracker>,
    pub axum_router: Router<InnerStateWrapper>,
    pub request_count: Arc<std::sync::atomic::AtomicUsize>,
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

#[derive(Clone)]
struct SseGuard {
    tracker: Arc<ConnectionTracker>,
    addr: Address,
}

impl Drop for SseGuard {
    fn drop(&mut self) {
        metrics::record_long_lived_connection("sse", -1);
        self.tracker.decrement(self.addr);
    }
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
                config.qos.clone(),
                config.rate_limit.requests_per_second,
                config.rate_limit.burst_size,
            )))
        } else {
            None
        };

        let conn_tracker = Arc::new(ConnectionTracker::new(config.qos.clone()));

        // Initialize QuoteEngine if blockchain enabled
        let quote_engine = if config.blockchain.enabled {
            match Wallet::from_env(config.blockchain.chain_id) {
                Ok(wallet) => {
                    tracing::info!("Quote engine initialized with wallet");
                    Some(QuoteEngine::new(wallet))
                }
                Err(e) => {
                    tracing::error!("Failed to init wallet for quote engine: {}", e);
                    None
                }
            }
        } else {
            None
        };
        // Initialize Subscription Cache
        let subscription_cache = match SubscriptionCache::load_from_file("subscriptions.json") {
            Ok(cache) => Arc::new(cache),
            Err(e) => {
                tracing::warn!("Failed to load subscription cache: {}. Starting empty.", e);
                Arc::new(SubscriptionCache::new(Some("subscriptions.json".to_string())))
            }
        };

        let request_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let mut axum_router: Router<InnerStateWrapper> = Router::new()
            .route("/api/v1/quote", any(crate::http::quote::create_quote))
            .route("/api/v1/quote/{id}", any(crate::http::quote::get_quote))
            .route("/{*path}", any(proxy_handler))
            .route("/", any(proxy_handler));

        if let Some(ref rl_state) = rate_limiter {
            axum_router = axum_router.layer(middleware::from_fn_with_state(
                rl_state.clone(),
                rate_limit_middleware,
            ));
        }

        // Access Control (Runs before Rate Limit)
        let ac_state = AccessControlState {
            cache: subscription_cache.clone(),
            enabled: config.payments.enabled,
            grace_period_secs: config.payments.grace_period_secs,
        };
        axum_router = axum_router.layer(middleware::from_fn_with_state(
            ac_state,
            access_control_middleware,
        ));

        // Security Hardening (Phase 24)
        if config.security.enable_headers {
            axum_router = axum_router
                .layer(SetResponseHeaderLayer::overriding(
                    header::STRICT_TRANSPORT_SECURITY,
                    header::HeaderValue::from_static("max-age=63072000; includeSubDomains; preload"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    header::X_CONTENT_TYPE_OPTIONS,
                    header::HeaderValue::from_static("nosniff"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    header::X_FRAME_OPTIONS,
                    header::HeaderValue::from_static("DENY"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    header::CONTENT_SECURITY_POLICY,
                    header::HeaderValue::from_static("default-src 'self'; frame-ancestors 'none';"),
                ))
                .layer(SetResponseHeaderLayer::overriding(
                    header::REFERRER_POLICY,
                    header::HeaderValue::from_static("strict-origin-when-cross-origin"),
                ));
        }

        axum_router = axum_router
            .layer(DefaultBodyLimit::max(config.security.max_body_size))
            .layer(TimeoutLayer::with_status_code(
                StatusCode::GATEWAY_TIMEOUT,
                Duration::from_secs(config.timeouts.request_secs),
            ))
            .layer(RequestIdLayer)
            .layer(TraceLayer::new_for_http());

        InnerState {
            config: config.clone(),
            router: proxy_router,
            backends: backend_manager,
            retry_budget,
            rate_limiter,
            quote_engine,
            subscription_cache,
            conn_tracker,
            axum_router,
            request_count,
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
                }
            }
        });

        // Spawn Admin Server if enabled
        let admin_config = self.config.admin.clone();
        if admin_config.enabled {
            let admin_addr: SocketAddr = admin_config.bind_address.parse()?;
            let admin_app_state = AppState {
                client: client.clone(),
                inner: inner_state.clone(),
            };
            let admin_router = setup_admin_router(admin_app_state);
            let mut admin_shutdown = shutdown.resubscribe();
            
            tokio::spawn(async move {
                tracing::info!(address = %admin_addr, "Admin dashboard starting");
                let admin_listener = TcpListener::bind(admin_addr).await.expect("failed to bind admin address");
                axum::serve(admin_listener, admin_router)
                    .with_graceful_shutdown(async move {
                        let _ = admin_shutdown.recv().await;
                        tracing::info!("Admin dashboard initiating graceful shutdown");
                    })
                    .await
                    .expect("admin server error");
            });
        }

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
        // Start Payment Monitor
        if self.config.payments.enabled {
            match BlockchainClient::new(self.config.blockchain.clone()).await {
                Ok(client) => {
                    match PaymentMonitor::new(client, self.config.payments.clone(), inner_state.load().subscription_cache.clone()) {
                        Ok(monitor) => {
                            tracing::info!("Spawning payment monitor task");
                            tokio::spawn(async move {
                                monitor.run().await;
                            });
                        }
                        Err(e) => tracing::error!("Failed to create payment monitor: {}", e),
                    }
                }
                Err(e) => tracing::error!("Failed to create blockchain client for payment monitor: {}", e),
            }
        }

        let app_state = AppState {
            client: client.clone(),
            inner: inner_state.clone(),
        };

        let client_for_fallback = client.clone();
        let inner_state_for_fallback = inner_state.clone();

        // The Master Router delegates every request to the latest inner router
        let app = Router::new()
            .fallback(move |req: AxumRequest| {
                let current_inner = inner_state_for_fallback.load_full();
                let inner_router = current_inner.axum_router.clone();
                let wrapper = InnerStateWrapper {
                    client: client_for_fallback.clone(),
                    inner: current_inner,
                };
                async move {
                    use tower::Service;
                    let mut router = inner_router.with_state(wrapper);
                    let response: Response = router.call(req).await.unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
                    response
                }
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
    request: AxumRequest,
) -> impl IntoResponse {
    let start_time = Instant::now();
    let inner = &wrapper.inner;
    inner.request_count.fetch_add(1, Ordering::Relaxed);
    let config = &inner.config;
    let router = &inner.router;
    let backends = &inner.backends;
    let retry_budget = &inner.retry_budget;
    let retry_config = &config.retries;
    let health_config = &config.health_check;

    let route = match router.match_request(&request) {
        Some(r) => r,
        None => {
            let path = request.uri().path();
            let method = request.method().as_str();
            tracing::warn!(path = %path, "No route matched");
            metrics::record_request(method, 404, "none", start_time);
            return (StatusCode::NOT_FOUND, "No matching route found").into_response();
        }
    };

    let method = request.method().clone();
    let method_str = method.as_str().to_string();
    let (mut parts, body) = request.into_parts();
    let mut body_opt = Some(body);

    let request_id_header = parts
        .headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let path = parts.uri.path().to_string();

    tracing::debug!(
        request_id = %request_id_header,
        method = %method,
        path = %path,
        "Proxying request"
    );

    // WebSocket logic
    if parts.headers.get("upgrade").and_then(|v| v.to_str().ok()).map(|s| s.to_lowercase() == "websocket").unwrap_or(false) {
        use axum::extract::FromRequestParts;
        if let Ok(ws) = axum::extract::ws::WebSocketUpgrade::from_request_parts(&mut parts, &wrapper).await {
            if let Some(backend_guard) = backends.get(&route.backend_group) {
                let mut backend_url = backend_guard.base_url.clone();
                backend_url.set_path(&path);
                if let Some(query) = parts.uri.query() {
                    backend_url.set_query(Some(query));
                }
                // Reconstruct request for handle_ws_upgrade if needed, but it only needs it for extensions.
                let req = Request::from_parts(parts, Body::empty()); 
                return crate::http::websocket::handle_ws_upgrade(ws, backend_url, req, inner.conn_tracker.clone()).await.into_response();
            }
        }
    }

    let body_bytes = if retry_config.enabled && method.is_idempotent() {
        if let Some(b) = body_opt.take() {
            match axum::body::to_bytes(b, 1024 * 1024).await {
                Ok(bytes) => Some(bytes),
                Err(_) => None,
            }
        } else {
            None
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
                tracing::warn!(request_id = %request_id_header, group = %route.backend_group, "No healthy backends");
                metrics::record_request(&method_str, 503, "none", start_time);
                return (StatusCode::SERVICE_UNAVAILABLE, "No healthy backends").into_response();
            }
        };

        let mut req = Request::builder()
            .method(method.clone())
            .version(axum::http::Version::HTTP_11);
        
        if let Some(headers) = req.headers_mut() {
            for (k, v) in parts.headers.iter() {
                headers.insert(k.clone(), v.clone());
            }
            headers.insert("x-request-id", header::HeaderValue::from_str(&request_id_header).unwrap());
            
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

        let mut backend_url = backend_guard.base_url.clone();
        backend_url.set_path(&path);
        if let Some(query) = parts.uri.query() {
            backend_url.set_query(Some(query));
        }
        let backend_addr_str = backend_guard.addr.to_string();

        let req_body = if let Some(ref bytes) = body_bytes {
            Body::from(bytes.clone())
        } else if attempts == 1 {
            body_opt.take().unwrap_or_else(Body::empty)
        } else {
            Body::empty()
        };

        let req = req.uri(backend_url.as_str())
            .body(req_body)
            .unwrap();

        match wrapper.client.request(req).await {
            Ok(response) => {
                let status = response.status();
                let is_sse = response.headers().get("content-type").map(|v| v == "text/event-stream").unwrap_or(false);

                if attempts < max_attempts 
                    && is_retryable(&method, Some(status), false)
                    && retry_budget.can_retry()
                    && !is_sse // Don't retry SSE if it started streaming?
                {
                    let backoff = calculate_backoff(attempts, retry_config.base_delay_ms, retry_config.max_delay_ms);
                    tracing::info!(request_id = %request_id_header, attempt = attempts, delay = ?backoff, status = %status, "Retrying request");
                    tokio::time::sleep(backoff).await;
                    continue;
                }

                metrics::record_request(&method_str, status.as_u16(), &backend_addr_str, start_time);

                // SSE Tracking
                let (mut res_parts, body) = response.into_parts();
                
                if is_sse {
                    // Check UserContext from REQUEST extensions (stored in parts earlier or extracted from Request)
                    if let Some(ctx) = parts.extensions.get::<crate::security::access_control::UserContext>() {
                        if inner.conn_tracker.try_increment(ctx.address, ctx.tier_id) {
                            metrics::record_long_lived_connection("sse", 1);
                            let tracker = inner.conn_tracker.clone();
                            let addr = ctx.address;
                            res_parts.extensions.insert(SseGuard { tracker, addr });
                        } else {
                            metrics::record_rate_limited("sse_limit");
                            return (StatusCode::TOO_MANY_REQUESTS, "SSE connection limit reached").into_response();
                        }
                    }
                }

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

                return Response::from_parts(res_parts, Body::new(body)).into_response();
            }
            Err(e) => {
                tracing::error!(request_id = %request_id_header, attempt = attempts, error = %e, "Upstream error");
                
                if attempts < max_attempts 
                    && is_retryable(&method, None, true)
                    && retry_budget.can_retry()
                {
                    let backoff = calculate_backoff(attempts, retry_config.base_delay_ms, retry_config.max_delay_ms);
                    tracing::info!(request_id = %request_id_header, attempt = attempts, delay = ?backoff, "Retrying after network error");
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
