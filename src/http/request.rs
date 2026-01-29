//! Request handling and transformation.
//!
//! # Responsibilities
//! - Generate unique request ID (UUID v4)
//! - Middleware to add X-Request-ID header
//! - Extract request metadata for logging

use axum::http::{HeaderName, HeaderValue, Request, Response};
use std::task::{Context, Poll};
use tower::{Layer, Service};
use uuid::Uuid;

/// Header name for request ID.
pub static X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

/// A unique identifier for each request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RequestId(Uuid);

impl RequestId {
    /// Generate a new unique request ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Get the UUID value.
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Layer that adds request ID to requests and responses.
#[derive(Debug, Clone, Copy, Default)]
pub struct RequestIdLayer;

impl<S> Layer<S> for RequestIdLayer {
    type Service = RequestIdMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequestIdMiddleware { inner }
    }
}

/// Middleware that adds request ID to requests and responses.
#[derive(Debug, Clone)]
pub struct RequestIdMiddleware<S> {
    inner: S,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for RequestIdMiddleware<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send,
    ReqBody: Send + 'static,
    ResBody: Send + 'static,
{
    type Response = Response<ResBody>;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut request: Request<ReqBody>) -> Self::Future {
        // Check for existing request ID or generate new one
        let request_id = request
            .headers()
            .get(&X_REQUEST_ID)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| Uuid::parse_str(s).ok())
            .map(RequestId)
            .unwrap_or_else(RequestId::new);

        // Add request ID to request headers (for downstream)
        if let Ok(value) = HeaderValue::from_str(&request_id.to_string()) {
            request.headers_mut().insert(X_REQUEST_ID.clone(), value);
        }

        // Store request ID in extensions for handlers to access
        request.extensions_mut().insert(request_id);

        let mut inner = self.inner.clone();

        Box::pin(async move {
            let mut response = inner.call(request).await?;

            // Add request ID to response headers
            if let Ok(value) = HeaderValue::from_str(&request_id.to_string()) {
                response.headers_mut().insert(X_REQUEST_ID.clone(), value);
            }

            Ok(response)
        })
    }
}

/// Extension trait to extract request ID from request.
pub trait RequestIdExt {
    /// Get the request ID from extensions.
    fn request_id(&self) -> Option<RequestId>;
}

impl<B> RequestIdExt for Request<B> {
    fn request_id(&self) -> Option<RequestId> {
        self.extensions().get::<RequestId>().copied()
    }
}
