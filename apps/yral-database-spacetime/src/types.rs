//! Shared types: enums + procedure response structs.

use spacetimedb::SpacetimeType;

/// Status of a video-generation request. Mirrors the canister's
/// `VideoGenRequestStatus` (the `Complete(String)` carries the result URL,
/// `Failed(String)` carries an error message).
#[derive(SpacetimeType, Debug, Clone, PartialEq)]
pub enum VideoGenRequestStatus {
    Pending,
    Processing,
    Complete(String),
    Failed(String),
}

/// Payment token type used for a video-gen request. Mirrors the canister's
/// `TokenType` (v2 of the create API). `Free` is the default.
#[derive(SpacetimeType, Debug, Clone, Copy, PartialEq)]
pub enum TokenType {
    Sats,
    Dolr,
    Free,
    YralProSubscription,
}

/// Response for the `get_rate_limit` procedure. Mirrors the canister's
/// `RateLimitStatus` (minus the `principal` field, which is passed in by
/// the caller and echoed back by the mobile if needed).
#[derive(SpacetimeType, Debug, Clone)]
pub struct RateLimitResponse {
    pub request_count: u64,
    pub window_start: u64,
    pub window_duration_seconds: u64,
    pub max_requests_per_window_per_user: u64,
    pub is_limited: bool,
}

/// Response for the `get_videogen_config` procedure. Mirrors the canister's
/// `PropertyRateLimitConfig` for the `VIDEOGEN` property — used by the
/// mobile app to show the credit ceiling for not-yet-registered users.
#[derive(SpacetimeType, Debug, Clone)]
pub struct VideogenConfigResponse {
    pub property: String,
    pub max_requests_per_window_registered: u64,
    pub max_requests_per_window_unregistered: u64,
    pub window_duration_seconds: u64,
    pub max_requests_per_property_all_users: Option<u64>,
    pub property_rate_limit_window_duration_seconds: Option<u64>,
}