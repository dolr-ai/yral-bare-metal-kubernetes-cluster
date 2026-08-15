use std::sync::Arc;

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use base64::Engine;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use utoipa::ToSchema;

use crate::app_state::AppState;
use crate::auth::extract_user_id_from_headers;
use crate::utils::s3::{delete_profile_image_from_s3, upload_profile_image_to_s3};

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct UploadProfileImageRequest {
    pub image_data: String, // Base64 encoded image data
}

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct UploadProfileImageResponse {
    pub profile_image_url: String,
}

/// Upload a profile image to S3.
///
/// The user is authenticated via JWT Bearer token in the `Authorization` header.
/// IC canister profile updates are decommissioned — the image URL can be stored
/// via SpacetimeDB in a follow-up PR.
#[utoipa::path(
    post,
    path = "/profile-image",
    request_body = UploadProfileImageRequest,
    tag = "user",
    responses(
        (status = 200, description = "Profile image uploaded successfully", body = UploadProfileImageResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    )
)]
#[instrument(skip(state, headers))]
pub async fn handle_upload_profile_image(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<UploadProfileImageRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let user_id = extract_user_id_from_headers(&headers).map_err(|(msg, code)| {
        let status = StatusCode::from_u16(code).unwrap_or(StatusCode::UNAUTHORIZED);
        (status, msg)
    })?;

    // Remove data URL prefix if present
    let base64_data = if let Some(comma_pos) = request.image_data.find(',') {
        &request.image_data[comma_pos + 1..]
    } else {
        &request.image_data
    };

    // Validate image data size
    if base64_data.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Image data is empty".to_string()));
    }

    // Maximum allowed size for base64 string (~5MB when decoded)
    const MAX_BASE64_SIZE: usize = 7 * 1024 * 1024; // ~5MB when decoded
    if base64_data.len() > MAX_BASE64_SIZE {
        let size_mb = base64_data.len() / (1024 * 1024);
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "Image too large: {}MB. Maximum allowed size is 5MB",
                size_mb
            ),
        ));
    }

    // Validate that it's actually base64 data
    if base64::engine::general_purpose::STANDARD
        .decode(base64_data)
        .is_err()
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "Invalid image data format. Please upload a valid image".to_string(),
        ));
    }

    // Upload image to S3
    let profile_image_url = upload_profile_image_to_s3(base64_data, &user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to upload profile image: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to upload profile image: {e}"),
            )
        })?;

    // IC canister profile update is decommissioned. The image URL can be
    // stored via SpacetimeDB (set profile image reducer) in a follow-up PR.
    log::info!(
        "Profile image uploaded for user {} -> {} (SpacetimeDB profile update pending)",
        user_id,
        profile_image_url
    );

    let _ = &state.spacetime_conn;

    Ok(Json(UploadProfileImageResponse { profile_image_url }))
}

#[derive(Serialize, Deserialize, ToSchema, Debug)]
pub struct DeleteProfileImageRequest {
    pub user_id: String,
}

/// Delete a profile image from S3.
///
/// The user is authenticated via JWT Bearer token in the `Authorization` header.
#[utoipa::path(
    delete,
    path = "/profile-image",
    request_body = DeleteProfileImageRequest,
    tag = "user",
    responses(
        (status = 200, description = "Profile image deleted successfully"),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    )
)]
#[instrument(skip(_state, headers))]
pub async fn handle_delete_profile_image(
    State(_state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<DeleteProfileImageRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let user_id = extract_user_id_from_headers(&headers).map_err(|(msg, code)| {
        let status = StatusCode::from_u16(code).unwrap_or(StatusCode::UNAUTHORIZED);
        (status, msg)
    })?;

    // Verify the request user_id matches the authenticated user
    if user_id != request.user_id {
        return Err((
            StatusCode::FORBIDDEN,
            "Cannot delete another user's profile image".to_string(),
        ));
    }

    // Delete image from S3
    delete_profile_image_from_s3(&user_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete profile image: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to delete profile image: {e}"),
            )
        })?;

    log::info!("Successfully deleted profile image for user {}", user_id);

    Ok(StatusCode::OK)
}