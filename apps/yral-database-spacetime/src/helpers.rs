//! Internal helpers: timestamp, admin check, blacklist, config-tier
//! resolution, counter increment/decrement, and rate-limit check.
//!
//! These mirror the canister's `CanisterData` methods
//! (`increment_request_with_property`, `decrement_request_with_property`,
//! `is_rate_limited_with_property`, `is_property_daily_rate_limited`, etc.)

use spacetimedb::{ReducerContext, Table};

use crate::consts::*;
use crate::tables::{PropertyRateLimit, RateLimit};

/// SpacetimeDB `Timestamp` → Unix seconds (matches the canister's
/// `ic_cdk::api::time() / 1_000_000_000` seconds granularity).
pub(crate) fn now_secs(ctx: &ReducerContext) -> u64 {
    (ctx.timestamp.to_micros_since_unix_epoch() / 1_000_000) as u64
}

pub(crate) fn now_secs_proc(ctx: &spacetimedb::ProcedureContext) -> u64 {
    (ctx.timestamp.to_micros_since_unix_epoch() / 1_000_000) as u64
}

pub(crate) fn is_admin(ctx: &ReducerContext) -> bool {
    ctx.sender() == ADMIN_IDENTITY
}

pub(crate) fn is_blacklisted(property: &str) -> bool {
    BLACKLIST.iter().any(|b| *b == property || *b == "all")
}

/// Resolve the per-principal `(max_requests, window_duration)` by the
/// canister's tier precedence: entry override → property constant → default
/// constant. `is_registered` selects the registered vs unregistered branch
/// at the property/default tiers (the entry override is registration-agnostic,
/// matching the canister's `entry.config`).
pub(crate) fn resolve_user_limit(
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
pub(crate) fn property_aggregate_limit(property: &str) -> Option<(u64, u64)> {
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
pub(crate) fn is_property_daily_rate_limited(ctx: &ReducerContext, property: &str) -> bool {
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
pub(crate) fn increment_request(ctx: &ReducerContext, principal: &str, property: &str) {
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
pub(crate) fn increment_property_counter(ctx: &ReducerContext, property: &str, now: u64) {
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
pub(crate) fn decrement_request(ctx: &ReducerContext, principal: &str, property: &str) {
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
pub(crate) fn decrement_property_counter(ctx: &ReducerContext, property: &str, now: u64) {
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
pub(crate) fn is_rate_limited(
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