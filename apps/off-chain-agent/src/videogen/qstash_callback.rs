use axum::http::StatusCode;
use std::sync::Arc;
use videogen_common::types_v2::VideoUploadHandling;
use yral_canisters_client::rate_limits::{RateLimits, VideoGenRequestKey, VideoGenRequestStatus};

use crate::{
    app_state::AppState,
    consts::RATE_LIMITS_CANISTER_ID,
    videogen::{
        qstash_types::{VideoGenCallback, VideoGenCallbackResult},
        utils::get_hon_worker_jwt_token,
    },
};

// Import utility functions for JWT and rollback
use super::utils::rollback_balance_on_failure;

/// Update status in rate limits canister
async fn update_rate_limit_status(
    rate_limits_client: &RateLimits<'_>,
    request_key: VideoGenRequestKey,
    status: VideoGenRequestStatus,
) -> Result<(), (StatusCode, String)> {
    match rate_limits_client
        .update_video_generation_status(request_key.clone(), status)
        .await
    {
        Ok(result) => match result {
            yral_canisters_client::rate_limits::Result1::Ok => {
                log::info!(
                    "Successfully updated video generation status for principal {} counter {}",
                    request_key.principal,
                    request_key.counter
                );
                Ok(())
            }
            yral_canisters_client::rate_limits::Result1::Err(e) => {
                log::error!("Failed to update video generation status: {e}");
                Err((StatusCode::INTERNAL_SERVER_ERROR, e))
            }
        },
        Err(e) => {
            log::error!("Failed to call update_video_generation_status: {e}");
            Err((
                StatusCode::SERVICE_UNAVAILABLE,
                format!("Canister call failed: {e}"),
            ))
        }
    }
}

/// Decrement rate limit counter for failed requests
async fn decrement_counter_for_failure(
    rate_limits_client: &RateLimits<'_>,
    request_key: VideoGenRequestKey,
    property: String,
) {
    log::info!(
        "Decrementing rate limit counter for failed request: principal {} counter {}",
        request_key.principal,
        request_key.counter
    );

    if let Err(e) = rate_limits_client
        .decrement_video_generation_counter_v_1(request_key, property)
        .await
    {
        log::error!("Failed to decrement counter: {e}");
        // Don't fail the callback if decrement fails
    }
}

/// Internal callback handler that can be used by webhook handlers
pub async fn handle_video_gen_callback_internal(
    state: Arc<AppState>,
    callback: VideoGenCallback,
) -> Result<StatusCode, (StatusCode, String)> {
    log::info!(
        "Processing video generation callback for principal {} counter {}",
        callback.request_key.principal,
        callback.request_key.counter
    );

    // 2. Determine status based on callback result
    let (status, should_decrement) = match &callback.result {
        VideoGenCallbackResult::Success(response) => (
            VideoGenRequestStatus::Complete(response.video_url.clone()),
            false,
        ),
        VideoGenCallbackResult::Failure(error) => {
            (VideoGenRequestStatus::Failed(error.clone()), true)
        }
    };

    // 3. Update status in rate limits canister
    let rate_limits_client = RateLimits(*RATE_LIMITS_CANISTER_ID, &state.agent);
    let request_key = VideoGenRequestKey {
        principal: callback.request_key.principal,
        counter: callback.request_key.counter,
    };

    update_rate_limit_status(&rate_limits_client, request_key.clone(), status.clone()).await?;

    // 4. Handle failure cleanup if needed
    if should_decrement {
        // Decrement counter
        decrement_counter_for_failure(&rate_limits_client, request_key, callback.property.clone())
            .await;

        // Rollback balance using our utils function
        if callback.deducted_amount.is_some() {
            match get_hon_worker_jwt_token() {
                Ok(jwt_token) => {
                    log::info!(
                        "Rolling back {} {:?} for failed video generation: principal {}",
                        callback.deducted_amount.unwrap_or(0),
                        callback.token_type,
                        callback.request_key.principal
                    );

                    if let Err(e) = rollback_balance_on_failure(
                        callback.request_key.principal,
                        callback.deducted_amount,
                        &callback.token_type,
                        jwt_token,
                        &state.agent,
                    )
                    .await
                    {
                        log::error!("Balance rollback failed: {e}");
                        // Don't fail the callback on rollback errors
                    }
                }
                Err(_) => {
                    log::error!("Cannot rollback - JWT token not available");
                }
            }
        }
    }

    if let VideoGenRequestStatus::Complete(_ai_video_url) = &status {
        match callback.handle_video_upload {
            Some(VideoUploadHandling::ServerDraft) => {
                // Decrypt identity from callback blob
                let delegated_identity = if let Some(encrypted) = &callback.encrypted_identity {
                    state.crypto.decrypt_identity(encrypted).ok()
                } else {
                    None
                };

                let _ = delegated_identity; // Identity available for future upload-to-canister flow

                Ok(StatusCode::OK)
            }

            _ => Ok::<_, (StatusCode, String)>(StatusCode::OK),
        }
    } else {
        Ok(StatusCode::OK)
    }
}
