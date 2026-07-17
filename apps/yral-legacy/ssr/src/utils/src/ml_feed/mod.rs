use candid::Principal;
use serde::Deserialize;
use serde::Serialize;
use yral_canisters_common::utils::posts::PostDetails;

/// Piece of post details that should be available as quickly as possible to ensure fast loading of the infinite scroller
#[derive(Clone)]
pub struct QuickPostDetails {
    pub video_uid: String,
    pub canister_id: Principal,
    pub publisher_user_id: Principal,
    pub nsfw_probability: f32,
    pub post_id: String,
}

impl From<PostDetails> for QuickPostDetails {
    fn from(value: PostDetails) -> Self {
        Self {
            video_uid: value.uid,
            canister_id: value.canister_id,
            post_id: value.post_id,
            publisher_user_id: value.poster_principal,
            nsfw_probability: value.nsfw_probability,
        }
    }
}

/// PostItem compatible with existing code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostItem {
    pub video_id: String,
    pub canister_id: Principal,
    pub post_id: String,
    pub publisher_user_id: Principal,
    pub views: u64,
}
