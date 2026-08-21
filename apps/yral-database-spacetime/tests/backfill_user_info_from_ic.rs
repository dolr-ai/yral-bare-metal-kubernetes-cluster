//! One-time IC→SpacetimeDB backfill test for user_info (profiles).
//!
//! Run with: `cargo test --package yral_database_spacetime backfill_user_info_from_ic -- --ignored --nocapture`
//!
//! This test is `#[ignore]`d so it doesn't run during normal `cargo test`.
//!
//! ## Strategy
//!
//! **Streaming + resumable.** Streams IC `fetch_posts` in batches of 1000
//! (same cursor pagination as the posts backfill). For each batch of posts,
//! extracts unique `creator_principal` values not yet seen, fetches their
//! profiles from IC `user_info_service` in batches of 100, and upserts to
//! SpacetimeDB via REST immediately. Nothing is buffered beyond a single
//! post batch + a single profile batch in memory.
//!
//! **Resumable:** The posts cursor (`last_post_id_fetched`) is printed every
//! batch. If interrupted, restart from a known cursor by setting
//! `BACKFILL_START_CURSOR` env var to the last printed cursor value.
//! Already-upserted profiles are idempotent (delete-then-insert by PK).
//!
//! **Idempotent:** safe to run multiple times. Re-running with the same
//! principal updates the row instead of duplicating.
//!
//! **Username/email:** Set to `None` — the metadata backfill
//! fills those from the yral-metadata service.
//!
//! **Delete this file once the mobile update has shipped and the IC canister
//! is decommissioned (Phase J cleanup).**

#![cfg(test)]

use std::collections::BTreeSet;

use anyhow::Context;
use candid::Principal;
use canisters_client::user_post_service::{FetchPostsArgs, UserPostService};
use canisters_client::user_info_service::{
    Result9, UserProfileDetailsForFrontendV7, UserInfoService,
};
use ic_agent::Agent;

/// IC canister ID for `user_post_service` on mainnet.
const IC_POSTS_CANISTER_ID: &str = "gxhc3-pqaaa-aaaas-qbh3q-cai";

/// IC canister ID for `user_info_service` on mainnet.
const IC_USER_INFO_CANISTER_ID: &str = "ivkka-7qaaa-aaaas-qbg3q-cai";

/// IC boundary node URL.
const IC_URL: &str = "https://ic0.app";

/// Batch size for IC `fetch_posts` pagination.
const IC_POSTS_BATCH_SIZE: u64 = 1000;

/// Batch size for IC `get_users_profile_details` (max 100 per call).
const IC_PROFILE_BATCH_SIZE: usize = 100;

/// Build an anonymous IC agent (no identity needed — all calls are queries).
async fn build_ic_agent() -> anyhow::Result<Agent> {
    let agent = Agent::builder().with_url(IC_URL).build()?;
    agent.fetch_root_key().await?;
    Ok(agent)
}

/// Escape a string for JSON.
fn json_escape(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| format!("\"{}\"", s.replace('"', "\\\"")))
}

/// Build the JSON for a single `UserProfileBatchEntry` (SpacetimeDB REST format).
///
/// SpacetimeDB serializes:
/// - Strings as `"text"`
/// - Integers as bare numbers
/// - Bools as `true`/`false`
/// - Enums as `{"variantName":{fields}}` or `{"variantName":value}`
/// - `Option<T>` as `[0, value]` (Some) or `[1, []]` (None)
/// - `Timestamp` as `[microseconds]`
fn build_entry_json(principal: &Principal, profile: &UserProfileDetailsForFrontendV7) -> String {
    let pic = profile.profile_picture.as_ref();

    let principal_text = principal.to_text();
    let bio = profile.bio.clone().unwrap_or_default();
    let website_url = profile.website_url.clone().unwrap_or_default();
    let profile_picture_url = pic.map(|p| p.url.clone()).unwrap_or_default();
    let followers_count = profile.followers_count;
    let following_count = profile.following_count;
    let is_ai_influencer = profile.is_ai_influencer;
    let is_nsfw = pic.map(|p| p.nsfw_info.is_nsfw).unwrap_or(false);
    let nsfw_ec = pic.map(|p| p.nsfw_info.nsfw_ec.clone()).unwrap_or_default();
    let nsfw_gore = pic.map(|p| p.nsfw_info.nsfw_gore.clone()).unwrap_or_default();
    let csam_detected = pic.map(|p| p.nsfw_info.csam_detected).unwrap_or(false);

    // SubscriptionPlan: {"free":{}} or {"pro":{"free_video_credits_left":N,"total_video_credits_alloted":N}}
    let subscription_plan_json = match &profile.subscription_plan {
        canisters_client::user_info_service::SubscriptionPlan::Free => {
            r#"{"free":{}}"#.to_string()
        }
        canisters_client::user_info_service::SubscriptionPlan::Pro(sub) => {
            format!(
                r#"{{"pro":{{"free_video_credits_left":{},"total_video_credits_alloted":{}}}}}"#,
                sub.free_video_credits_left, sub.total_video_credits_alloted
            )
        }
    };

    // last_access_time: Timestamp as [microseconds] — use 0 (UNIX_EPOCH)
    let last_access_time_json = "[0]";

    // username: Option<String> → [1, []] for None
    let username_json = "[1,[]]";
    // email: Option<String> → [1, []] for None
    let email_json = "[1,[]]";

    format!(
        r#"{{"principal_text":{},"bio":{},"website_url":{},"profile_picture_url":{},"followers_count":{},"following_count":{},"subscription_plan":{},"is_ai_influencer":{},"is_nsfw":{},"nsfw_ec":{},"nsfw_gore":{},"csam_detected":{},"last_access_time":{},"username":{},"email":{}}}"#,
        json_escape(&principal_text),
        json_escape(&bio),
        json_escape(&website_url),
        json_escape(&profile_picture_url),
        followers_count,
        following_count,
        subscription_plan_json,
        is_ai_influencer,
        is_nsfw,
        json_escape(&nsfw_ec),
        json_escape(&nsfw_gore),
        csam_detected,
        last_access_time_json,
        username_json,
        email_json,
    )
}

