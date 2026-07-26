//! Write-path reducers (admin-gated).
//!
//! Only the latest v2/v1 variants are implemented; the v0
//! `increment_request_count` / `check_rate_limit` /
//! `create_video_generation_request` (v0/v1) are dropped (no production
//! caller). All reducers return `Result<(), String>`.

use spacetimedb::{ReducerContext, Table};

use crate::consts::ADMIN_IDENTITY;
use crate::helpers::*;
use crate::tables::{UserRequestCounter, VideoGenRequest};
use crate::types::{TokenType, VideoGenRequestStatus};

fn is_admin(ctx: &ReducerContext) -> bool {
    ctx.sender() == ADMIN_IDENTITY
}

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