//! Yral rate-limits + video-generation tracking module for SpacetimeDB.
//!
//! Migrated from the ICP `rate_limits` canister
//! (`apps/yral-backend-canister/src/canister/rate_limits/`). Only the latest
//! v2/v1 variants of the canister APIs are implemented; admin/config methods
//! with zero production callers were dropped (config is static constants
//! here, not DB rows or `init`-seeded).
//!
//! ## Architecture
//! - **Writers** (the Prakash `yral-video-storage-service` backend, at
//!   `https://storage-interface.prakash.yral.com`) call the reducers below
//!   via the `spacetimedb-sdk` (typed bindings) over WebSocket, gated by
//!   `ADMIN_IDENTITY`.
//! - **Readers** (the yral-mobile app) call the procedures below via REST
//!   (`POST /v1/database/{db}/call/:name`, JSON array body → typed JSON
//!   return). No raw SQL is sent from the client.
//! - **ICP principals** are stored as `String` (canonical text form). The
//!   text form is lossless: bytes / `anonymous` / comparisons / hashing are
//!   all re-derivable. This keeps the module free of the `candid`/`ic-agent`
//!   dependency.
//!
//! ## Config
//! All rate-limit config is static Rust constants. Changing a value = edit
//! the constant + `spacetime publish` (hot-swap, no data loss). There is no
//! `init` reducer and no `PropertyConfig`/`Blacklist` table.

use spacetimedb::{ProcedureContext, ReducerContext, SpacetimeType, Table};

// ── Config constants ───────────────────────────────────────────────────────
// Mirrors `apps/yral-backend-canister/.../rate_limits/consts.rs` +
// `PropertyRateLimitConfig` for the single live property ("VIDEOGEN").
// No production caller writes these via admin methods (only integration
// tests do), so they are hardcoded here.

/// Default maximum requests per window for registered users.
const DEFAULT_MAX_REQUESTS_PER_WINDOW_REGISTERED: u64 = 1;
/// Default maximum requests per window for unregistered users.
const DEFAULT_MAX_REQUESTS_PER_WINDOW_UNREGISTERED: u64 = 0;
/// Default window duration in seconds (24 hours).
const DEFAULT_WINDOW_DURATION_SECONDS: u64 = 86_400;

/// The single live rate-limit property. Mirrors the canister's
/// `VIDEOGEN_RATE_LIMIT_PROPERTY = "VIDEOGEN"` (see
/// `apps/yral-mobile/.../RateLimitDataSourceImpl.kt`).
const VIDEOGEN_PROPERTY: &str = "VIDEOGEN";

/// Per-property config for `VIDEOGEN`. Mirrors a `PropertyRateLimitConfig`
/// row the canister would have had configured for "VIDEOGEN".
/// If we ever need a second property, add another constant + branch on
/// `property` in the helpers below (or, if runtime config becomes
/// necessary, reintroduce a `PropertyConfig` table — but not today).
const VIDEOGEN_MAX_REQUESTS_PER_WINDOW_REGISTERED: u64 = 1;
const VIDEOGEN_MAX_REQUESTS_PER_WINDOW_UNREGISTERED: u64 = 0;
const VIDEOGEN_WINDOW_DURATION_SECONDS: u64 = 86_400;
/// Aggregate cap across all users for the `VIDEOGEN` property, per the
/// property-wide window. `Some` enables the aggregate counter; `None`
/// disables it (matches `max_requests_per_property_all_users`).
const VIDEOGEN_MAX_REQUESTS_PER_PROPERTY_ALL_USERS: Option<u64> = None;
/// Window for the aggregate counter; defaults to 24h if `None`.
const VIDEOGEN_PROPERTY_WINDOW_DURATION_SECONDS: Option<u64> = None;

/// Blacklist of properties. Mirrors the canister's `blacklist: HashSet<String>`.
/// The magic string `"all"` blacklists every property. Empty in production.
const BLACKLIST: &[&str] = &[];

/// The SpacetimeDB `Identity` permitted to call write reducers + the
/// create/update/decrement video-gen paths. The Prakash backend connects
/// with this identity (token from config). Replaced by a real identity
/// before publish (TODO: wire from `mise.toml [env]`/fnox once known).
// TODO: replace with the Prakash backend's Maincloud identity.
const ADMIN_IDENTITY: spacetimedb::Identity = spacetimedb::Identity::ZERO;

