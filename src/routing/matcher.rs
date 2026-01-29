//! Route matching logic.
//!
//! # Responsibilities
//! - Match host header (exact match, case-insensitive)
//! - Match path prefix (case-sensitive)
//! - Combine conditions with AND semantics
//!
//! # Design Decisions
//! - Host matching is case-insensitive (per HTTP spec)
//! - Path matching is case-sensitive
//! - Empty condition = always matches (wildcard)
//! - No regex to guarantee O(n) matching

use axum::http::Request;
use axum::body::Body;

/// Trait for matching requests against conditions.
pub trait Matcher: Send + Sync + std::fmt::Debug {
    /// Returns true if the request matches this condition.
    fn matches(&self, req: &Request<Body>) -> bool;
}

/// Matches the Host header.
#[derive(Debug, Clone)]
pub struct HostMatcher {
    expected_host: String,
}

impl HostMatcher {
    /// Create a new host matcher.
    /// The host is normalized to lowercase for case-insensitive matching.
    pub fn new(host: impl Into<String>) -> Self {
        Self {
            expected_host: host.into().to_lowercase(),
        }
    }
}

impl Matcher for HostMatcher {
    fn matches(&self, req: &Request<Body>) -> bool {
        req.headers()
            .get("host")
            .and_then(|h| h.to_str().ok())
            .map(|h| h.to_lowercase() == self.expected_host)
            .unwrap_or(false)
    }
}

/// Matches the request path prefix.
#[derive(Debug, Clone)]
pub struct PathPrefixMatcher {
    prefix: String,
}

impl PathPrefixMatcher {
    /// Create a new path prefix matcher.
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }
}

impl Matcher for PathPrefixMatcher {
    fn matches(&self, req: &Request<Body>) -> bool {
        req.uri().path().starts_with(&self.prefix)
    }
}

/// Combines multiple matchers with AND semantics.
#[derive(Debug)]
pub struct AndMatcher {
    matchers: Vec<Box<dyn Matcher>>,
}

// Manual implementation of Clone because Box<dyn Matcher> doesn't derive Clone easily
// For Phase 2, we won't strictly need Clone for the runtime Router, but it's good practice.
// However, cloning trait objects requires a bit more boilerplate. 
// For simplicity in this phase, we'll avoid `Clone` on the trait object for now
// or implement a workaround if needed. Since Router is usually shared via Arc,
// deep cloning isn't strictly necessary.
// removing Clone derive from AndMatcher to avoid complexity.

impl AndMatcher {
    pub fn new(matchers: Vec<Box<dyn Matcher>>) -> Self {
        Self { matchers }
    }
}

impl Matcher for AndMatcher {
    fn matches(&self, req: &Request<Body>) -> bool {
        // All matchers must pass (AND)
        self.matchers.iter().all(|m| m.matches(req))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_matcher() {
        let matcher = HostMatcher::new("example.com");
        
        let req1 = Request::builder()
            .header("Host", "example.com")
            .body(Body::default())
            .unwrap();
        assert!(matcher.matches(&req1));

        let req2 = Request::builder()
            .header("Host", "EXAMPLE.COM")
            .body(Body::default())
            .unwrap();
        assert!(matcher.matches(&req2)); // Case insensitive

        let req3 = Request::builder()
            .header("Host", "other.com")
            .body(Body::default())
            .unwrap();
        assert!(!matcher.matches(&req3));
    }

    #[test]
    fn test_path_matcher() {
        let matcher = PathPrefixMatcher::new("/api");

        let req1 = Request::builder()
            .uri("http://example.com/api/v1")
            .body(Body::default())
            .unwrap();
        assert!(matcher.matches(&req1));

        let req2 = Request::builder()
            .uri("http://example.com/images")
            .body(Body::default())
            .unwrap();
        assert!(!matcher.matches(&req2));
    }
}
