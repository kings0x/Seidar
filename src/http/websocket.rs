//! WebSocket proxy handling.

use axum::{
    body::Body,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use std::sync::Arc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::{self, Message as TgMessage};
use tracing::{error, info, warn};
use url::Url;

use crate::observability::metrics;

use crate::security::qos::ConnectionTracker;
use crate::security::access_control::UserContext;

/// Handles a WebSocket upgrade request and proxies it to the backend.
pub async fn handle_ws_upgrade(
    ws: WebSocketUpgrade,
    backend_url: Url,
    request: Request<Body>,
    tracker: Arc<ConnectionTracker>,
) -> Response {
    let user_ctx = request.extensions().get::<UserContext>().cloned();
    
    // Enforcement: Check connection limits if we have user context
    if let Some(ref ctx) = user_ctx {
        if !tracker.try_increment(ctx.address, ctx.tier_id) {
            warn!(user = %ctx.address, "WebSocket connection limit reached");
            metrics::record_rate_limited("websocket_limit");
            return (StatusCode::TOO_MANY_REQUESTS, "WebSocket connection limit reached").into_response();
        }
    }

    metrics::record_long_lived_connection("websocket", 1);

    info!(backend = %backend_url, "Handling WebSocket upgrade");

    let t = tracker.clone();
    let addr = user_ctx.map(|c| c.address);

    ws.on_upgrade(move |socket| async move {
        proxy_ws(socket, backend_url).await;
        // Decrement on finish
        metrics::record_long_lived_connection("websocket", -1);
        if let Some(a) = addr {
            t.decrement(a);
        }
    })
}

async fn proxy_ws(client_ws: WebSocket, backend_url: Url) {
    // 1. Establish connection to backend
    // We use a raw TCP stream or another WS client?
    // Most robust way is to use a WS library to connect to backend and proxy messages.
    // However, for high-performance proxying, raw stream forwarding (after backend upgrade) might be better.
    // But axum-ws gives us high-level Message types.
    
    // Convert http/https to ws/wss
    let mut ws_backend_url = backend_url.clone();
    let scheme = match backend_url.scheme() {
        "http" => "ws",
        "https" => "wss",
        s => s,
    };
    if let Err(_) = ws_backend_url.set_scheme(scheme) {
        error!("Failed to set WS scheme: {}", scheme);
        return;
    }

    match connect_async(ws_backend_url.as_str()).await {
        Ok((backend_ws, _)) => {
            let (mut b_sink, mut b_stream) = backend_ws.split();
            let (mut c_sink, mut c_stream) = client_ws.split();

            let client_to_backend = async {
                while let Some(Ok(msg)) = c_stream.next().await {
                    let b_msg = match msg {
                        Message::Text(t) => TgMessage::Text(t.to_string().into()),
                        Message::Binary(b) => TgMessage::Binary(b.into()),
                        Message::Ping(p) => TgMessage::Ping(p.into()),
                        Message::Pong(p) => TgMessage::Pong(p.into()),
                        Message::Close(c) => {
                            let frame = c.map(tg_close_frame_converter);
                            TgMessage::Close(frame)
                        },
                    };
                    if let Err(e) = b_sink.send(b_msg).await {
                        warn!("Error forwarding to backend: {}", e);
                        break;
                    }
                }
            };

            let backend_to_client = async {
                while let Some(Ok(msg)) = b_stream.next().await {
                    let c_msg = match msg {
                        TgMessage::Text(t) => Message::Text(t.to_string().into()),
                        TgMessage::Binary(b) => Message::Binary(b.into()),
                        TgMessage::Ping(p) => Message::Ping(p.into()),
                        TgMessage::Pong(p) => Message::Pong(p.into()),
                        TgMessage::Close(c) => {
                            let frame = c.map(ax_close_frame_converter);
                            Message::Close(frame)
                        },
                        _ => continue, 
                    };
                    if let Err(e) = c_sink.send(c_msg).await {
                        warn!("Error forwarding to client: {}", e);
                        break;
                    }
                }
            };

            tokio::select! {
                _ = client_to_backend => {},
                _ = backend_to_client => {},
            }
            info!(backend = %backend_url, "WebSocket connection closed");
        }
        Err(e) => {
            error!(backend = %backend_url, error = %e, "Failed to connect to backend WebSocket");
        }
    }
}

fn tg_close_frame_converter(cf: axum::extract::ws::CloseFrame) -> tungstenite::protocol::CloseFrame {
    tungstenite::protocol::CloseFrame {
        code: cf.code.into(),
        reason: cf.reason.to_string().into(),
    }
}

fn ax_close_frame_converter(cf: tungstenite::protocol::CloseFrame) -> axum::extract::ws::CloseFrame {
    axum::extract::ws::CloseFrame {
        code: cf.code.into(),
        reason: cf.reason.to_string().into(),
    }
}