// ── Types ──────────────────────────────────────────────────────────────────

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

// ── Tables ──────────────────────────────────────────────────────────────────
// All tables are `public` so procedure reads + SDK clients can access them.
// ICP principals are stored as `String` (canonical text form).

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
#[spacetimedb::table(
    accessor = property_rate_limit,
    public,
    // `#[unique]` on `property` below auto-creates a unique btree index;
    // no separate index(...) needed.
)]
pub struct PropertyRateLimit {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
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
    pub token_type: Option<TokenType>,
}

/// Per-user monotonic counter for video-gen requests. Mirrors the canister's
/// `user_request_counters: StableBTreeMap<Principal, u64>`.
#[spacetimedb::table(
    accessor = user_request_counter,
    public,
    // `#[unique]` on `principal` below auto-creates a unique btree index;
    // no separate index(...) needed.
)]
pub struct UserRequestCounter {
    #[primary_key]
    #[auto_inc]
    pub id: u64,
    #[unique]
    pub principal: String,
    pub counter: u64,
}

// ── Response types for procedures (mobile read path) ───────────────────────

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

// ── Helpers ───────────────────────────────────────────────────────────────

fn now_secs(ctx: &ReducerContext) -> u64 {
    // SpacetimeDB `Timestamp` is microseconds since the Unix epoch; the
    // canister uses `ic_cdk::api::time() / 1_000_000_000` (seconds).
    (ctx.timestamp.to_micros_since_unix_epoch() / 1_000_000) as u64
}

fn now_secs_proc(ctx: &ProcedureContext) -> u64 {
    (ctx.timestamp.to_micros_since_unix_epoch() / 1_000_000) as u64
}

fn is_admin(ctx: &ReducerContext) -> bool {
    ctx.sender() == ADMIN_IDENTITY
}

fn is_blacklisted(property: &str) -> bool {
    BLACKLIST
        .iter()
        .any(|b| *b == property || *b == "all")
}

/// Resolve the per-principal `(max_requests, window_duration)` by the
/// canister's tier precedence: entry override → property constant → default
/// constant. `is_registered` selects the registered vs unregistered branch
/// at the property/default tiers (the entry override is registration-agnostic,
/// matching the canister's `entry.config`).
fn resolve_user_limit(
    entry: Option<&RateLimit>,
    property: &str,
    is_registered: bool,
) -> (u64, u64) {
    // Tier 1: per-entry override.
    if let Some(e) = entry
        && let (Some(max), Some(win)) = (e.config_max, e.config_window)
    {
        return (max, win);
    }
    // Tier 2: property config (only VIDEOGEN is configured).
    if property == VIDEOGEN_PROPERTY {
        let max = if is_registered {
            VIDEOGEN_MAX_REQUESTS_PER_WINDOW_REGISTERED
        } else {
            VIDEOGEN_MAX_REQUESTS_PER_WINDOW_UNREGISTERED
        };
        return (max, VIDEOGEN_WINDOW_DURATION_SECONDS);
    }
    // Tier 3: default config.
    let max = if is_registered {
        DEFAULT_MAX_REQUESTS_PER_WINDOW_REGISTERED
    } else {
        DEFAULT_MAX_REQUESTS_PER_WINDOW_UNREGISTERED
    };
    (max, DEFAULT_WINDOW_DURATION_SECONDS)
}

/// Property-wide aggregate limit for a property, or `None` if not configured.
/// Mirrors `PropertyRateLimitConfig.max_requests_per_property_all_users`.
fn property_aggregate_limit(property: &str) -> Option<(u64, u64)> {
    if property == VIDEOGEN_PROPERTY {
        VIDEOGEN_MAX_REQUESTS_PER_PROPERTY_ALL_USERS.map(|limit| {
            (
                limit,
                VIDEOGEN_PROPERTY_WINDOW_DURATION_SECONDS.unwrap_or(86_400),
            )
        })
    } else {
        None
    }
}

/// Is the property-wide aggregate counter at the limit right now?
fn is_property_daily_rate_limited(ctx: &ReducerContext, property: &str) -> bool {
    let Some((limit, window_duration)) = property_aggregate_limit(property) else {
        return false;
    };
    let now = now_secs(ctx);
    if let Some(entry) = ctx
        .db
        .property_rate_limit()
        .iter()
        .find(|r| r.property == property)
    {
        if now < entry.window_start + window_duration {
            return entry.request_count >= limit;
        }
    }
    false
}

