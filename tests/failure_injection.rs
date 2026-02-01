//! Failure injection tests for the reverse proxy.

use std::net::SocketAddr;
use std::time::Duration;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use reverse_proxy::config::{ProxyConfig, BackendConfig, RouteConfig};
use reverse_proxy::http::HttpServer;
use reverse_proxy::lifecycle::Shutdown;
use axum::http::StatusCode;

mod common;

#[tokio::test]
async fn test_retry_on_failure() {
    // Phase 10: Use unique ports and non-pooled client
    let backend_addr: SocketAddr = "127.0.0.1:28181".parse().unwrap();
    let proxy_addr: SocketAddr = "127.0.0.1:28182".parse().unwrap();
    
    let call_count = Arc::new(AtomicU32::new(0));
    let cc = call_count.clone();
    common::start_programmable_backend(backend_addr, move || {
        let cc = cc.clone();
        async move {
            let count = cc.fetch_add(1, Ordering::SeqCst);
            if count < 2 {
                (503, "Service Unavailable".into())
            } else {
                (200, "Success".into())
            }
        }
    }).await;

    let mut config = ProxyConfig::default();
    config.listener.bind_address = proxy_addr.to_string();
    config.backends.push(BackendConfig {
        name: "b1".into(),
        group: "web".into(),
        address: backend_addr.to_string(),
        weight: 1,
        max_connections: 10,
    });
    config.routes.push(RouteConfig {
        name: "r1".into(),
        host: None,
        path_prefix: Some("/".into()),
        backend_group: "web".into(),
        priority: 0,
    });
    
    // Hardened settings for test stability
    config.retries.enabled = true;
    config.retries.max_attempts = 3;
    config.retries.base_delay_ms = 100;
    config.retries.budget_ratio = 1.0; 
    
    config.health_check.enabled = false;
    config.health_check.unhealthy_threshold = 10; 

    let shutdown = Shutdown::new();
    let (_, config_updates) = mpsc::unbounded_channel();
    let server = HttpServer::new(config);
    let listener = tokio::net::TcpListener::bind(proxy_addr).await.unwrap();
    let server_shutdown = shutdown.subscribe();
    
    tokio::spawn(async move {
        let _ = server.run(listener, config_updates, server_shutdown).await;
    });

    tokio::time::sleep(Duration::from_secs(1)).await;

    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(0)
        .no_proxy()
        .build().unwrap();
    
    let res = client.get(format!("http://{}", proxy_addr)).send().await.expect("Proxy unreachable");
    
    assert_eq!(res.status(), 200, "Should eventually succeed after retries");
    assert!(call_count.load(Ordering::SeqCst) >= 3, "Should have attempted 3 times"); 

    shutdown.trigger();
}

