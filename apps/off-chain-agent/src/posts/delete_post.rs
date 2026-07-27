use std::sync::Arc;

use super::types::{UserPost, UserPostV2};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use types::PostRequest;
use verify::VerifiedPostRequest;
use yral_canisters_client::user_post_service::UserPostService;

use crate::kvrocks::{KvrocksClient, VideoDeleted};
use crate::{
    app_state::AppState,
    consts::{USER_INFO_SERVICE_CANISTER_ID, USER_POST_SERVICE_CANISTER_ID},
    user::utils::get_agent_from_delegated_identity_wire,
};

use super::{types, verify, DeletePostRequest, DeletePostRequestV2};

const BULK_DELETE_BATCH_SIZE: usize = 500;

// TODO: canister_id still being used
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
    // Verify that the canister ID matches the user's canister
    if verified_request.request.request_body.canister_id != verified_request.user_canister {
        return Err((StatusCode::FORBIDDEN, "Forbidden".to_string()));
    }

    let request_body = verified_request.request.request_body;

    let canister_id = request_body.canister_id.to_string();
    let post_id = request_body.post_id;
    let video_id = request_body.video_id;

    let user_ic_agent =
        get_agent_from_delegated_identity_wire(&verified_request.request.delegated_identity_wire)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // TODO: migrate to user_post_service/user_info_service
    // Previously used IndividualUserTemplate to delete posts from individual user
    // canisters. Those canisters have been decommissioned — route all deletes
    // through UserPostService now.
    let user_post_service = UserPostService(*USER_POST_SERVICE_CANISTER_ID, &user_ic_agent);

    // Call the canister to delete the post
    let delete_res = user_post_service.delete_post(post_id.to_string()).await;
    match delete_res {
        Ok(yral_canisters_client::user_post_service::Result_::Ok) => (),
        Ok(yral_canisters_client::user_post_service::Result_::Err(_)) => {
            return Err((
                StatusCode::BAD_REQUEST,
                "Delete post failed - either the post doesn't exist or already deleted".to_string(),
            ))
        }
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal server error: {e}"),
            ))
        }
    }

    record_video_delete(state.clone(), canister_id, post_id, video_id.clone())
        .await
        .map_err(|e| {
            log::error!("Failed to record video delete row: {e}");

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to record video delete: {e}"),
            )
        })?;

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
    if verified_request.user_principal != publisher_user_id {
        return Err((
            StatusCode::FORBIDDEN,
            "Only the publisher can delete their own post".to_string(),
        ));
    }

    // Get the publisher's canister
    let publisher_canister_id = state
        .get_individual_canister_by_user_principal(publisher_user_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get publisher canister: {e}"),
            )
        })?;

    let user_ic_agent =
        get_agent_from_delegated_identity_wire(&verified_request.request.delegated_identity_wire)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Route based on canister or post_id format: UUID post_ids always belong to UserPostService,
    // even if the user's metadata still points to a legacy individual canister.
    let is_uuid_post_id = post_id.parse::<u64>().is_err();

    if publisher_canister_id == *USER_INFO_SERVICE_CANISTER_ID || is_uuid_post_id {
        // Use UserPostService
        let user_post_service = UserPostService(*USER_POST_SERVICE_CANISTER_ID, &user_ic_agent);

        // UserPostService.delete_post takes a String
        let delete_res = user_post_service.delete_post(post_id.clone()).await;
        match delete_res {
            Ok(yral_canisters_client::user_post_service::Result_::Ok) => (),
            Ok(yral_canisters_client::user_post_service::Result_::Err(_)) => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    "Delete post failed - either the post doesn't exist or already deleted"
                        .to_string(),
                ))
            }
            Err(e) => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Internal server error: {e}"),
                ))
            }
        }
    } else {
        // TODO: migrate to user_post_service/user_info_service
        // Individual user canisters have been decommissioned. Route all deletes
        // through UserPostService, even for users whose metadata still points to
        // a legacy canister.
        let user_post_service = UserPostService(*USER_POST_SERVICE_CANISTER_ID, &user_ic_agent);

        let delete_res = user_post_service.delete_post(post_id.clone()).await;
        match delete_res {
            Ok(yral_canisters_client::user_post_service::Result_::Ok) => (),
            Ok(yral_canisters_client::user_post_service::Result_::Err(_)) => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    "Delete post failed - either the post doesn't exist or already deleted"
                        .to_string(),
                ))
            }
            Err(e) => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Internal server error: {e}"),
                ))
            }
        }
    }

    // Record video deletion in Kvrocks (V2, String post_id)
    record_video_delete_v2(
        state.clone(),
        publisher_canister_id.to_string(),
        post_id,
        video_id.clone(),
    )
    .await
    .map_err(|e| {
        log::error!("Failed to record video delete row: {e}");

        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to record video delete: {e}"),
        )
    })?;

    Ok((StatusCode::OK, "Post deleted".to_string()))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoUniqueRow {
    pub video_id: String,
    pub videohash: String,
    pub created_at: String,
}

pub async fn record_video_delete(
    state: Arc<AppState>,
    canister_id: String,
    post_id: u64,
    video_id: String,
) -> Result<(), anyhow::Error> {
    bulk_insert_video_delete_rows(
        &state.kvrocks_client,
        vec![UserPost {
            canister_id,
            post_id,
            video_id,
        }],
    )
    .await?;

    Ok(())
}

pub async fn record_video_delete_v2(
    state: Arc<AppState>,
    canister_id: String,
    post_id: String, // Changed from u64 to String
    video_id: String,
) -> Result<(), anyhow::Error> {
    bulk_insert_video_delete_rows_v2(
        &state.kvrocks_client,
        vec![UserPostV2 {
            canister_id,
            post_id,
            video_id,
        }],
    )
    .await?;

    Ok(())
}

pub async fn bulk_insert_video_delete_rows(
    kvrocks_client: &KvrocksClient,
    posts: Vec<UserPost>,
) -> Result<(), anyhow::Error> {
    // Process posts in batches of 500
    for chunk in posts.chunks(BULK_DELETE_BATCH_SIZE) {
        // Push to kvrocks
        for post in chunk {
            let delete_data = VideoDeleted {
                canister_id: post.canister_id.to_string(),
                post_id: post.post_id.to_string(),
                video_id: post.video_id.clone(),
                gcs_video_id: format!("gs://yral-videos/{}.mp4", post.video_id),
                deleted_at: chrono::Utc::now().to_rfc3339(),
            };
            if let Err(e) = kvrocks_client.store_video_deleted(&delete_data).await {
                log::error!(
                    "Error pushing video delete data to kvrocks for {}: {}",
                    post.video_id,
                    e
                );
            }
        }
    }

    Ok(())
}

pub async fn bulk_insert_video_delete_rows_v2(
    kvrocks_client: &KvrocksClient,
    posts: Vec<UserPostV2>,
) -> Result<(), anyhow::Error> {
    // Process posts in batches of 500
    for chunk in posts.chunks(BULK_DELETE_BATCH_SIZE) {
        // Push to kvrocks
        for post in chunk {
            let delete_data = VideoDeleted {
                canister_id: post.canister_id.clone(),
                post_id: post.post_id.clone(),
                video_id: post.video_id.clone(),
                gcs_video_id: format!("gs://yral-videos/{}.mp4", post.video_id),
                deleted_at: chrono::Utc::now().to_rfc3339(),
            };
            if let Err(e) = kvrocks_client.store_video_deleted(&delete_data).await {
                log::error!(
                    "Error pushing video delete data to kvrocks for {}: {}",
                    post.video_id,
                    e
                );
            }
        }
    }

    Ok(())
}