/// Increment the per-(principal, property) counter with window rollover.
/// Mirrors `CanisterData::increment_request_with_property`.
fn increment_request(ctx: &ReducerContext, principal: &str, property: &str) {
    let now = now_secs(ctx);
    let existing = ctx
        .db
        .rate_limit()
        .iter()
        .find(|r| r.principal == principal && r.property == property);
    let window_duration = if let Some(e) = &existing {
        e.config_window.unwrap_or_else(|| {
            if property == VIDEOGEN_PROPERTY {
                VIDEOGEN_WINDOW_DURATION_SECONDS
            } else {
                DEFAULT_WINDOW_DURATION_SECONDS
            }
        })
    } else if property == VIDEOGEN_PROPERTY {
        VIDEOGEN_WINDOW_DURATION_SECONDS
    } else {
        DEFAULT_WINDOW_DURATION_SECONDS
    };
    let new_entry = match existing {
        Some(mut e) => {
            if now >= e.window_start + window_duration {
                RateLimit {
                    id: e.id,
                    principal: e.principal,
                    property: e.property,
                    request_count: 1,
                    window_start: now,
                    config_max: e.config_max,
                    config_window: e.config_window,
                }
            } else {
                e.request_count += 1;
                e
            }
        }
        None => RateLimit {
            id: 0,
            principal: principal.to_string(),
            property: property.to_string(),
            request_count: 1,
            window_start: now,
            config_max: None,
            config_window: None,
        },
    };
    if new_entry.id == 0 {
        ctx.db.rate_limit().insert(new_entry);
    } else {
        ctx.db.rate_limit().id().update(new_entry);
    }

    // Then increment the aggregate counter if configured.
    if property_aggregate_limit(property).is_some() {
        increment_property_counter(ctx, property, now);
    }
}

/// Increment only the property-wide aggregate counter (paid path).
/// Mirrors `CanisterData::increment_paid_request_property_only`.
fn increment_property_counter(ctx: &ReducerContext, property: &str, now: u64) {
    let Some((_, window_duration)) = property_aggregate_limit(property) else {
        return;
    };
    let existing = ctx
        .db
        .property_rate_limit()
        .iter()
        .find(|r| r.property == property);
    let new_entry = match existing {
        Some(mut e) => {
            if now >= e.window_start + window_duration {
                PropertyRateLimit {
                    id: e.id,
                    property: e.property,
                    request_count: 1,
                    window_start: now,
                }
            } else {
                e.request_count += 1;
                e
            }
        }
        None => PropertyRateLimit {
            id: 0,
            property: property.to_string(),
            request_count: 1,
            window_start: now,
        },
    };
    if new_entry.id == 0 {
        ctx.db.property_rate_limit().insert(new_entry);
    } else {
        ctx.db.property_rate_limit().id().update(new_entry);
    }
}

/// Decrement the per-(principal, property) counter, only within the same
/// window and when count > 0. Mirrors `decrement_request_with_property`.
fn decrement_request(ctx: &ReducerContext, principal: &str, property: &str) {
    let now = now_secs(ctx);
    if let Some(mut e) = ctx
        .db
        .rate_limit()
        .iter()
        .find(|r| r.principal == principal && r.property == property)
    {
        let window_duration = e.config_window.unwrap_or_else(|| {
            if property == VIDEOGEN_PROPERTY {
                VIDEOGEN_WINDOW_DURATION_SECONDS
            } else {
                DEFAULT_WINDOW_DURATION_SECONDS
            }
        });
        if now < e.window_start + window_duration && e.request_count > 0 {
            e.request_count -= 1;
            ctx.db.rate_limit().id().update(e);
        }
    }
    // Decrement aggregate counter too.
    if property_aggregate_limit(property).is_some() {
        decrement_property_counter(ctx, property, now);
    }
}

