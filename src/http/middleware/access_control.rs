//! Access Control Middleware.
//! Enforces subscription requirements.

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::IntoResponse,
};
use std::sync::Arc;
use tracing::warn;
use alloy::primitives::Address;

use crate::http::server::InnerStateWrapper;
use crate::payments::cache::SubscriptionCache;

/// Context attached to authenticated requests.
#[derive(Clone, Debug)]
pub struct UserContext {
    pub address: Address,
    pub tier_id: u8,
}

pub async fn access_control_middleware(
    State(state): State<InnerStateWrapper>,
    mut req: Request<Body>,
    next: Next,
) -> impl IntoResponse {
    // 1. Check if payments are enabled. If not, allow all (passthrough mode).
    if !state.inner.config.payments.enabled {
        return next.run(req).await;
    }

    // 2. Extract X-User-Address header
    let user_address = match req.headers().get("X-User-Address") {
        Some(val) => val.to_str().unwrap_or_default(),
        None => {
            return (StatusCode::UNAUTHORIZED, "Missing X-User-Address header").into_response();
        }
    };

    let address: Address = match user_address.parse() {
        Ok(a) => a,
        Err(_) => {
            return (StatusCode::BAD_REQUEST, "Invalid X-User-Address format").into_response();
        }
    };

    // 3. Verify subscription in cache
    match state.inner.subscription_cache.get_subscription(&address) {
        Some(sub) => {
            if sub.is_active() {
                // Attach context
                let ctx = UserContext {
                    address,
                    tier_id: sub.tier_id,
                };
                req.extensions_mut().insert(ctx);
                next.run(req).await
            } else {
                (StatusCode::FORBIDDEN, "Subscription expired").into_response()
            }
        }
        None => {
            // Optional: for dev, maybe allow strict mode?
            // For now, deny.
            (StatusCode::FORBIDDEN, "No active subscription found").into_response()
        }
    }
}
