use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
};

use candid::Principal;
use global_constants::USERNAME_MAX_LEN;
use serde::{Deserialize, Serialize};
use username_gen::random_username_from_principal;
use web_time::Duration;

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PostDetails {
    pub canister_id: Principal, // canister id of the publishing canister.
    pub post_id: String,
    pub uid: String,
    pub description: String,
    pub views: u64,
    pub likes: u64,
    pub display_name: Option<String>,
    pub username: Option<String>,
    pub propic_url: String,
    /// Whether post is liked by the authenticated
    /// user or not, None if unknown
    pub liked_by_user: Option<bool>,
    pub poster_principal: Principal,
    pub creator_follows_user: Option<bool>,
    pub user_follows_creator: Option<bool>,
    pub creator_bio: Option<String>,
    pub hastags: Vec<String>,
    pub is_nsfw: bool,
    pub hot_or_not_feed_ranking_score: Option<u64>,
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
    pub fn is_hot_or_not(&self) -> bool {
        self.hot_or_not_feed_ranking_score.is_some()
    }

    pub fn username_or_principal(&self) -> String {
        self.username
            .clone()
            .unwrap_or_else(|| self.poster_principal.to_text())
    }

    /// Get the user's username
    /// or a consistent random username
    /// WARN: do not use this method for URLs
    /// use `username_or_principal` instead
    pub fn username_or_fallback(&self) -> String {
        self.username.clone().unwrap_or_else(|| {
            random_username_from_principal(self.poster_principal, USERNAME_MAX_LEN)
        })
    }

    pub fn display_name_or_fallback(&self) -> String {
        self.display_name
            .clone()
            .unwrap_or_else(|| self.username_or_fallback())
    }
}