/// Build the JSON argument for `upsert_user_profile_batch(profiles: Vec<UserProfileBatchEntry>)`.
/// The REST API wraps reducer args in an outer array: `[[<Vec<UserProfileBatchEntry> as JSON array>]]`.
fn build_batch_json(entries: &[String]) -> String {
    format!("[[{}]]", entries.join(","))
}

/// Upsert a batch of profiles to SpacetimeDB via a single `upsert_user_profile_batch` REST call.
/// Retries up to 3 times with exponential backoff on transient errors.
/// Returns (success_count, error_count).
async fn upsert_profile_batch(
    http: &reqwest::Client,
    call_url: &str,
    token: &str,
    entries_json: &[String],
) -> (usize, usize) {
    let body = build_batch_json(entries_json);

    for attempt in 0..3u32 {
        let resp = match http
            .post(call_url)
            .bearer_auth(token)
            .header("Content-Type", "application/json")
            .body(body.clone())
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                if attempt < 2 {
                    eprintln!("  retry {}/3 profile batch (conn error: {})", attempt + 1, e);
                    tokio::time::sleep(std::time::Duration::from_secs(1 << attempt)).await;
                    continue;
                }
                eprintln!("  FAILED profile batch after 3 retries: {}", e);
                return (0, entries_json.len());
            }
        };

        let status = resp.status();
        if status.is_success() {
            return (entries_json.len(), 0);
        }

        if (status.is_server_error() || status.as_u16() == 429) && attempt < 2 {
            let resp_body = resp.text().await.unwrap_or_default();
            eprintln!("  retry {}/3 profile batch ({} {})", attempt + 1, status, resp_body);
            tokio::time::sleep(std::time::Duration::from_secs(1 << attempt)).await;
            continue;
        }

        let resp_body = resp.text().await.unwrap_or_default();
        eprintln!("  FAILED profile batch: {} {}", status, resp_body);
        return (0, entries_json.len());
    }

    (0, entries_json.len())
}

