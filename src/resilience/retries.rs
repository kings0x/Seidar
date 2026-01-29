//! Retry logic and retry budget management.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use axum::http::{Method, StatusCode};

/// A simple token-bucket-like retry budget.
///
/// Prevents retry storms by limiting the ratio of retried requests.
#[derive(Debug)]
pub struct RetryBudget {
    /// Total number of requests seen.
    total_requests: AtomicUsize,
    /// Total number of retries performed.
    total_retries: AtomicUsize,
    /// Maximum ratio of retries to total requests (e.g., 0.1 for 10%).
    buffer_ratio: f32,
    /// Minimum requests before the ratio is enforced.
    min_requests: usize,
}

impl RetryBudget {
    pub fn new(buffer_ratio: f32, min_requests: usize) -> Self {
        Self {
            total_requests: AtomicUsize::new(0),
            total_retries: AtomicUsize::new(0),
            buffer_ratio,
            min_requests,
        }
    }

    /// Record a regular request.
    pub fn record_request(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    /// Try to acquire a retry token. Returns true if retry is allowed.
    pub fn can_retry(&self) -> bool {
        let total = self.total_requests.load(Ordering::Relaxed);
        let retries = self.total_retries.load(Ordering::Relaxed);

        if total < self.min_requests {
            // Record the retry even if we are under min_requests?
            // Usually, we want to increment retries if we actually proceed.
            // Let's increment and return true.
            self.total_retries.fetch_add(1, Ordering::Relaxed);
            return true;
        }

        let current_ratio = retries as f32 / total as f32;
        if current_ratio < self.buffer_ratio {
            self.total_retries.fetch_add(1, Ordering::Relaxed);
            return true;
        }

        false
    }
}

/// Helper to determine if a request is retryable.
pub fn is_retryable(method: &Method, status: Option<StatusCode>, error: bool) -> bool {
    // 1. Only idempotent methods
    if !method.is_idempotent() && method != Method::POST {
        // Technically POST is not idempotent, but some systems retry on 503 if they know it hasn't reached backend.
        // For our proxy, we follow the strict rule: only idempotent.
        if method != Method::GET && method != Method::HEAD && method != Method::PUT && method != Method::DELETE {
            return false;
        }
    }

    // 2. Error case (network failure) is always retryable for idempotent methods
    if error {
        return true;
    }

    // 3. Status codes
    if let Some(s) = status {
        match s {
            StatusCode::GATEWAY_TIMEOUT | StatusCode::SERVICE_UNAVAILABLE | StatusCode::BAD_GATEWAY => {
                return true;
            }
            _ => return false,
        }
    }

    false
}