/// Decrement only the property-wide aggregate counter (paid failed path).
/// Mirrors `CanisterData::decrement_property_counter_only`.
fn decrement_property_counter(ctx: &ReducerContext, property: &str, now: u64) {
    let Some((_, window_duration)) = property_aggregate_limit(property) else {
        return;
    };
    if let Some(mut e) = ctx
        .db
        .property_rate_limit()
        .iter()
        .find(|r| r.property == property)
    {
        if now < e.window_start + window_duration && e.request_count > 0 {
            e.request_count -= 1;
            ctx.db.property_rate_limit().id().update(e);
        }
    }
}

/// Is the principal rate-limited for the property right now? Mirrors
/// `CanisterData::is_rate_limited_with_property` (blacklist → property-wide
/// → per-principal).
fn is_rate_limited(
    ctx: &ReducerContext,
    principal: &str,
    property: &str,
    is_registered: bool,
) -> bool {
    if is_blacklisted(property) {
        return true;
    }
    if is_property_daily_rate_limited(ctx, property) {
        return true;
    }
    let entry = ctx
        .db
        .rate_limit()
        .iter()
        .find(|r| r.principal == principal && r.property == property);
    let (max_requests, window_duration) =
        resolve_user_limit(entry.as_ref(), property, is_registered);
    let now = now_secs(ctx);
    let within_window = match &entry {
        Some(e) => now < e.window_start + window_duration,
        None => false,
    };
    within_window && (max_requests == 0 || entry.map_or(0, |e| e.request_count) >= max_requests)
}

// ── Reducers (write path, admin-gated) ──────────────────────────────────────
// Only the latest v2/v1 variants are implemented; the v0 `increment_request_count`
// / `check_rate_limit` / `create_video_generation_request` (v0/v1) are dropped
// (no production caller). All reducers return `Result<(), String>`.

/// Increment the per-(principal, property) counter (unpaid path). Mirrors
/// the canister's `increment_request_count`. Admin-gated.
#[spacetimedb::reducer]
pub fn increment_rate_limit(
    ctx: &ReducerContext,
    principal: String,
    property: String,
    is_registered: bool,
) -> Result<(), String> {
    if !is_admin(ctx) {
        return Err("Unauthorized: admin only".to_string());
    }
    if is_rate_limited(ctx, &principal, &property, is_registered) {
        return Err("Rate limit exceeded".to_string());
    }
    increment_request(ctx, &principal, &property);
    Ok(())
}

/// Paid-aware increment. Mirrors the canister's `increment_request_count_v1`:
/// paid → only the property-aggregate counter; unpaid → full per-user path.
#[spacetimedb::reducer]
pub fn increment_rate_limit_paid(
    ctx: &ReducerContext,
    principal: String,
    property: String,
    is_registered: bool,
    is_paid: bool,
    payment_amount: Option<String>,
) -> Result<(), String> {
    if !is_admin(ctx) {
        return Err("Unauthorized: admin only".to_string());
    }
    if is_paid {
        if is_property_daily_rate_limited(ctx, &property) {
            return Err("Property rate limit exceeded for paid request".to_string());
        }
        increment_property_counter(ctx, &property, now_secs(ctx));
        let _ = payment_amount; // payment_amount is recorded on the VideoGenRequest, not here.
        return Ok(());
    }
    if is_rate_limited(ctx, &principal, &property, is_registered) {
        return Err("Rate limit exceeded".to_string());
    }
    increment_request(ctx, &principal, &property);
    Ok(())
}

/// Decrement the per-(principal, property) counter (unpaid failed path).
/// Mirrors `decrement_video_generation_counter`.
#[spacetimedb::reducer]
pub fn decrement_rate_limit(
    ctx: &ReducerContext,
    principal: String,
    property: String,
) -> Result<(), String> {
    if !is_admin(ctx) {
        return Err("Unauthorized: admin only".to_string());
    }
    if property.is_empty() || property.len() > 50 {
        return Err("Invalid property".to_string());
    }
    decrement_request(ctx, &principal, &property);
    Ok(())
}

