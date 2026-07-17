use crate::{
    app_state::AppState,
    consts::{STORJ_INTERFACE_TOKEN, STORJ_INTERFACE_URL},
    AppError,
};
use axum::{extract::State, Json};
use std::sync::Arc;

/// for the purpose of backfilling, can be removed once there are no more items
/// to be filled
pub async fn enqueue_storj_backfill_item(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<storj_interface::duplicate::Args>,
) -> Result<(), AppError> {
    // Perform the storj duplicate directly instead of queueing.
    let client = reqwest::Client::new();
    client
        .post(
            STORJ_INTERFACE_URL
                .join("/duplicate")
                .expect("url to be valid"),
        )
        .json(&payload)
        .bearer_auth(STORJ_INTERFACE_TOKEN.as_str())
        .send()
        .await?
        .error_for_status()?;

    Ok(())
}
