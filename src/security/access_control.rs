//! Access Control Middleware.
//! Enforces subscription requirements.

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use alloy::primitives::Address;

use crate::payments::cache::SubscriptionCache;

/// State required for access control.
#[derive(Clone)]
pub struct AccessControlState {
    pub cache: Arc<SubscriptionCache>,
    pub enabled: bool,
    pub grace_period_secs: u64,
}

/// Context attached to authenticated requests.
#[derive(Clone, Debug)]
pub struct UserContext {
    pub address: Address,
    pub tier_id: u8,
}

pub async fn access_control_middleware(
    State(state): State<AccessControlState>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    // 1. Check if payments are enabled. If not, allow all (passthrough mode).
    if !state.enabled {
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
    match state.cache.get_subscription(&address) {
        Some(sub) => {
            if sub.is_active_with_grace(state.grace_period_secs) {
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