/// Paid-aware decrement (rollback on Failed). Mirrors the canister's
/// `decrement_video_generation_counter_v1`: paid → only property counter;
/// unpaid → both user + property counters. The caller (Prakash backend)
/// chooses which path based on the VideoGenRequest's `payment_amount` —
/// call `decrement_rate_limit` (unpaid, both counters) or
/// `decrement_rate_limit_paid_property_only` (paid, property-only). This
/// reducer covers the unpaid-style "both counters" rollback for parity with
/// the canister's v1 decrement (which did both unconditionally); the
/// backend should prefer the two specific reducers above/below.
/// `decrement_rate_limit_paid_property_only` (paid, property-only).
#[spacetimedb::reducer]
pub fn decrement_rate_limit_paid(
    ctx: &ReducerContext,
    principal: String,
    property: String,
) -> Result<(), String> {
    if !is_admin(ctx) {
        return Err("Unauthorized: admin only".to_string());
    }
    if property.is_empty() || property.len() > 50 {
        return Err("Invalid property".to_string());
    }
    decrement_request(ctx, &principal, &property);
    Ok(())
}

/// Decrement only the property-aggregate counter (paid failed path).
#[spacetimedb::reducer]
pub fn decrement_rate_limit_paid_property_only(
    ctx: &ReducerContext,
    property: String,
) -> Result<(), String> {
    if !is_admin(ctx) {
        return Err("Unauthorized: admin only".to_string());
    }
    if property.is_empty() || property.len() > 50 {
        return Err("Invalid property".to_string());
    }
    decrement_property_counter(ctx, &property, now_secs(ctx));
    Ok(())
}

/// Create a new video-generation request after checking rate limits.
/// Mirrors the canister's `create_video_generation_request_v2` (v2 drops the
/// prompt-length check — only model/property are validated — preserve that
/// quirk). The Prakash backend reads the inserted row via the SDK table
/// accessor (reducers can't return data to callers).
#[spacetimedb::reducer]
pub fn create_video_gen_request(
    ctx: &ReducerContext,
    principal: String,
    model_name: String,
    prompt: String,
    property: String,
    token_type: TokenType,
    is_registered: bool,
    is_paid: bool,
    payment_amount: Option<String>,
) -> Result<(), String> {
    if !is_admin(ctx) {
        return Err("Unauthorized: admin only".to_string());
    }
    // Validate inputs (v2 quirk: no prompt-length check).
    if model_name.is_empty() || model_name.len() > 100 {
        return Err("Invalid model name".to_string());
    }
    if property.is_empty() || property.len() > 50 {
        return Err("Invalid property".to_string());
    }

    // Check + increment counters based on payment status.
    if is_paid {
        if is_property_daily_rate_limited(ctx, &property) {
            return Err("Property rate limit exceeded for paid request".to_string());
        }
        increment_property_counter(ctx, &property, now_secs(ctx));
    } else {
        if is_rate_limited(ctx, &principal, &property, is_registered) {
            return Err("Rate limit exceeded".to_string());
        }
        increment_request(ctx, &principal, &property);
    }

    // Bump the per-user monotonic counter.
    let new_counter = match ctx
        .db
        .user_request_counter()
        .iter()
        .find(|r| r.principal == principal)
    {
        Some(mut c) => {
            c.counter += 1;
            let new_counter = c.counter;
            ctx.db.user_request_counter().id().update(c);
            new_counter
        }
        None => {
            let c = UserRequestCounter {
                id: 0,
                principal: principal.clone(),
                counter: 1,
            };
            ctx.db.user_request_counter().insert(c);
            1
        }
    };

    let now = now_secs(ctx);
    ctx.db.video_gen_request().insert(VideoGenRequest {
        id: 0,
        principal: principal.clone(),
        counter: new_counter,
        model_name,
        prompt,
        status: VideoGenRequestStatus::Pending,
        created_at: now,
        updated_at: now,
        payment_amount,
        token_type: Some(token_type),
    });
    log::info!(
        "Created video-gen request for {principal} counter={new_counter} property={property}"
    );
    Ok(())
}

/// Update a video-generation request's status. Mirrors
/// `update_video_generation_status`. Identified by `(principal, counter)`
/// (the canister's `VideoGenRequestKey`).
#[spacetimedb::reducer]
pub fn update_video_gen_status(
    ctx: &ReducerContext,
    principal: String,
    counter: u64,
    status: VideoGenRequestStatus,
) -> Result<(), String> {
    if !is_admin(ctx) {
        return Err("Unauthorized: admin only".to_string());
    }
    let now = now_secs(ctx);
    if let Some(mut r) = ctx
        .db
        .video_gen_request()
        .iter()
        .find(|r| r.principal == principal && r.counter == counter)
    {
        r.status = status;
        r.updated_at = now;
        ctx.db.video_gen_request().id().update(r);
        Ok(())
    } else {
        Err("Video generation request not found".to_string())
    }
}

