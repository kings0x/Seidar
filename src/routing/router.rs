//! Route lookup and dispatch.
//!
//! # Responsibilities
//! - Store compiled routes
//! - Look up matching route for request
//! - Return matched route or explicit no-match
//!
//! # Design Decisions
//! - Immutable after construction (thread-safe without locks)
//! - O(1) host lookup via HashMap for efficiency
//! - O(n) path prefix scan within host bucket (ordered by priority)
//! - Explicit NoMatch rather than silent default

use std::collections::HashMap;
use std::sync::Arc;
use axum::http::Request;
use axum::body::Body;
use crate::config::RouteConfig;
use crate::routing::matcher::{Matcher, HostMatcher, PathPrefixMatcher, AndMatcher};

/// A compiled route ready for matching.
#[derive(Debug)]
pub struct Route {
    pub id: String,
    pub matcher: Box<dyn Matcher>,
    pub backend_group: String,
    pub priority: u32,
}

/// The main router that holds all routes.
#[derive(Debug, Default)]
pub struct Router {
    /// Optimization: Map host -> Routes.
    /// Key "" (empty string) holds routes without host requirements (wildcard host).
    routes_by_host: HashMap<String, Vec<Arc<Route>>>,
}

impl Router {
    /// Create a new, empty router.
    pub fn new() -> Self {
        Self::default()
    }

    /// Compile a list of RouteConfigs into an optimized Router.
    pub fn from_config(configs: Vec<RouteConfig>) -> Self {
        let mut routes_by_host: HashMap<String, Vec<Arc<Route>>> = HashMap::new();

        for config in configs {
            let host_key: String = config.host.clone().map(|h: String| h.to_lowercase()).unwrap_or_default();
            
            // Build the matcher chain
            let mut matchers: Vec<Box<dyn Matcher>> = Vec::new();

            if let Some(host) = &config.host {
                let h: String = host.clone();
                matchers.push(Box::new(HostMatcher::new(h)));
            }

            if let Some(path) = &config.path_prefix {
                let p: String = path.clone();
                matchers.push(Box::new(PathPrefixMatcher::new(p)));
            }

            // In Phase 2, we just treat the combination as an AND match
            // Even if there's only 1 matcher, wrapping it works fine, or we could unwrap 
            // if single. For simplicity:
            let matcher: Box<dyn Matcher> = if matchers.len() == 1 {
                matchers.pop().unwrap()
            } else if matchers.is_empty() {
                 // Match everything if no conditions
                 Box::new(PathPrefixMatcher::new("/")) 
            } else {
                Box::new(AndMatcher::new(matchers))
            };

            let route = Arc::new(Route {
                id: config.name,
                matcher,
                backend_group: config.backend_group,
                priority: config.priority,
            });

            routes_by_host.entry(host_key).or_default().push(route);
        }

        // Sort each bucket by priority (descending)
        for routes in routes_by_host.values_mut() {
            routes.sort_by(|a, b| b.priority.cmp(&a.priority));
        }

        Self { routes_by_host }
    }

    /// Find the best matching route for a request.
    pub fn match_request(&self, req: &Request<Body>) -> Option<Arc<Route>> {
        // 1. Check exact host match
        if let Some(host_header) = req.headers().get("host").and_then(|h| h.to_str().ok()) {
            let host_lower = host_header.to_lowercase();
            // Note: In a real proxy we might want to strip port from host header if present
            // ignoring port stripping for Phase 2 simplicity unless needed.
            
            // Check for routes bound to this specific host
            if let Some(routes) = self.routes_by_host.get(&host_lower) {
                 for route in routes {
                     if route.matcher.matches(req) {
                         return Some(route.clone());
                     }
                 }
            }
        }

        // 2. Check wildcard host routes
        if let Some(routes) = self.routes_by_host.get("") {
            for route in routes {
                if route.matcher.matches(req) {
                    return Some(route.clone());
                }
            }
        }

        None
    }
}
