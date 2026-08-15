use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};

use crate::app_state::AppState;
use crate::auth::extract_user_id_from_headers;

use super::{EventBulkRequest, VerifiedEventBulkRequest};

/// V2 verified bulk events (after middleware validation)
use super::{EventBulkRequestV2, VerifiedEventBulkRequestV2};

/// Middleware: verify JWT Bearer token from the `Authorization` header and
/// inject the authenticated `user_id` into the verified request.
///
/// The mobile app sends events with `Authorization: Bearer <jwt>` (the
/// yral-auth access token). The JWT `sub` claim is the user_id.
pub async fn verify_event_bulk_request(
    State(_state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    // Extract the JSON body
    let (parts, body) = request.into_parts();
    let bytes = match axum::body::to_bytes(body, usize::MAX).await {
        Ok(bytes) => bytes,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Failed to parse request body #1: {e}"),
            ))
        }
    };

    // Extract user_id from the JWT in the Authorization header
    let user_id = extract_user_id_from_headers(&parts.headers).map_err(|(msg, code)| {
        let status = StatusCode::from_u16(code).unwrap_or(StatusCode::UNAUTHORIZED);
        (status, msg)
    })?;

    // Parse the JSON body (no more delegated_identity_wire field)
    let event_bulk_request: EventBulkRequest = match serde_json::from_slice(&bytes) {
        Ok(req) => req,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Failed to parse request body to EventBulkRequest: {e}"),
            ))
        }
    };

    // Verify all events belong to the authenticated user
    for event in &event_bulk_request.events {
        if let Some(event_user_id) = event.user_id() {
            if event_user_id != user_id {
                return Err((
                    StatusCode::BAD_REQUEST,
                    "Invalid user id: event user_id does not match authenticated user".to_string(),
                ));
            }
        }
    }

    let verified_request = VerifiedEventBulkRequest {
        events: event_bulk_request.events,
        user_id,
    };

    let request_body = serde_json::to_string(&verified_request).unwrap();
    let request = Request::from_parts(parts, axum::body::Body::from(request_body));

    Ok(next.run(request).await)
}

/// V2/V3 middleware: verify JWT Bearer token and inject `user_id` for
/// arbitrary JSON payload events.
pub async fn verify_event_bulk_request_v3(
    State(_state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    let (parts, body) = request.into_parts();
    let bytes = match axum::body::to_bytes(body, usize::MAX).await {
        Ok(bytes) => bytes,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Failed to parse request body: {e}"),
            ))
        }
    };

    // Extract user_id from the JWT in the Authorization header
    let user_id = extract_user_id_from_headers(&parts.headers).map_err(|(msg, code)| {
        let status = StatusCode::from_u16(code).unwrap_or(StatusCode::UNAUTHORIZED);
        (status, msg)
    })?;

    let event_bulk_request: EventBulkRequestV2 = match serde_json::from_slice(&bytes) {
        Ok(req) => req,
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Failed to parse request body to EventBulkRequestV2: {e}"),
            ))
        }
    };

    // Verify user_id in each event matches the authenticated user
    for event in &event_bulk_request.events {
        if let Some(event_user_id) = event.get("user_id").and_then(|v| v.as_str()) {
            if event_user_id != user_id {
                return Err((
                    StatusCode::BAD_REQUEST,
                    "Invalid user_id: does not match authenticated user".to_string(),
                ));
            }
        }
    }

    let verified_request = VerifiedEventBulkRequestV2 {
        events: event_bulk_request.events,
        user_id,
    };

    let request_body = serde_json::to_string(&verified_request).unwrap();
    let request = Request::from_parts(parts, axum::body::Body::from(request_body));

    Ok(next.run(request).await)
}