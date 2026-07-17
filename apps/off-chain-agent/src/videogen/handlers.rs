use crate::app_state::AppState;
use axum::{extract::State, http::StatusCode, Json};
use std::sync::Arc;
use videogen_common::VideoGenerator;

/// Generate a video using delegated identity for authentication and balance deduction
#[utoipa::path(
    post,
    path = "/generate_with_identity",
    request_body = videogen_common::VideoGenRequestWithIdentity,
    responses(
        (status = 200, description = "Video generation started successfully", body = videogen_common::VideoGenQueuedResponse),
        (status = 400, description = "Invalid input", body = videogen_common::VideoGenError),
        (status = 401, description = "Authentication failed - Invalid identity", body = videogen_common::VideoGenError),
        (status = 402, description = "Insufficient balance", body = videogen_common::VideoGenError),
        (status = 429, description = "Rate limit exceeded", body = videogen_common::VideoGenError),
        (status = 502, description = "Provider error", body = videogen_common::VideoGenError),
        (status = 503, description = "Service unavailable", body = videogen_common::VideoGenError),
    ),
    tag = "VideoGen"
)]
pub async fn generate_video_with_identity(
    State(app_state): State<Arc<AppState>>,
    Json(identity_request): Json<videogen_common::VideoGenRequestWithIdentity>,
) -> Result<
    Json<videogen_common::VideoGenQueuedResponse>,
    (StatusCode, Json<videogen_common::VideoGenError>),
> {
    // Validate identity and extract user principal
    let user_principal = super::utils::validate_delegated_identity(&identity_request)?;

    // Extract request metadata
    let metadata = super::utils::extract_request_metadata(&identity_request.request);

    let input = identity_request.request.input;

    // Use common processing function
    let request_key = super::utils::process_video_generation(
        &app_state,
        user_principal,
        input,
        metadata.token_type,
        identity_request.delegated_identity.clone(),
        None,
    )
    .await?;

    // Build and return response
    Ok(Json(super::utils::build_queued_response(
        request_key,
        metadata.provider,
    )))
}
