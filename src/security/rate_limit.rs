//! IP-based rate limiting middleware.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::net::SocketAddr;

/// A simple token bucket rate limiter.
struct TokenBucket {
    tokens: f64,
    last_update: Instant,
}

impl TokenBucket {
    fn new(capacity: f64) -> Self {
        Self {
            tokens: capacity,
            last_update: Instant::now(),
        }
    }

    fn try_acquire(&mut self, capacity: f64, refill_rate: f64) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_secs_f64();
        
        // Refill tokens
        self.tokens = (self.tokens + elapsed * refill_rate).min(capacity);
        self.last_update = now;

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

/// State for the IP rate limiter.
pub struct RateLimiterState {
    buckets: Mutex<HashMap<IpAddr, TokenBucket>>,
    rps: f64,
    burst: f64,
}

impl RateLimiterState {
    pub fn new(rps: u32, burst: u32) -> Self {
        Self {
            buckets: Mutex::new(HashMap::new()),
            rps: rps as f64,
            burst: burst as f64,
        }
    }

    fn check(&self, ip: IpAddr) -> bool {
        let mut buckets = self.buckets.lock().unwrap();
        let bucket = buckets
            .entry(ip)
            .or_insert_with(|| TokenBucket::new(self.burst));
        
        bucket.try_acquire(self.burst, self.rps)
    }
}

/// Middleware function for IP-based rate limiting.
pub async fn rate_limit_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    state: axum::extract::State<Arc<RateLimiterState>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    if state.check(addr.ip()) {
        next.run(request).await
    } else {
        tracing::warn!(ip = %addr.ip(), "Rate limit exceeded");
        let mut response = Response::new(Body::from("Rate limit exceeded"));
        *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
        response
    }
}
