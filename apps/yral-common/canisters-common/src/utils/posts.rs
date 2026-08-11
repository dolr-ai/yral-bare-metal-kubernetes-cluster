use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
};

use candid::Principal;
use canisters_client::{
    ic::USER_INFO_SERVICE_ID,
    user_post_service::{
        PostDetailsForFrontend as PostServicePostDetailsForFrontend, Result3 as PostServiceResult3,
    },
};
use futures_util::try_join;
use global_constants::USERNAME_MAX_LEN;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use username_gen::random_username_from_principal;
use web_time::Duration;

use crate::{Canisters, Error, Result};

use super::profile::propic_from_principal;

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
    pub fn from_service_post(
        username: Option<String>,
        canister_id: Principal,
        post_details: PostServicePostDetailsForFrontend,
    ) -> Self {
        Self {
            canister_id,
            post_id: post_details.id,
            uid: post_details.video_uid,
            description: post_details.description,
            views: post_details.total_view_count,
            likes: post_details.like_count,
            display_name: None,
            propic_url: propic_from_principal(post_details.created_by_user_principal_id),
            liked_by_user: Some(post_details.liked_by_me),
            poster_principal: post_details.creator_principal,
            creator_follows_user: None,
            user_follows_creator: None,
            creator_bio: None,
            hastags: post_details.hashtags,
            is_nsfw: false,
            hot_or_not_feed_ranking_score: Some(0),
            created_at: Duration::new(
                post_details.created_at.secs_since_epoch,
                post_details.created_at.nanos_since_epoch,
            ),
            nsfw_probability: 0.0,
            username,
        }
    }

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

#[derive(Debug, Deserialize)]
struct NsfwApiResponse {
    nsfw_probability: f32,
}

impl<const A: bool> Canisters<A> {
    #[instrument(skip(self))]
    async fn fetch_nsfw_probability(&self, video_uid: &str) -> Result<f32> {
        let url = format!("https://offchain.yral.com/api/v2/posts/nsfw_prob/{video_uid}");
        let response = reqwest::get(&url).await?;
        let nsfw_response: NsfwApiResponse = response.json().await?;
        Ok(nsfw_response.nsfw_probability)
    }

    /// Fetch post details from the IC canister, enriched with creator metadata
    /// and NSFW probability. Still used by yral-legacy until SpacetimeDB rewiring.
    #[tracing::instrument(skip(self))]
    pub async fn get_post_details(
        &self,
        user_canister: Principal,
        post_id: String,
    ) -> Result<Option<PostDetails>> {
        let post_details = self
            .get_post_details_with_nsfw_info(user_canister, post_id, None)
            .await?;
        Ok(post_details)
    }

    #[tracing::instrument(skip(self))]
    async fn get_post_details_with_nsfw_info(
        &self,
        user_canister: Principal,
        post_id: String,
        nsfw_probability: Option<f32>,
    ) -> Result<Option<PostDetails>> {
        if user_canister != USER_INFO_SERVICE_ID {
            return Err(Error::YralCanister(format!(
                "User canister {} is not USER_INFO_SERVICE_ID; individual_user_template canisters have been decommissioned",
                user_canister
            )));
        }

        let post_service_canister = self.user_post_service().await;
        let post_details = post_service_canister
            .get_individual_post_details_by_id_for_user(
                post_id.into(),
                post_service_canister.1.get_principal().unwrap(),
            )
            .await?;

        let PostServiceResult3::Ok(post_details) = post_details else {
            return Ok(None);
        };

        let mut post_details = PostDetails::from_service_post(None, user_canister, post_details);

        let creator_principal = post_details.poster_principal;
        let (creator_meta, nsfw_prob) = try_join!(
            async {
                let meta = self
                    .metadata_client
                    .get_user_metadata_v2(creator_principal.to_text())
                    .await?;
                Ok::<_, Error>(meta)
            },
            async {
                if let Some(nsfw_prob) = nsfw_probability {
                    return Ok(nsfw_prob);
                }
                Ok(self
                    .fetch_nsfw_probability(&post_details.uid)
                    .await
                    .inspect_err(|e| {
                        log::warn!(
                            "Failed to fetch NSFW probability for video {}: {}, defaulting to 1.0",
                            post_details.uid,
                            e
                        );
                    })
                    .unwrap_or(1.0))
            }
        )?;

        post_details.nsfw_probability = nsfw_prob;
        post_details.username = creator_meta.map(|m| m.user_name).filter(|s| !s.is_empty());

        Ok(Some(post_details))
    }
}
