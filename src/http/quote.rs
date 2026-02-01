use axum::{extract::{State, Json, Path}, http::StatusCode, response::IntoResponse};
use uuid::Uuid;
use crate::http::server::InnerStateWrapper;
use crate::quoting::QuoteRequest;

pub async fn create_quote(
    State(state): State<InnerStateWrapper>,
    Json(request): Json<QuoteRequest>,
) -> impl IntoResponse {
    let engine = match &state.inner.quote_engine {
        Some(e) => e,
        None => return (StatusCode::SERVICE_UNAVAILABLE, "Quoting service disabled").into_response(),
    };

    // Strict Validation (Phase 24)
    if state.inner.config.security.strict_validation {
        if request.user_address.is_zero() {
            return (StatusCode::BAD_REQUEST, "Zero address not allowed").into_response();
        }
        
        if let Some(ds) = request.duration_seconds {
            if ds < 3600 { // Min 1 hour
                return (StatusCode::BAD_REQUEST, "Duration too short (min 1h)").into_response();
            }
            if ds > 365 * 24 * 3600 { // Max 1 year
                return (StatusCode::BAD_REQUEST, "Duration too long (max 1y)").into_response();
            }
        }
    }

    match engine.generate_quote(request).await {
        Ok(quote) => (StatusCode::CREATED, Json(quote)).into_response(),
        Err(e) => {
            tracing::error!("Failed to generate quote: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to generate quote").into_response()
        }
    }
}

pub async fn get_quote(
    State(state): State<InnerStateWrapper>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let engine = match &state.inner.quote_engine {
        Some(e) => e,
        None => return (StatusCode::SERVICE_UNAVAILABLE, "Quoting service disabled").into_response(),
    };

    match engine.get_quote(id) {
        Some(quote) => (StatusCode::OK, Json(quote)).into_response(),
        None => (StatusCode::NOT_FOUND, "Quote not found").into_response(),
    }
}
