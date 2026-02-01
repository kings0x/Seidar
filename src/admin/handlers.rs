use axum::{
    extract::State,
    Json,
};
use serde::Serialize;
use std::sync::atomic::Ordering;
use crate::http::server::AppState;

#[derive(Serialize)]
pub struct SystemStatus {
    pub version: &'static str,
    pub status: &'static str,
}

#[derive(Serialize)]
pub struct BackendStatus {
    pub name: String,
    pub group: String,
    pub address: String,
    pub healthy: bool,
    pub active_connections: usize,
}

#[derive(Serialize)]
pub struct AnalyticsSummary {
    pub total_requests: usize,
    pub active_subscriptions: usize,
    pub expired_subscriptions: usize,
}

pub async fn get_status() -> Json<SystemStatus> {
    Json(SystemStatus {
        version: env!("CARGO_PKG_VERSION"),
        status: "operational",
    })
}

pub async fn get_backends(
    State(state): State<AppState>,
) -> Json<Vec<BackendStatus>> {
    let mut statuses = Vec::new();
    let inner = state.inner.load_full();
    let backends = inner.backends.all_backends();
    
    for b in backends {
        statuses.push(BackendStatus {
            name: "unknown".to_string(), // Backend struct doesn't store name currently
            group: "unknown".to_string(),
            address: b.addr.to_string(),
            healthy: b.state.load(Ordering::Relaxed) == 1,
            active_connections: b.active_connections.load(Ordering::Relaxed),
        });
    }
    
    Json(statuses)
}

pub async fn get_analytics(
    State(state): State<AppState>,
) -> Json<AnalyticsSummary> {
    let inner = state.inner.load();
    let (active_subs, expired_subs) = inner.subscription_cache.get_summary();
    
    Json(AnalyticsSummary {
        total_requests: inner.request_count.load(Ordering::Relaxed),
        active_subscriptions: active_subs,
        expired_subscriptions: expired_subs,
    })
}

pub async fn get_cache(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let inner = state.inner.load();
    let (active, expired) = inner.subscription_cache.get_summary();
    Json(serde_json::json!({
        "total_tracked": active + expired,
        "active": active,
        "expired": expired,
    }))
}
