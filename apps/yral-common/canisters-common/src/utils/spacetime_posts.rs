//! SpacetimeDB REST client for post reads.
//!
//! Replaces the IC canister `get_post_details_from_canister` call with a
//! SpacetimeDB `get_post_by_id` procedure call via REST. The SpacetimeDB
//! `PostV2` table stores all post data (backfilled from IC), including
//! `creator_principal_text` (the original IC Principal as text, needed for
//! CDN URLs, propic, profile enrichment).
//!
//! ## REST API format
//! SpacetimeDB wraps procedure results in an outer array: `[[<result>]]`.
//! `get_post_by_id` returns `Option<PostDetailsForFrontend>`:
//! - `Some(post)` → `[[0, {post fields}]]`
//! - `None` → `[[1, []]]`

use candid::Principal;
use serde::Deserialize;
use web_time::Duration;

use crate::{Error, Result};

use super::posts::PostDetails;
use super::profile::propic_from_principal;

/// SpacetimeDB REST response wrapper for `Option<PostDetailsForFrontend>`.
/// The REST API encodes `Option` as a 2-element array: `[variant_index, payload]`.
/// - `Some(post)` → `[0, {post JSON}]`
/// - `None` → `[1, []]`
#[derive(Deserialize)]
#[serde(untagged)]
enum OptionPostResponse {
    Some((u8, SpacetimePostDetails)),
    None((u8, serde_json::Value)),
}

/// SpacetimeDB `PostDetailsForFrontend` — the JSON shape returned by the
/// `get_post_by_id` procedure via REST.
///
/// Field names use camelCase (SpacetimeDB's JSON serialization convention).
/// `creator` is `Identity` serialized as `["0x<hex>"]`.
/// `created_at` is `Timestamp` serialized as `[<micros>]`.
#[derive(Deserialize)]
struct SpacetimePostDetails {
    id: String,
    description: String,
    hashtags: Vec<String>,
    #[serde(rename = "videoUid")]
    video_uid: String,
    /// SpacetimeDB `Identity` serializes as `["0x<hex>"]` in JSON.
    /// We don't use it — we use `creator_principal_text` instead.
    #[serde(default)]
    creator: serde_json::Value,
    #[serde(rename = "creatorPrincipalText")]
    creator_principal_text: String,
    /// `Timestamp` serializes as `[<micros_since_epoch>]`.
    #[serde(rename = "createdAt")]
    created_at: Vec<i64>,
    #[serde(rename = "totalViewCount")]
    total_view_count: u64,
    #[serde(rename = "likeCount")]
    like_count: u64,
    #[serde(rename = "likedByMe")]
    liked_by_me: bool,
}

/// A SpacetimeDB REST client for post reads.
///
/// Reads env vars:
/// - `SPACETIMEDB_URL` — e.g. `https://maincloud.spacetimedb.com`
/// - `SPACETIMEDB_DB_NAME` — e.g. `yral-database-spacetime-4lbo7`
/// - `SPACETIMEDB_ADMIN_TOKEN` — bearer token for authentication
#[derive(Clone)]
pub struct SpacetimePostsClient {
    client: reqwest::Client,
    url: String,
    db_name: String,
    token: String,
}

impl SpacetimePostsClient {
    /// Build from env vars. Returns `None` if any env var is missing
    /// (callers should fall back to IC canister reads in that case).
    pub fn from_env() -> Option<Self> {
        let url = std::env::var("SPACETIMEDB_URL").ok()?;
        let db_name = std::env::var("SPACETIMEDB_DB_NAME").ok()?;
        let token = std::env::var("SPACETIMEDB_ADMIN_TOKEN").ok()?;
        Some(Self {
            client: reqwest::Client::new(),
            url,
            db_name,
            token,
        })
    }

    fn call_url(&self, name: &str) -> String {
        format!(
            "{}/v1/database/{}/call/{}",
            self.url.trim_end_matches('/'),
            self.db_name,
            name
        )
    }

    /// Get a single post by ID from SpacetimeDB.
    /// Returns `Ok(None)` if the post doesn't exist or is deleted.
    pub async fn get_post_by_id(&self, post_id: &str) -> Result<Option<PostDetails>> {
        let resp = self
            .client
            .post(self.call_url("get_post_by_id"))
            .bearer_auth(&self.token)
            .json(&[post_id])
            .send()
            .await
            .map_err(|e| Error::YralCanister(format!("SpacetimeDB get_post_by_id request failed: {e}")))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(Error::YralCanister(format!(
                "SpacetimeDB get_post_by_id returned {status}: {body}"
            )));
        }

        let parsed: Vec<OptionPostResponse> = resp
            .json()
            .await
            .map_err(|e| Error::YralCanister(format!("SpacetimeDB get_post_by_id parse failed: {e}")))?;

        let result = parsed
            .into_iter()
            .next()
            .ok_or_else(|| Error::YralCanister("SpacetimeDB get_post_by_id returned empty array".to_string()))?;

        Ok(match result {
            OptionPostResponse::Some((_, post)) => {
                let principal = Principal::from_text(&post.creator_principal_text).unwrap_or(Principal::anonymous());
                let micros = post.created_at.first().copied().unwrap_or(0);
                let secs = (micros / 1_000_000).max(0) as u64;
                let nanos = ((micros % 1_000_000) * 1_000).max(0) as u32;
                Some(PostDetails {
                    canister_id: Principal::anonymous(), // Not used by mobile — enriched separately
                    post_id: post.id,
                    uid: post.video_uid,
                    description: post.description,
                    views: post.total_view_count,
                    likes: post.like_count,
                    display_name: None,
                    username: None,
                    propic_url: propic_from_principal(principal),
                    liked_by_user: Some(post.liked_by_me),
                    poster_principal: principal,
                    creator_follows_user: None,
                    user_follows_creator: None,
                    creator_bio: None,
                    hastags: post.hashtags,
                    is_nsfw: false,
                    hot_or_not_feed_ranking_score: Some(0),
                    created_at: Duration::new(secs, nanos),
                    nsfw_probability: 0.0,
                })
            }
            OptionPostResponse::None(_) => None,
        })
    }
}