use axum::{routing::get, Router};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(|| async { "Hello from the pretend website! 🎈" }))
                           .route("/status", get(|| async { "Backend is healthy! ✅" }));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8081));
    println!("Pretend Website is listening on http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
