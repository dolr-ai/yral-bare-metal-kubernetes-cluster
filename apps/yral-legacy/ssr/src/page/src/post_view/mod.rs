pub mod error;
pub mod overlay;
pub mod single_post;
pub mod video_loader;

use leptos::prelude::*;
use std::collections::HashMap;
use utils::types::PostId;
use yral_canisters_common::utils::posts::PostDetails;

#[derive(Clone, Default)]
pub struct PostDetailsCacheCtx {
    pub post_details: StoredValue<HashMap<PostId, PostDetails>>,
}
