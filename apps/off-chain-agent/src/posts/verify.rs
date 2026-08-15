use super::PostRequest;
use crate::app_state::AppState;
use crate::auth::extract_user_id_from_headers;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Verified post request — the user_id is extracted from the JWT Bearer
/// token in the `Authorization` header by the middleware.
#[derive(Clone, Serialize, Deserialize)]
pub struct VerifiedPostRequest<T> {
    pub user_id: String,
    pub request: PostRequest<T>,
}

/// Middleware: verify JWT Bearer token and inject the authenticated `user_id`
/// into the verified request. The IC `delegated_identity_wire` model is
/// replaced by JWT auth (yral-auth access token).
pub async fn verify_post_request<T>(
    State(_state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode>
where
    T: for<'de> Deserialize<'de> + Serialize + Clone + Send + Sync + 'static,
{
    // Extract the JSON body
    let (parts, body) = request.into_parts();
    let bytes = match axum::body::to_bytes(body, usize::MAX).await {
        Ok(bytes) => bytes,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    // Extract user_id from the JWT in the Authorization header
    let user_id = extract_user_id_from_headers(&parts.headers)
        .map_err(|(_, code)| StatusCode::from_u16(code).unwrap_or(StatusCode::UNAUTHORIZED))?;

    // Parse the JSON body
    let post_request: PostRequest<T> = match serde_json::from_slice(&bytes) {
        Ok(req) => req,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    // Create a verified request with the authenticated user_id
    let verified_request = VerifiedPostRequest {
        user_id,
        request: post_request,
    };

    let request_body = serde_json::to_string(&verified_request).unwrap();
    let request = Request::from_parts(parts, axum::body::Body::from(request_body));

    Ok(next.run(request).await)
}
