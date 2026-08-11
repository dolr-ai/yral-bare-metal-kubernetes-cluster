use candid::Principal;
use serde::{Deserialize, Serialize};
use types::delegated_identity::DelegatedIdentityWire;

pub type PostId = (Principal, String);

#[derive(PartialEq, Clone)]
pub struct PostParams {
    pub canister_id: Principal,
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

#[derive(Serialize, Deserialize, Clone)]
pub struct NewIdentity {
    pub id_wire: DelegatedIdentityWire,
    pub fallback_username: Option<String>,
    pub email: Option<String>,
}

impl NewIdentity {
    pub fn new_without_username(id: DelegatedIdentityWire) -> Self {
        Self {
            id_wire: id,
            fallback_username: None,
            email: None,
        }
    }
}