#[tokio::test]
async fn test_health_check_eviction() {
    let b1_addr: SocketAddr = "127.0.0.1:28281".parse().unwrap();
    let b2_addr: SocketAddr = "127.0.0.1:28282".parse().unwrap();
    let proxy_addr: SocketAddr = "127.0.0.1:28283".parse().unwrap();
    
    common::start_mock_backend(b1_addr, "b1").await;
    
    let b2_healthy = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let b2h = b2_healthy.clone();
    common::start_programmable_backend(b2_addr, move || {
        let b2h = b2h.clone();
        async move {
            if b2h.load(Ordering::SeqCst) {
                (200, "b2".into())
            } else {
                (500, "dead".into())
            }
        }
    }).await;

    let mut config = ProxyConfig::default();
    config.listener.bind_address = proxy_addr.to_string();
    config.backends.push(BackendConfig {
        name: "b1".into(),
        group: "web".into(),
        address: b1_addr.to_string(),
        weight: 1,
        max_connections: 10,
    });
    config.backends.push(BackendConfig {
        name: "b2".into(),
        group: "web".into(),
        address: b2_addr.to_string(),
        weight: 1,
        max_connections: 10,
    });
    config.routes.push(RouteConfig {
        name: "r1".into(),
        host: None,
        path_prefix: Some("/".into()),
        backend_group: "web".into(),
        priority: 0,
    });
    
    config.health_check.enabled = true;
    config.health_check.interval_secs = 1;
    config.health_check.unhealthy_threshold = 2;
    config.health_check.healthy_threshold = 1;
    
    config.retries.enabled = false;

    let shutdown = Shutdown::new();
    let (_, config_updates) = mpsc::unbounded_channel();
    let server = HttpServer::new(config);
    let listener = tokio::net::TcpListener::bind(proxy_addr).await.unwrap();
    let server_shutdown = shutdown.subscribe();
    
    tokio::spawn(async move {
        let _ = server.run(listener, config_updates, server_shutdown).await;
    });

    tokio::time::sleep(Duration::from_secs(2)).await;

    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    
    let mut b1_hits = 0;
    let mut b2_hits = 0;
    for _ in 0..20 {
        if let Ok(res) = client.get(format!("http://{}", proxy_addr)).send().await {
            if let Ok(body) = res.text().await {
                if body == "b1" { b1_hits += 1; }
                if body == "b2" { b2_hits += 1; }
            }
        }
    }
    assert!(b1_hits > 0, "b1 should have hits (got {})", b1_hits);
    assert!(b2_hits > 0, "b2 should have hits (got {})", b2_hits);

    b2_healthy.store(false, Ordering::SeqCst);
    
    tokio::time::sleep(Duration::from_secs(3)).await;

    b1_hits = 0;
    b2_hits = 0;
    for _ in 0..10 {
        if let Ok(res) = client.get(format!("http://{}", proxy_addr)).send().await {
            let body = res.text().await.unwrap();
            if body == "b1" { b1_hits += 1; }
            if body == "b2" { b2_hits += 1; }
        }
    }
    assert_eq!(b1_hits, 10, "Only b1 should be hit after b2 eviction");
    assert_eq!(b2_hits, 0, "b2 should have 0 hits after eviction");

    shutdown.trigger();
}

#[tokio::test]
async fn test_max_connections_limit() {
    let backend_addr: SocketAddr = "127.0.0.1:28381".parse().unwrap();
    let proxy_addr: SocketAddr = "127.0.0.1:28384".parse().unwrap();
    
    common::start_programmable_backend(backend_addr, move || {
        async move {
            tokio::time::sleep(Duration::from_millis(200)).await;
            (200, "slow".into())
        }
    }).await;

    let mut config = ProxyConfig::default();
    config.listener.bind_address = proxy_addr.to_string();
    config.backends.push(BackendConfig {
        name: "b1".into(),
        group: "web".into(),
        address: backend_addr.to_string(),
        weight: 1,
        max_connections: 2,
    });
    config.routes.push(RouteConfig {
        name: "r1".into(),
        host: None,
        path_prefix: Some("/".into()),
        backend_group: "web".into(),
        priority: 0,
    });
    config.health_check.enabled = false;
    config.retries.enabled = false;

    let shutdown = Shutdown::new();
    let (_, config_updates) = mpsc::unbounded_channel();
    let server = HttpServer::new(config);
    let listener = tokio::net::TcpListener::bind(proxy_addr).await.unwrap();
    let server_shutdown = shutdown.subscribe();
    
    tokio::spawn(async move {
        let _ = server.run(listener, config_updates, server_shutdown).await;
    });

    tokio::time::sleep(Duration::from_millis(500)).await;

    let client = reqwest::Client::new();
    
    let url = format!("http://{}", proxy_addr);
    let c1_a = client.clone();
    let url_a = url.clone();
    let t1 = tokio::spawn(async move { c1_a.get(&url_a).send().await });
    
    let c1_b = client.clone();
    let url_b = url.clone();
    let t2 = tokio::spawn(async move { c1_b.get(&url_b).send().await });
    
    tokio::time::sleep(Duration::from_millis(50)).await;

    let res3 = client.get(format!("http://{}", proxy_addr)).send().await.unwrap();
    assert_eq!(res3.status(), StatusCode::SERVICE_UNAVAILABLE, "Should be rejected when max_connections hit");

    let _ = t1.await;
    let _ = t2.await;

    shutdown.trigger();
}
