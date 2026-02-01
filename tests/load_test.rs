//! Load testing for the reverse proxy.

use std::net::SocketAddr;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use reverse_proxy::config::{ProxyConfig, BackendConfig, RouteConfig};
use reverse_proxy::http::HttpServer;
use reverse_proxy::lifecycle::Shutdown;

mod common;

#[tokio::test]
async fn test_load_performance() {
    // 1. Setup Mock Backend
    let backend_addr: SocketAddr = "127.0.0.1:8081".parse().unwrap();
    common::start_mock_backend(backend_addr, "Hello from backend").await;

    // 2. Setup Proxy Config
    let proxy_addr: SocketAddr = "127.0.0.1:8082".parse().unwrap();
    let mut config = ProxyConfig::default();
    config.listener.bind_address = proxy_addr.to_string();
    config.backends.push(BackendConfig {
        name: "b1".into(),
        group: "web".into(),
        address: backend_addr.to_string(),
        weight: 1,
        max_connections: 1000,
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

    // 3. Start Proxy
    let shutdown = Shutdown::new();
    let (_, config_updates) = mpsc::unbounded_channel();
    let server = HttpServer::new(config);
    let listener = tokio::net::TcpListener::bind(proxy_addr).await.unwrap();
    let server_shutdown = shutdown.subscribe();
    
    tokio::spawn(async move {
        let _ = server.run(listener, config_updates, server_shutdown).await;
    });

    // Wait for server to start
    tokio::time::sleep(Duration::from_millis(300)).await;

    // 4. Run Load Test
    let concurrency = 20; // Reduced for consistency in debug mode
    let requests_per_task = 50;
    let total_requests = concurrency * requests_per_task;
    
    let client = reqwest::Client::new();
    let start = Instant::now();
    
    let mut tasks = Vec::new();
    for _ in 0..concurrency {
        let client = client.clone();
        let url = format!("http://{}", proxy_addr);
        tasks.push(tokio::spawn(async move {
            let mut latencies = Vec::new();
            for _ in 0..requests_per_task {
                let req_start = Instant::now();
                match client.get(&url).send().await {
                    Ok(res) => {
                        if res.status().is_success() {
                            latencies.push(req_start.elapsed());
                        } else {
                            // log error
                        }
                    }
                    Err(_) => {
                        // log error
                    }
                }
            }
            latencies
        }));
    }

    let mut all_latencies = Vec::new();
    for task in tasks {
        let latencies = task.await.unwrap();
        all_latencies.extend(latencies);
    }

    let duration = start.elapsed();
    let rps = total_requests as f64 / duration.as_secs_f64();
    
    if all_latencies.is_empty() {
        panic!("No successful requests recorded");
    }

    all_latencies.sort();
    let p50 = all_latencies[all_latencies.len() / 2];
    let p95 = all_latencies[(all_latencies.len() as f64 * 0.95) as usize];
    let p99 = all_latencies[(all_latencies.len() as f64 * 0.99) as usize];

    println!("\n--- Load Test Results ---");
    println!("Total Requests: {}", total_requests);
    println!("Concurrency:    {}", concurrency);
    println!("Total Duration: {:?}", duration);
    println!("Requests/sec:   {:.2}", rps);
    println!("P50 Latency:    {:?}", p50);
    println!("P95 Latency:    {:?}", p95);
    println!("P99 Latency:    {:?}", p99);
    println!("Success Rate:   {}/{}", all_latencies.len(), total_requests);
    println!("-------------------------\n");

    shutdown.trigger();
}
