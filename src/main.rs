//! Rust Production Reverse Proxy (v1)

use std::path::Path;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use reverse_proxy::config::loader::load_config;
use reverse_proxy::config::watcher::ConfigWatcher;
use reverse_proxy::http::HttpServer;
use reverse_proxy::lifecycle::Shutdown;

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

    // Phase 9: Initialize Shutdown coordinator
    let shutdown = Shutdown::new();

    // Phase 8: Load Configuration from file
    let config_path = Path::new("config.toml");
    
    if !config_path.exists() {
        tracing::warn!("config.toml not found, creating default configuration");
        let default_config = reverse_proxy::config::ProxyConfig::default();
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
            reverse_proxy::observability::metrics::init_metrics(addr);
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
    let server_shutdown = shutdown.subscribe();
    
    // Spawn signal handler
    let signal_shutdown = shutdown;
    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                tracing::info!("Shutdown signal received (Ctrl+C)");
                signal_shutdown.trigger();
            }
            Err(err) => {
                tracing::error!("Failed to listen for shutdown signal: {}", err);
            }
        }
    });

    server.run(listener, config_updates, server_shutdown).await?;

    tracing::info!("Graceful shutdown complete. Exiting.");
    Ok(())
}
