//! Rust Production Reverse Proxy (v1)
//!
//! A production-ready reverse proxy built with Tokio and Axum.
//!
//! # Architecture Overview
//!
//! ```text
//!                              ┌─────────────────────────────────────────────────────────┐
//!                              │                    REVERSE PROXY                         │
//!                              │                                                          │
//!     Client Request           │  ┌─────────┐    ┌─────────┐    ┌──────────────┐         │
//!     ─────────────────────────┼─▶│   net   │───▶│  http   │───▶│   routing    │         │
//!                              │  │listener │    │ server  │    │   engine     │         │
//!                              │  └─────────┘    └─────────┘    └──────┬───────┘         │
//!                              │                                       │                  │
//!                              │                                       ▼                  │
//!                              │                               ┌──────────────┐          │
//!                              │                               │load_balancer │          │
//!                              │                               │   + pool     │          │
//!                              │                               └──────┬───────┘          │
//!                              │                                       │                  │
//!                              │                                       ▼                  │
//!     Client Response          │  ┌─────────┐    ┌─────────┐    ┌──────────────┐         │
//!     ◀────────────────────────┼──│response │◀───│ http    │◀───│   backend    │◀────────┼──── Backend
//!                              │  │transform│    │ client  │    │  connection  │         │     Server
//!                              │  └─────────┘    └─────────┘    └──────────────┘         │
//!                              │                                                          │
//!                              │  ┌────────────────────────────────────────────────────┐ │
//!                              │  │              Cross-Cutting Concerns                 │ │
//!                              │  │  ┌─────────┐ ┌────────┐ ┌──────────┐ ┌───────────┐ │ │
//!                              │  │  │ config  │ │ health │ │observa-  │ │ security  │ │ │
//!                              │  │  │         │ │ checks │ │ bility   │ │ + limits  │ │ │
//!                              │  │  └─────────┘ └────────┘ └──────────┘ └───────────┘ │ │
//!                              │  │  ┌─────────────────┐  ┌─────────────────────────┐  │ │
//!                              │  │  │   resilience    │  │       lifecycle         │  │ │
//!                              │  │  │timeout/retry/cb │  │   startup/shutdown      │  │ │
//!                              │  │  └─────────────────┘  └─────────────────────────┘  │ │
//!                              │  └────────────────────────────────────────────────────┘ │
//!                              └─────────────────────────────────────────────────────────┘
//! ```
//!
//! # Phase 1 Status
//!
//! Implements network foundation:
//! - TCP listener with connection limits
//! - HTTP/1.1 and HTTP/2 via Axum
//! - Request ID generation (UUID v4)
//! - Request timeout and body limits
//! - Echo handler (no routing yet)

// Core subsystems
pub mod config;
pub mod http;
pub mod net;
pub mod routing;

// Traffic management
pub mod health;
pub mod load_balancer;

// Cross-cutting concerns
pub mod lifecycle;
pub mod observability;
pub mod resilience;
pub mod security;

use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::ProxyConfig;
use crate::http::HttpServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing subscriber
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "reverse_proxy=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("reverse-proxy v0.1.0 starting");

    // Load configuration (using defaults for Phase 1)
    let config = ProxyConfig::default();

    tracing::info!(
        bind_address = %config.listener.bind_address,
        max_connections = config.listener.max_connections,
        request_timeout_secs = config.timeouts.request_secs,
        "Configuration loaded"
    );

    // Bind TCP listener
    let listener = TcpListener::bind(&config.listener.bind_address).await?;
    let local_addr = listener.local_addr()?;

    tracing::info!(
        address = %local_addr,
        "Listening for connections"
    );

    // Initialize metrics server (Phase 6)
    if config.observability.metrics_enabled {
        if let Ok(addr) = config.observability.metrics_address.parse() {
            crate::observability::metrics::init_metrics(addr);
        } else {
            tracing::error!(
                metrics_address = %config.observability.metrics_address,
                "Failed to parse metrics address"
            );
        }
    }

    // Create and run HTTP server
    let server = HttpServer::new(config);
    server.run(listener).await?;

    tracing::info!("Shutdown complete");
    Ok(())
}
