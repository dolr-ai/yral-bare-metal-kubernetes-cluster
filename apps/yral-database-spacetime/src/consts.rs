//! Config constants for the rate-limits module.
//!
//! Mirrors `apps/yral-backend-canister/.../rate_limits/consts.rs` +
//! `PropertyRateLimitConfig` for the single live property ("VIDEOGEN").
//! No production caller writes these via admin methods (only integration
//! tests do), so they are hardcoded here. Changing a value = edit the
//! constant + `spacetime publish` (hot-swap, no data loss).

/// Default maximum requests per window for registered users.
pub const DEFAULT_MAX_REQUESTS_PER_WINDOW_REGISTERED: u64 = 1;
/// Default maximum requests per window for unregistered users.
pub const DEFAULT_MAX_REQUESTS_PER_WINDOW_UNREGISTERED: u64 = 0;
/// Default window duration in seconds (24 hours).
pub const DEFAULT_WINDOW_DURATION_SECONDS: u64 = 86_400;

/// The single live rate-limit property. Mirrors the canister's
/// `VIDEOGEN_RATE_LIMIT_PROPERTY = "VIDEOGEN"` (see
/// `apps/yral-mobile/.../RateLimitDataSourceImpl.kt`).
pub const VIDEOGEN_PROPERTY: &str = "VIDEOGEN";

/// Per-property config for `VIDEOGEN`. Mirrors a `PropertyRateLimitConfig`
/// row the canister would have had configured for "VIDEOGEN".
/// If we ever need a second property, add another constant + branch on
/// `property` in the helpers below (or, if runtime config becomes
/// necessary, reintroduce a `PropertyConfig` table — but not today).
pub const VIDEOGEN_MAX_REQUESTS_PER_WINDOW_REGISTERED: u64 = 1;
pub const VIDEOGEN_MAX_REQUESTS_PER_WINDOW_UNREGISTERED: u64 = 0;
pub const VIDEOGEN_WINDOW_DURATION_SECONDS: u64 = 86_400;
/// Aggregate cap across all users for the `VIDEOGEN` property, per the
/// property-wide window. `Some` enables the aggregate counter; `None`
/// disables it (matches `max_requests_per_property_all_users`).
pub const VIDEOGEN_MAX_REQUESTS_PER_PROPERTY_ALL_USERS: Option<u64> = None;
/// Window for the aggregate counter; defaults to 24h if `None`.
pub const VIDEOGEN_PROPERTY_WINDOW_DURATION_SECONDS: Option<u64> = None;

/// Blacklist of properties. Mirrors the canister's `blacklist: HashSet<String>`.
/// The magic string `"all"` blacklists every property. Empty in production.
pub const BLACKLIST: &[&str] = &[];

/// The SpacetimeDB `Identity` permitted to call write reducers + the
/// create/update/decrement video-gen paths. The Prakash backend connects
/// with this identity (token from config).
// TODO: replace with the Prakash backend's Maincloud identity.
pub const ADMIN_IDENTITY: spacetimedb::Identity = spacetimedb::Identity::ZERO;