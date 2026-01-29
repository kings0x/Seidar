//! Rust Production Reverse Proxy (v1)

pub mod config;
pub mod http;
pub mod net;
pub mod routing;
pub mod health;
pub mod load_balancer;
pub mod lifecycle;
pub mod observability;
pub mod resilience;
pub mod security;

use std::path::Path;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::config::loader::load_config;
use crate::config::watcher::ConfigWatcher;
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

    // Phase 8: Load Configuration from file
    let config_path = Path::new("config.toml");
    
    // Create a default config if it doesn't exist (for easier first run)
    if !config_path.exists() {
        tracing::warn!("config.toml not found, creating default configuration");
        let default_config = crate::config::ProxyConfig::default();
        let toml_string = toml::to_string_pretty(&default_config)?;
        std::fs::write(config_path, toml_string)?;
    }

    let config = match load_config(config_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            tracing::error!("Failed to load initial configuration: {}. Exiting.", e);
            std::process::exit(1);
        }
    };

    tracing::info!(
        bind_address = %config.listener.bind_address,
        max_connections = config.listener.max_connections,
        "Configuration loaded"
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

    // Start Configuration Watcher (Phase 8)
    let (watcher_instance, config_updates) = ConfigWatcher::new(config_path);
    let _watcher = watcher_instance.run()?;

    // Bind TCP listener
    let listener = TcpListener::bind(&config.listener.bind_address).await?;
    let local_addr = listener.local_addr()?;

    tracing::info!(
        address = %local_addr,
        "Listening for connections"
    );

    // Create and run HTTP server
    let server = HttpServer::new(config);
    server.run(listener, config_updates).await?;

    tracing::info!("Shutdown complete");
    Ok(())
}
