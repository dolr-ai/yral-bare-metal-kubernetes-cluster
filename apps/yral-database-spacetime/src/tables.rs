//! Table definitions.
//!
//! All tables are `public` so procedure reads + SDK clients can access them.
//! ICP principals are stored as `String` (canonical text form).

use spacetimedb::{SpacetimeType, Table};

use crate::types::VideoGenRequestStatus;

/// Per-(principal, property) rate-limit counter. Mirrors the canister's
/// `rate_limits: StableBTreeMap<RateLimitKey, RateLimitEntry>`.
#[spacetimedb::table(
    accessor = rate_limit,
    public,
    index(accessor = by_principal_property, btree(columns = [principal, property]))
)]
pub struct RateLimit {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    /// ICP principal in canonical text form (e.g. "2vxsx-fae-...").
    pub principal: String,
    /// Property string (e.g. "VIDEOGEN").
    pub property: String,
    /// Number of requests in the current window.
    pub request_count: u64,
    /// Window start, Unix seconds (matches canister's seconds granularity).
    pub window_start: u64,
    /// Per-entry override of `max_requests_per_window`. `None` → use the
    /// property/default config. Mirrors `RateLimitEntry.config.max_requests_per_window`.
    pub config_max: Option<u64>,
    /// Per-entry override of `window_duration_seconds`. `None` → use the
    /// property/default config. Mirrors `RateLimitEntry.config.window_duration_seconds`.
    pub config_window: Option<u64>,
}

/// Aggregate per-property counter for `max_requests_per_property_all_users`.
/// Mirrors the canister's `property_rate_limits: StableBTreeMap<String, RateLimitEntry>`.
#[spacetimedb::table(accessor = property_rate_limit, public)]
pub struct PropertyRateLimit {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    // `#[unique]` auto-creates a unique btree index; no separate index(...) needed.
    #[unique]
    pub property: String,
    pub request_count: u64,
    pub window_start: u64,
}

/// A single video-generation request. Mirrors the canister's
/// `video_gen_requests: StableBTreeMap<VideoGenRequestKey, VideoGenRequest>`.
#[spacetimedb::table(
    accessor = video_gen_request,
    public,
    index(accessor = by_principal_counter, btree(columns = [principal, counter]))
)]
pub struct VideoGenRequest {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    /// ICP principal in canonical text form.
    pub principal: String,
    /// Per-user monotonic counter (matches canister's
    /// `UserRequestCounter`). Used together with `principal` to address a
    /// request (mirrors `VideoGenRequestKey { principal, counter }`).
    pub counter: u64,
    pub model_name: String,
    pub prompt: String,
    pub status: VideoGenRequestStatus,
    pub created_at: u64,
    pub updated_at: u64,
    pub payment_amount: Option<String>,
    pub token_type: Option<crate::types::TokenType>,
}

/// Per-user monotonic counter for video-gen requests. Mirrors the canister's
/// `user_request_counters: StableBTreeMap<Principal, u64>`.
#[spacetimedb::table(accessor = user_request_counter, public)]
pub struct UserRequestCounter {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    // `#[unique]` auto-creates a unique btree index; no separate index(...) needed.
    #[unique]
    pub principal: String,
    pub counter: u64,
}