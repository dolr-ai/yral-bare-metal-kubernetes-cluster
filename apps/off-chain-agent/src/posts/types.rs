use crate::events::types::string_or_number;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Request body for post operations. The user is authenticated via JWT
/// Bearer token in the `Authorization` header (verified by middleware),
/// so the body no longer carries `delegated_identity_wire`.
#[derive(Serialize, Deserialize, Clone, ToSchema)]
pub struct PostRequest<T> {
    pub post_id: String,
    #[serde(flatten)]
    pub request_body: T,
}

#[derive(Serialize)]
pub struct VideoDeleteRow {
    pub canister_id: String,
    pub post_id: u64,
    pub video_id: String,
    pub gcs_video_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPost {
    pub canister_id: String,
    pub post_id: u64,
    pub video_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPostV2 {
    pub canister_id: String,
    #[serde(deserialize_with = "string_or_number")]
    pub post_id: String, // Changed from u64 to String
    pub video_id: String,
}

impl From<UserPost> for UserPostV2 {
    fn from(post: UserPost) -> Self {
        Self {
            canister_id: post.canister_id,
            post_id: post.post_id.to_string(),
            video_id: post.video_id,
        }
    }
}
