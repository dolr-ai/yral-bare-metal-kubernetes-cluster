use axum::{extract::State, Json};
use std::sync::Arc;
use types::PopulateIndexResponse;

use crate::state::AppState;
use crate::utils::canister::populate_canister_to_principal_index;
use crate::utils::error::Result;

pub async fn populate_canister_index(
    State(state): State<Arc<AppState>>,
) -> Result<Json<PopulateIndexResponse>> {
    // Call the populate function
    let (total, processed) = populate_canister_to_principal_index(
        &state.backend_admin_ic_agent,
        &state.dragonfly_redis_store,
    )
    .await?;

    let response = PopulateIndexResponse {
        total,
        processed,
        failed: total - processed,
    };

    Ok(Json(response))
}
