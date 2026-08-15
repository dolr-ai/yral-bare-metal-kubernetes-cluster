use std::sync::Arc;

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use utoipa::ToSchema;

use crate::app_state::AppState;
use crate::auth::extract_user_id_from_headers;

/// Delete a user's data.
///
/// The user is authenticated via JWT Bearer token in the `Authorization` header.
/// IC canister deletion is decommissioned — this endpoint logs the request and
/// returns success. Full user data deletion from SpacetimeDB will be implemented
/// in a follow-up PR.
#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct DeleteUserRequest {
    pub user_id: String,
}

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct DeleteUserResponse {
    pub success: bool,
}

#[utoipa::path(
    delete,
    path = "/",
    request_body = DeleteUserRequest,
    tag = "user",
    responses(
        (status = 200, description = "User deleted successfully", body = DeleteUserResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    )
)]
#[instrument(skip(state, headers))]
pub async fn handle_delete_user(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<DeleteUserRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Verify the JWT Bearer token and extract the authenticated user_id
    let authenticated_user_id =
        extract_user_id_from_headers(&headers).map_err(|(msg, code)| {
            let status = StatusCode::from_u16(code).unwrap_or(StatusCode::UNAUTHORIZED);
            (status, msg)
        })?;

    // Verify the request user_id matches the authenticated user
    if authenticated_user_id != request.user_id {
        return Err((
            StatusCode::FORBIDDEN,
            "Cannot delete another user's account".to_string(),
        ));
    }

    // IC canister deletion is decommissioned. Full user data deletion from
    // SpacetimeDB will be implemented in a follow-up PR.
    log::info!(
        "User deletion requested for user_id={} (SpacetimeDB deletion pending implementation)",
        request.user_id
    );

    // If SpacetimeDB connection is available, we could delete user data here.
    // For now, just log and return success.
    let _ = &state.spacetime_conn;

    Ok(Json(DeleteUserResponse { success: true }))
}