pub mod error;
pub mod overlay;
pub mod single_post;
pub mod video_loader;

use leptos::prelude::*;
use std::collections::HashMap;
use utils::posts::PostDetails;
use utils::types::PostId;

#[derive(Clone, Default)]
pub struct PostDetailsCacheCtx {
    pub post_details: StoredValue<HashMap<PostId, PostDetails>>,
}
