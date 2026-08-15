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

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct FollowUserRequest {
    pub target_user_id: String,
    pub follower_username: Option<String>,
}

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct FollowUserResponse {
    pub success: bool,
}

/// Follow a user.
///
/// The user is authenticated via JWT Bearer token in the `Authorization` header.
/// IC canister follow calls are decommissioned — follow relationships will be
/// tracked via SpacetimeDB in a follow-up PR. For now, this endpoint logs the
/// follow event and returns success.
#[utoipa::path(
    post,
    path = "/follow",
    request_body = FollowUserRequest,
    tag = "user",
    responses(
        (status = 200, description = "Follow successful", body = FollowUserResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    )
)]
#[instrument(skip(state, headers))]
pub async fn handle_follow_user(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<FollowUserRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let follower_user_id =
        extract_user_id_from_headers(&headers).map_err(|(msg, code)| {
            let status = StatusCode::from_u16(code).unwrap_or(StatusCode::UNAUTHORIZED);
            (status, msg)
        })?;

    // Don't allow users to follow themselves
    if follower_user_id == request.target_user_id {
        return Err((
            StatusCode::BAD_REQUEST,
            "Cannot follow yourself".to_string(),
        ));
    }

    // IC canister follow call is decommissioned. Follow relationships will be
    // tracked via SpacetimeDB in a follow-up PR.
    log::info!(
        "Follow request: {} -> {} (follower_username={:?}) — SpacetimeDB follow pending implementation",
        follower_user_id,
        request.target_user_id,
        request.follower_username
    );

    let _ = &state.spacetime_conn;

    Ok(Json(FollowUserResponse { success: true }))
}

/// Follow notification endpoint — push notifications are decommissioned.
/// Kept for API compatibility with existing mobile clients.
#[utoipa::path(
    post,
    path = "/follow-notification",
    request_body = FollowUserRequest,
    tag = "user",
    responses(
        (status = 200, description = "Notification sent successfully", body = FollowUserResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    )
)]
#[instrument(skip(_state, headers))]
pub async fn handle_follow_user_notification(
    State(_state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<FollowUserRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let follower_user_id =
        extract_user_id_from_headers(&headers).map_err(|(msg, code)| {
            let status = StatusCode::from_u16(code).unwrap_or(StatusCode::UNAUTHORIZED);
            (status, msg)
        })?;

    // Push notifications are decommissioned. Log the event for backwards compat.
    log::info!(
        "Follow notification (push decommissioned): {} -> {} (follower_username={:?})",
        follower_user_id,
        request.target_user_id,
        request.follower_username
    );

    Ok(Json(FollowUserResponse { success: true }))
}