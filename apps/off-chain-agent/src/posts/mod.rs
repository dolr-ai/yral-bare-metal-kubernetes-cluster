use std::sync::Arc;
use axum::middleware;
use delete_post::{handle_delete_post, handle_delete_post_v2};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use self::types::PostRequest;
use utoipa::ToSchema;
use utoipa_axum::{
    router::{OpenApiRouter, UtoipaMethodRouterExt},
    routes,
};
use verify::verify_post_request;
use crate::posts::delete_post::{__path_handle_delete_post, __path_handle_delete_post_v2};
use crate::app_state::AppState;

pub mod delete_post;
pub mod types;
mod utils;
mod verify;

/// Macro to create a route with verification middleware
macro_rules! verified_route {
    ($router:expr, $handler:path, $request_type:ty, $state:expr) => {
        $router.routes(routes!($handler).layer(middleware::from_fn_with_state(
            $state.clone(),
            verify_post_request::<$request_type>,
        )))
    };
}

#[instrument(skip(state))]
pub fn posts_router(state: Arc<AppState>) -> OpenApiRouter {
    let mut router = OpenApiRouter::new();

    router = verified_route!(router, handle_delete_post, DeletePostRequest, state);

    router.with_state(state)
}

#[instrument(skip(state))]
pub fn posts_router_v2(state: Arc<AppState>) -> OpenApiRouter {
    let mut router = OpenApiRouter::new();

    router = verified_route!(router, handle_delete_post_v2, DeletePostRequestV2, state);

    router.with_state(state)
}

#[derive(Serialize, Deserialize, Clone, ToSchema, Debug)]
pub struct DeletePostRequest {
    pub canister_id: String,
    pub post_id: u64,
    pub video_id: String,
}

#[derive(Serialize, Deserialize, Clone, ToSchema, Debug)]
pub struct DeletePostRequestV2 {
    pub publisher_user_id: String,
    post_id: String, // Changed from u64 to String
    video_id: String,
}
