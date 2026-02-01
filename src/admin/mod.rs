pub mod handlers;
pub mod auth;

use axum::{
    routing::get,
    Router,
    middleware,
};
use crate::http::server::AppState;
use self::handlers::*;
use self::auth::admin_auth_middleware;

pub fn setup_admin_router(state: AppState) -> Router {
    Router::new()
        .route("/admin/status", get(get_status))
        .route("/admin/backends", get(get_backends))
        .route("/admin/analytics", get(get_analytics))
        .route("/admin/cache", get(get_cache))
        .layer(middleware::from_fn(admin_auth_middleware))
        .with_state(state)
}
