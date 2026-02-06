//! Rate limiting middleware with tiered QoS.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::net::SocketAddr;

use crate::config::QosConfig;
use crate::security::access_control::UserContext;
use crate::observability::metrics;

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

/// State for the Tiered Rate Limiter.
pub struct RateLimiterState {
    buckets: Mutex<HashMap<String, TokenBucket>>,
    config: QosConfig,
    // Fallback/Default limits
    default_rps: f64,
    default_burst: f64,
}

impl RateLimiterState {
    pub fn new(qos_config: QosConfig, default_rps: u32, default_burst: u32) -> Self {
        Self {
            buckets: Mutex::new(HashMap::new()),
            config: qos_config,
            default_rps: default_rps as f64,
            default_burst: default_burst as f64,
        }
    }

    fn check(&self, key: String, tier_id: Option<u8>) -> bool {
        let (rps, burst) = match tier_id {
            Some(1) => (self.config.tier_1_rps as f64, self.config.tier_1_rps as f64 * 2.0), // Burst 2x RPS
            Some(2) => (self.config.tier_2_rps as f64, self.config.tier_2_rps as f64 * 2.0),
            Some(3) => (self.config.tier_3_rps as f64, self.config.tier_3_rps as f64 * 2.0),
            _ => (self.default_rps, self.default_burst),
        };

        let mut buckets = self.buckets.lock().expect("rate limiter mutex poisoned");
        let bucket = buckets
            .entry(key)
            .or_insert_with(|| TokenBucket::new(burst));
        
        bucket.try_acquire(burst, rps)
    }
}

/// Middleware function for tiered rate limiting.
pub async fn rate_limit_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<Arc<RateLimiterState>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    // Check for authenticated UserContext
    let (key, tier_id) = if let Some(ctx) = request.extensions().get::<UserContext>() {
        (ctx.address.to_string(), Some(ctx.tier_id))
    } else {
        (addr.ip().to_string(), None)
    };

    if state.check(key.clone(), tier_id) {
        next.run(request).await
    } else {
        tracing::warn!(client = %key, tier = ?tier_id, "Rate limit exceeded");
        metrics::record_rate_limited("rps_limit");
        let mut response = Response::new(Body::from("Rate limit exceeded"));
        *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
        response
    }
}
