//! Shared utilities for integration and load testing.

use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::io::AsyncWriteExt;
use std::time::Duration;
use std::future::Future;

/// Start a simple mock backend that returns a fixed response.
pub async fn start_mock_backend(addr: SocketAddr, response: &'static str) {
    let listener = TcpListener::bind(addr).await.unwrap();
    
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((mut socket, _)) => {
                    tokio::spawn(async move {
                        let response_str = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            response.len(),
                            response
                        );
                        let _ = socket.write_all(response_str.as_bytes()).await;
                        let _ = socket.shutdown().await;
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    });
                }
                Err(_) => break,
            }
        }
    });
}

/// Start a programmable mock backend with async support.
#[allow(dead_code)]
pub async fn start_programmable_backend<F, Fut>(addr: SocketAddr, f: F) 
where 
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = (u16, String)> + Send + 'static
{
    let listener = TcpListener::bind(addr).await.unwrap();
    let f = std::sync::Arc::new(f);
    
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((mut socket, _)) => {
                    let f = f.clone();
                    tokio::spawn(async move {
                        let (status, body) = f().await;
                        let status_text = match status {
                            200 => "200 OK",
                            404 => "404 Not Found",
                            429 => "429 Too Many Requests",
                            500 => "500 Internal Server Error",
                            502 => "502 Bad Gateway",
                            503 => "503 Service Unavailable",
                            _ => "200 OK",
                        };
                        
                        let response_str = format!(
                            "HTTP/1.1 {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                            status_text,
                            body.len(),
                            body
                        );
                        let _ = socket.write_all(response_str.as_bytes()).await;
                        let _ = socket.shutdown().await;
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    });
                }
                Err(_) => break,
            }
        }
    });
}
