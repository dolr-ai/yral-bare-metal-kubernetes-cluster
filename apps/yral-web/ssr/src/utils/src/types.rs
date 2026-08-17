use serde::{Deserialize, Serialize};

pub type PostId = (String, String);

#[derive(PartialEq, Clone)]
pub struct PostParams {
    pub canister_id: String,
    pub post_id: String,
}

#[derive(PartialEq, Debug, Eq)]
pub enum PostStatus {
    BannedForExplicitness,
    Draft,
    BannedDueToUserReporting,
    Uploaded,
    CheckingExplicitness,
    ReadyToView,
    Transcoding,
    Deleted,
}

/// Identity for the authenticated user.
/// Replaces the old `DelegatedIdentityWire` (IC identity) with
/// a simple JWT-based identity. The `user_id` is the JWT `sub` claim
/// (OAuth sub or UUID for AI accounts). The `id_token` and
/// `refresh_token` are the yral-auth JWTs.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NewIdentity {
    pub user_id: String,
    pub id_token: String,
    pub refresh_token: String,
    pub fallback_username: Option<String>,
    pub email: Option<String>,
}

impl NewIdentity {
    pub fn new_without_username(user_id: String, id_token: String, refresh_token: String) -> Self {
        Self {
            user_id,
            id_token,
            refresh_token,
            fallback_username: None,
            email: None,
        }
    }
}
