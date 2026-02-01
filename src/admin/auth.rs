use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use crate::http::server::AppState;

pub async fn admin_auth_middleware(
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let state = request
        .extensions()
        .get::<AppState>()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let inner = state.inner.load_full();

    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok());

    if let Some(auth_val) = auth_header {
        if auth_val == format!("Bearer {}", inner.config.admin.api_key) {
            return Ok(next.run(request).await);
        }
    }

    Err(StatusCode::UNAUTHORIZED)
}