/// Fetch profiles from IC and upsert to SpacetimeDB. Returns (fetched, upserted, errors).
async fn fetch_and_upsert_profiles(
    agent: &Agent,
    http: &reqwest::Client,
    call_url: &str,
    token: &str,
    principals: &[Principal],
) -> (usize, usize, usize) {
    let user_info_canister_id = Principal::from_text(IC_USER_INFO_CANISTER_ID)
        .expect("invalid user_info canister ID");
    let user_info_service = UserInfoService(user_info_canister_id, agent);

    let mut total_fetched = 0usize;
    let mut total_upserted = 0usize;
    let mut total_errors = 0usize;

    for chunk in principals.chunks(IC_PROFILE_BATCH_SIZE) {
        let result = match user_info_service
            .get_users_profile_details(chunk.to_vec())
            .await
        {
            Ok(r) => r,
            Err(e) => {
                eprintln!("  IC profile fetch error: {e}");
                total_errors += chunk.len();
                continue;
            }
        };

        let profiles: Vec<(Principal, UserProfileDetailsForFrontendV7)> = match result {
            Result9::Ok(profiles) => profiles
                .into_iter()
                .filter_map(|p| {
                    if chunk.contains(&p.principal_id) {
                        Some((p.principal_id, p))
                    } else {
                        None
                    }
                })
                .collect(),
            Result9::Err(e) => {
                eprintln!("  IC profile error: {e}");
                total_errors += chunk.len();
                continue;
            }
        };

        total_fetched += profiles.len();
        let skipped = chunk.len() - profiles.len();

        if profiles.is_empty() {
            continue;
        }

        // Build JSON entries
        let entries_json: Vec<String> = profiles
            .iter()
            .map(|(p, v)| build_entry_json(p, v))
            .collect();

        let (ok, err) = upsert_profile_batch(http, call_url, token, &entries_json).await;
        total_upserted += ok;
        total_errors += err;

        if skipped > 0 {
            eprintln!("    profiles: {} fetched, {} upserted, {} skipped (not in IC)", profiles.len(), ok, skipped);
        }
    }

    (total_fetched, total_upserted, total_errors)
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn backfill_user_info_from_ic() -> anyhow::Result<()> {
    eprintln!("=== IC → SpacetimeDB User Info Backfill (streaming, resumable) ===");

    // --- IC agent ---
    eprintln!("Connecting to IC...");
    let agent = build_ic_agent().await?;
    let posts_canister_id = Principal::from_text(IC_POSTS_CANISTER_ID)?;
    let post_service = UserPostService(posts_canister_id, &agent);

    // --- SpacetimeDB REST config ---
    let db_name = std::env::var("SPACETIMEDB_DB_NAME")
        .unwrap_or_else(|_| "yral-database-spacetime-4lbo7".to_string());
    let uri = std::env::var("SPACETIMEDB_URL")
        .unwrap_or_else(|_| "https://maincloud.spacetimedb.com".to_string());
    let token = std::env::var("SPACETIMEDB_ADMIN_TOKEN").context("SPACETIMEDB_ADMIN_TOKEN")?;

    let http = reqwest::Client::new();
    let call_url = format!(
        "{}/v1/database/{}/call/upsert_user_profile_batch",
        uri.trim_end_matches('/'),
        db_name,
    );
    eprintln!("SpacetimeDB REST: {}", call_url);

    // --- Resumable: start from a known cursor if provided ---
    let mut last_uuid: Option<String> = std::env::var("BACKFILL_START_CURSOR").ok().filter(|s| !s.is_empty());

    // Track principals already processed (avoids re-fetching profiles on resume)
    let mut seen_principals: BTreeSet<String> = BTreeSet::new();

    let mut total_posts_read: u64 = 0;
    let mut total_profiles_fetched: u64 = 0;
    let mut total_profiles_upserted: u64 = 0;
    let mut total_errors: u64 = 0;
    let mut batch_num: u64 = 0;

    eprintln!("=== Streaming IC posts → collecting principals → fetching profiles → upserting ===");

    loop {
        let result = post_service
            .fetch_posts(FetchPostsArgs {
                limit: IC_POSTS_BATCH_SIZE,
                last_uuid_processed: last_uuid.clone(),
            })
            .await?;

        if result.posts.is_empty() {
            break;
        }

        let batch_len = result.posts.len();
        last_uuid = result.last_post_id_fetched.clone();
        batch_num += 1;
        total_posts_read += batch_len as u64;

        // Extract new unique principals from this batch
        let new_principals: Vec<Principal> = result
            .posts
            .iter()
            .map(|p| p.creator_principal.to_text())
            .filter(|p| seen_principals.insert(p.clone()))
            .filter_map(|p| Principal::from_text(&p).ok())
            .collect();

        let new_count = new_principals.len();

        if new_count > 0 {
            // Fetch profiles from IC and upsert to SpacetimeDB immediately
            let (fetched, upserted, errors) =
                fetch_and_upsert_profiles(&agent, &http, &call_url, &token, &new_principals)
                    .await;

            total_profiles_fetched += fetched as u64;
            total_profiles_upserted += upserted as u64;
            total_errors += errors as u64;
        }

        eprintln!(
            "  Batch {batch_num}: {batch_len} posts, {new_count} new principals | Cumulative: {total_posts_read} posts, {total_seen} principals, {total_profiles_fetched} fetched, {total_profiles_upserted} upserted, {total_errors} errors, cursor={cursor:?}",
            total_seen = seen_principals.len(),
            cursor = last_uuid,
        );

        if last_uuid.is_none() {
            break;
        }
    }

    eprintln!();
    eprintln!("=== User Info Backfill Complete ===");
    eprintln!("  Total posts streamed:          {total_posts_read}");
    eprintln!("  Unique principals found:       {}", seen_principals.len());
    eprintln!("  Profiles fetched from IC:      {total_profiles_fetched}");
    eprintln!("  Profiles upserted:             {total_profiles_upserted}");
    eprintln!("  Errors:                        {total_errors}");

    if total_errors > 0 {
        anyhow::bail!("{total_errors} errors (see above)");
    }

    Ok(())
}