// ── Procedures (read path, mobile) ─────────────────────────────────────────
// Procedures can open transactions (`ctx.with_tx`) to read table data and
// return typed `SpacetimeType` values to the caller — over REST
// (`POST /v1/database/{db}/call/:name`, JSON array body) or the WS SDK
// (`conn.procedures().foo_then(...)`). No raw SQL on the client.
//
// The quota read is already unauthenticated in the canister (anyone can call
// `get_rate_limit_status` with any principal); the procedures preserve that
// behavior. `ctx.sender()` is a SpacetimeDB Identity, NOT the ICP principal,
// so we pass the ICP principal as an arg (matching the canister's semantics).

/// Read a principal's rate-limit status for a property. Mirrors the
/// canister's `get_rate_limit_status` (registered-user path on the mobile).
#[spacetimedb::procedure]
pub fn get_rate_limit(
    ctx: &mut ProcedureContext,
    principal: String,
    property: String,
    is_registered: bool,
) -> RateLimitResponse {
    // Capture the timestamp before `with_tx` to avoid borrowing `ctx`
    // (which is mutably borrowed by `with_tx`) inside the closure.
    let now = now_secs_proc(ctx);
    ctx.with_tx(|tx| {
        if is_blacklisted(&property) {
            return RateLimitResponse {
                request_count: 1,
                window_start: now,
                window_duration_seconds: 0,
                max_requests_per_window_per_user: 0,
                is_limited: true,
            };
        }
        // Property-wide aggregate limit.
        if let Some((limit, window_duration)) = property_aggregate_limit(&property) {
            if let Some(e) = tx
                .db
                .property_rate_limit()
                .iter()
                .find(|r| r.property == property)
            {
                if now < e.window_start + window_duration && e.request_count >= limit {
                    let entry = tx
                        .db
                        .rate_limit()
                        .iter()
                        .find(|r| r.principal == principal && r.property == property);
                    return RateLimitResponse {
                        request_count: entry.as_ref().map_or(0, |e| e.request_count),
                        window_start: entry.as_ref().map_or(now, |e| e.window_start),
                        window_duration_seconds: window_duration,
                        max_requests_per_window_per_user: limit,
                        is_limited: true,
                    };
                }
            }
        }
        // Per-principal limit.
        let entry = tx
            .db
            .rate_limit()
            .iter()
            .find(|r| r.principal == principal && r.property == property);
        let (max_requests, window_duration) =
            resolve_user_limit(entry.as_ref(), &property, is_registered);
        let request_count = entry.as_ref().map_or(0, |e| e.request_count);
        let window_start = entry.as_ref().map_or(now, |e| e.window_start);
        let within_window = match &entry {
            Some(e) => now < e.window_start + window_duration,
            None => false,
        };
        let is_limited =
            within_window && (max_requests == 0 || request_count >= max_requests);
        RateLimitResponse {
            request_count,
            window_start,
            window_duration_seconds: window_duration,
            max_requests_per_window_per_user: max_requests,
            is_limited,
        }
    })
}

/// Return the `VIDEOGEN` config constant (for unregistered-user quota
/// display on the mobile). Mirrors the canister's
/// `get_property_rate_limit_config("VIDEOGEN")`.
#[spacetimedb::procedure]
pub fn get_videogen_config(ctx: &mut ProcedureContext) -> VideogenConfigResponse {
    let _ = ctx; // no DB read needed; constants only.
    VideogenConfigResponse {
        property: VIDEOGEN_PROPERTY.to_string(),
        max_requests_per_window_registered: VIDEOGEN_MAX_REQUESTS_PER_WINDOW_REGISTERED,
        max_requests_per_window_unregistered: VIDEOGEN_MAX_REQUESTS_PER_WINDOW_UNREGISTERED,
        window_duration_seconds: VIDEOGEN_WINDOW_DURATION_SECONDS,
        max_requests_per_property_all_users: VIDEOGEN_MAX_REQUESTS_PER_PROPERTY_ALL_USERS,
        property_rate_limit_window_duration_seconds: VIDEOGEN_PROPERTY_WINDOW_DURATION_SECONDS,
    }
}
