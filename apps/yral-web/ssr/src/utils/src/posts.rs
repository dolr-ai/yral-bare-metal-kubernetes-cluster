use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

use leptos::prelude::RwSignal;
use serde::{Deserialize, Serialize};
use username_gen::random_username_from_principal;
use web_time::Duration;

const USERNAME_MAX_LEN: usize = 29;

/// Post details for frontend display. Populated from SpacetimeDB.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PostDetails {
    pub canister_id: String,
    pub post_id: String,
    pub uid: String,
    pub description: String,
    pub views: u64,
    pub likes: u64,
    pub display_name: Option<String>,
    pub username: Option<String>,
    pub propic_url: String,
    pub liked_by_user: Option<bool>,
    pub poster_principal: String,
    pub creator_follows_user: Option<bool>,
    pub user_follows_creator: Option<bool>,
    pub creator_bio: Option<String>,
    pub hastags: Vec<String>,
    pub is_nsfw: bool,
    pub created_at: Duration,
    pub nsfw_probability: f32,
}

impl PartialOrd for PostDetails {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PostDetails {
    fn cmp(&self, other: &Self) -> Ordering {
        self.created_at.cmp(&other.created_at)
    }
}

impl Hash for PostDetails {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.canister_id.hash(state);
        self.post_id.hash(state);
    }
}

impl Eq for PostDetails {}

impl PostDetails {
    pub fn username_or_principal(&self) -> String {
        self.username
            .clone()
            .unwrap_or_else(|| self.poster_principal.clone())
    }

    /// Get the user's username
    /// or a consistent random username
    /// WARN: do not use this method for URLs
    /// use `username_or_principal` instead
    pub fn username_or_fallback(&self) -> String {
        self.username.clone().unwrap_or_else(|| {
            random_username_from_principal(&self.poster_principal, USERNAME_MAX_LEN)
        })
    }

    pub fn display_name_or_fallback(&self) -> String {
        self.display_name
            .clone()
            .unwrap_or_else(|| self.username_or_fallback())
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct FetchCursor {
    pub start: u64,
    pub limit: u64,
}

impl Default for FetchCursor {
    fn default() -> Self {
        Self {
            start: 0,
            limit: 10,
        }
    }
}

impl FetchCursor {
    pub fn advance(&mut self) {
        self.start += self.limit;
        self.limit = 25;
    }

    pub fn set_limit(&mut self, limit: u64) {
        self.limit = limit;
    }

    pub fn advance_and_set_limit(&mut self, limit: u64) {
        self.start += self.limit;
        self.limit = limit;
    }
}

#[derive(Clone, Default)]
pub struct FeedPostCtx<DetailResolver: Sync + Send + 'static = PostDetails> {
    pub key: usize,
    pub value: RwSignal<Option<DetailResolver>>,
}
