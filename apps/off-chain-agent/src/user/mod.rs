// Decommissioned endpoints (both were logging-only stubs from the IC
// migration era; the migration is complete):
//   - `delete_user` — account deletion now lives in the SpacetimeDB
//     module's `delete_user_info` reducer (one transactional cascade).
//   - `migrate_user` — the IC → SpacetimeDB user migration it queued
//     externally is long done (user_profiles_2 + posts_3 migrations
//     both complete).
pub mod follow;
pub mod utils;

use std::sync::Arc;

use utoipa_axum::{router::OpenApiRouter, routes};

use crate::app_state::AppState;

pub fn user_router(state: Arc<AppState>) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(follow::handle_follow_user))
        .routes(routes!(follow::handle_follow_user_notification))
        .with_state(state)
}
