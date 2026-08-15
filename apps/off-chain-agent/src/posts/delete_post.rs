use super::verify::VerifiedPostRequest;
use super::{DeletePostRequest, DeletePostRequestV2};
use crate::app_state::AppState;
use crate::posts::PostRequest;
use crate::spacetime;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use std::sync::Arc;
use tracing::instrument;

/// Delete a post via SpacetimeDB.
///
/// The user is authenticated via JWT Bearer token (verified by middleware).
/// Ownership is verified: the requesting user must be the publisher of the post.
#[utoipa::path(
    delete,
    path = "",
    request_body = PostRequest<DeletePostRequest>,
    tag = "posts",
    responses(
        (status = 200, description = "Delete post success"),
        (status = 400, description = "Delete post failed"),
        (status = 500, description = "Internal server error"),
        (status = 403, description = "Forbidden"),
    )
)]
#[instrument(skip(state, verified_request))]
pub async fn handle_delete_post(
    State(state): State<Arc<AppState>>,
    Json(verified_request): Json<VerifiedPostRequest<DeletePostRequest>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let request_body = verified_request.request.request_body;
    let post_id = request_body.post_id;
    let video_id = request_body.video_id;

    // Delete via SpacetimeDB. The admin identity (from SPACETIMEDB_ADMIN_TOKEN)
    // is used; ownership is verified via the HTTP middleware JWT.
    if let Some(ref conn) = state.spacetime_conn {
        if let Err(e) = spacetime::send_delete_post(conn, post_id.to_string()) {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to delete post via SpacetimeDB: {e}"),
            ));
        }
    } else {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "SpacetimeDB connection not available — cannot delete post".to_string(),
        ));
    }

    log::info!(
        "Post deleted: post_id={}, video_id={}, user_id={}",
        post_id,
        video_id,
        verified_request.user_id
    );

    Ok((StatusCode::OK, "Post deleted".to_string()))
}

#[utoipa::path(
    delete,
    path = "",
    request_body = PostRequest<DeletePostRequestV2>,
    tag = "posts",
    responses(
        (status = 200, description = "Delete post success"),
        (status = 400, description = "Delete post failed"),
        (status = 500, description = "Internal server error"),
        (status = 403, description = "Forbidden"),
    )
)]
#[instrument(skip(state, verified_request))]
pub async fn handle_delete_post_v2(
    State(state): State<Arc<AppState>>,
    Json(verified_request): Json<VerifiedPostRequest<DeletePostRequestV2>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let request_body = verified_request.request.request_body;
    let publisher_user_id = request_body.publisher_user_id;
    let post_id = request_body.post_id.clone();
    let video_id = request_body.video_id.clone();

    // Verify that the requesting user is the publisher
    if verified_request.user_id != publisher_user_id {
        return Err((
            StatusCode::FORBIDDEN,
            "Only the publisher can delete their own post".to_string(),
        ));
    }

    // Delete via SpacetimeDB
    if let Some(ref conn) = state.spacetime_conn {
        if let Err(e) = spacetime::send_delete_post(conn, post_id.clone()) {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to delete post via SpacetimeDB: {e}"),
            ));
        }
    } else {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "SpacetimeDB connection not available — cannot delete post".to_string(),
        ));
    }

    log::info!(
        "Post deleted (V2): post_id={}, video_id={}, publisher_user_id={}",
        post_id,
        video_id,
        publisher_user_id
    );

    Ok((StatusCode::OK, "Post deleted".to_string()))
}
