//! One-time IC→SpacetimeDB backfill test.
//!
//! Run with: `cargo test --package yral_database_spacetime backfill_from_ic -- --ignored --nocapture`
//!
//! This test is `#[ignore]`d so it doesn't run during normal `cargo test`.
//! It connects to the live IC canister (`user_post_service`, mainnet
//! `gxhc3-pqaaa-aaaas-qbh3q-cai`) and the target SpacetimeDB instance,
//! reads posts via cursor-paginated `fetch_posts`, and upserts each batch
//! into SpacetimeDB via the `upsert_post` REST API.
//!
//! **Streaming / batched:** Reads IC posts in batches of 1000 and immediately
//! upserts each batch to SpacetimeDB before fetching the next. Nothing is
//! buffered in memory beyond a single batch.
//!
//! **Idempotent:** safe to run multiple times. Re-running with the same
//! post ID updates the row instead of duplicating (the reducer deletes-then-inserts
//! by primary key). This enables the two-run migration strategy:
//! 1. Run once at staging merge → seed data.
//! 2. Run again at app-store rollout → picks up delta posts + updated view counts.
//!
//! **Principal → Identity mapping:** Uses `Identity::from_claims(issuer, &principal.to_text())`
//! to match yral-auth's JWT-based identity derivation.
//!
//! **Delete this file once the mobile update has shipped and the IC canister
//! is decommissioned (Phase J cleanup).**

#![cfg(test)]

use anyhow::Context;
use canisters_client::user_post_service::{
    FetchPostsArgs, Post as IcPost, PostStatus as IcPostStatus,
    UserPostService,
};
use candid::Principal;
use ic_agent::Agent;
use spacetimedb_sdk::Identity;

/// IC canister ID for `user_post_service` on mainnet.
const IC_CANISTER_ID: &str = "gxhc3-pqaaa-aaaas-qbh3q-cai";

/// IC boundary node URL.
const IC_URL: &str = "https://ic0.app";

/// Batch size for IC `fetch_posts` pagination.
const IC_BATCH_SIZE: u64 = 1000;

/// Map an IC `Principal` to a SpacetimeDB `Identity`.
///
/// Uses `Identity::from_claims(issuer, &principal.to_text())` to match
/// yral-auth's JWT-based identity derivation. The issuer URL comes from
/// the `YRAL_AUTH_ISSUER` env var (defaults to `https://auth.yral.com`).
fn principal_to_identity(principal: &Principal) -> Identity {
    let issuer = std::env::var("YRAL_AUTH_ISSUER")
        .unwrap_or_else(|_| "https://auth.yral.com".to_string());
    Identity::from_claims(&issuer, &principal.to_text())
}

/// Map an IC `PostStatus` to the SpacetimeDB status enum JSON string.
fn status_to_json(ic_status: &IcPostStatus) -> String {
    match ic_status {
        IcPostStatus::Uploaded => r#"{"uploaded":{}}"#,
        IcPostStatus::Transcoding => r#"{"transcoding":{}}"#,
        IcPostStatus::CheckingExplicitness => r#"{"checkingExplicitness":{}}"#,
        IcPostStatus::BannedForExplicitness => r#"{"bannedForExplicitness":{}}"#,
        IcPostStatus::ReadyToView => r#"{"readyToView":{}}"#,
        IcPostStatus::BannedDueToUserReporting => r#"{"bannedDueToUserReporting":{}}"#,
        IcPostStatus::Deleted => r#"{"deleted":{}}"#,
        IcPostStatus::Draft => r#"{"draft":{}}"#,
    }
    .to_string()
}

/// Escape a string for JSON.
fn json_escape(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| format!("\"{}\"", s.replace('"', "\\\"")))
}

/// Build the JSON argument for a single `upsert_post` REST call.
///
/// The SpacetimeDB REST API expects reducer arguments as a JSON array where
/// each element is the reducer's argument. For `upsert_post(post: Post)`,
/// the single argument is the Post struct with these special encodings:
/// - `creator`: `["0x<32-byte-hex>"]` — 1-element tuple wrapping a U256 hex string
/// - `created_at`: `[<micros>]` — 1-element tuple wrapping an i64 timestamp
/// - `status`: `{"variantName":{}}` — enum as object with unit variant
fn build_upsert_json(post: &IcPost) -> String {
    let identity = principal_to_identity(&post.creator_principal);
    let creator_hex = format!("0x{}", identity.to_hex());

    let micros = (post.created_at.secs_since_epoch as i64) * 1_000_000
        + (post.created_at.nanos_since_epoch as i64) / 1_000;

    let hashtags: Vec<String> = post.hashtags.iter().map(|h| json_escape(h)).collect();

    // Build JSON manually to avoid format! escaping issues with {{ }}.
    let mut s = String::with_capacity(512);
    s.push('['); // outer array (reducer args)
    s.push('{'); // Post struct
    s.push_str("\"id\":");
    s.push_str(&json_escape(&post.id));
    s.push_str(",\"video_uid\":");
    s.push_str(&json_escape(&post.video_uid));
    s.push_str(",\"description\":");
    s.push_str(&json_escape(&post.description));
    s.push_str(",\"hashtags\":[");
    s.push_str(&hashtags.join(","));
    s.push(']');
    s.push_str(",\"creator\":[\"");
    s.push_str(&creator_hex);
    s.push_str("\"]");
    s.push_str(",\"status\":");
    s.push_str(&status_to_json(&post.status));
    s.push_str(",\"created_at\":[");
    s.push_str(&micros.to_string());
    s.push(']');
    s.push_str(",\"share_count\":");
    s.push_str(&post.share_count.to_string());
    s.push_str(",\"view_total_count\":");
    s.push_str(&post.view_stats.total_view_count.to_string());
    s.push_str(",\"view_threshold_count\":");
    s.push_str(&post.view_stats.threshold_view_count.to_string());
    s.push_str(",\"view_average_watch_percentage\":");
    s.push_str(&post.view_stats.average_watch_percentage.to_string());
    s.push('}'); // close Post
    s.push(']'); // close outer array
    s
}

/// Build an anonymous IC agent (no identity needed — `fetch_posts` is a query).
async fn build_ic_agent() -> anyhow::Result<Agent> {
    let agent = Agent::builder()
        .with_url(IC_URL)
        .build()?;
    agent.fetch_root_key().await?;
    Ok(agent)
}

/// Upsert a single post to SpacetimeDB via REST API, with retry on transient errors.
///
/// Retries up to 3 times with exponential backoff (1s, 2s, 4s) on connection
/// errors or 5xx responses. Returns `true` on success, `false` on permanent failure.
async fn upsert_one_with_retry(
    http: &reqwest::Client,
    call_url: &str,
    token: &str,
    post: &IcPost,
) -> bool {
    let body = build_upsert_json(post);

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
                    eprintln!("  retry {}/3 post {} (conn error: {})", attempt + 1, post.id, e);
                    tokio::time::sleep(std::time::Duration::from_secs(1 << attempt)).await;
                    continue;
                }
                eprintln!("  FAILED post {} after 3 retries: {}", post.id, e);
                return false;
            }
        };

        let status = resp.status();
        if status.is_success() {
            return true;
        }

        // Retry on 5xx (server error) or 429 (rate limit); fail on 4xx (client error)
        if (status.is_server_error() || status.as_u16() == 429) && attempt < 2 {
            let body = resp.text().await.unwrap_or_default();
            eprintln!("  retry {}/3 post {} ({} {})", attempt + 1, post.id, status, body);
            tokio::time::sleep(std::time::Duration::from_secs(1 << attempt)).await;
            continue;
        }

        let body = resp.text().await.unwrap_or_default();
        eprintln!("  FAILED post {}: {} {}", post.id, status, body);
        return false;
    }

    false
}

/// Upsert a batch of posts to SpacetimeDB via REST API.
async fn upsert_batch(
    http: &reqwest::Client,
    call_url: &str,
    token: &str,
    posts: &[IcPost],
) -> anyhow::Result<(usize, usize)> {
    let mut ok = 0usize;
    let mut err = 0usize;

    for post in posts {
        if upsert_one_with_retry(http, call_url, token, post).await {
            ok += 1;
        } else {
            err += 1;
        }
    }

    Ok((ok, err))
}

#[tokio::test]
#[ignore]
async fn backfill_from_ic() -> anyhow::Result<()> {
    eprintln!("=== IC → SpacetimeDB Backfill (streaming) ===");

    // --- IC agent ---
    eprintln!("Connecting to IC canister {}...", IC_CANISTER_ID);
    let agent = build_ic_agent().await?;
    let canister_id = Principal::from_text(IC_CANISTER_ID)?;;
    let post_service = UserPostService(canister_id, &agent);

    // --- SpacetimeDB REST config ---
    let db_name = std::env::var("SPACETIMEDB_DB_NAME")
        .unwrap_or_else(|_| "yral-database-spacetime-4lbo7".to_string());
    let uri = std::env::var("SPACETIMEDB_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
    let token = std::env::var("SPACETIMEDB_ADMIN_TOKEN")
        .context("SPACETIMEDB_ADMIN_TOKEN")?;

    let http = reqwest::Client::new();
    let call_url = format!(
        "{}/v1/database/{}/call/upsert_post",
        uri.trim_end_matches('/'),
        db_name,
    );
    eprintln!("SpacetimeDB REST: {}", call_url);

    // --- Stream: read IC batch → upsert to SpacetimeDB → next batch ---
    let mut last_uuid: Option<String> = None;
    let mut total_read: u64 = 0;
    let mut total_upserted: u64 = 0;
    let mut total_errors: u64 = 0;
    let mut batch_num: u64 = 0;

    loop {
        let result = post_service
            .fetch_posts(FetchPostsArgs {
                limit: IC_BATCH_SIZE,
                last_uuid_processed: last_uuid.clone(),
            })
            .await?;

        if result.posts.is_empty() {
            break;
        }

        let batch_len = result.posts.len();
        last_uuid = result.last_post_id_fetched.clone();
        batch_num += 1;
        total_read += batch_len as u64;

        // Upsert this batch immediately.
        let (ok, err) = upsert_batch(&http, &call_url, &token, &result.posts).await?;
        total_upserted += ok as u64;
        total_errors += err as u64;

        eprintln!(
            "  Batch {batch_num}: {batch_len} posts read, {ok} upserted, {err} errors | Cumulative: {total_read} read, {total_upserted} upserted, {total_errors} errors, cursor={cursor:?}",
            cursor = last_uuid,
        );

        if last_uuid.is_none() {
            break;
        }
    }

    eprintln!();
    eprintln!("=== Backfill Complete ===");
    eprintln!("  Total posts read from IC:    {total_read}");
    eprintln!("  Successfully upserted:       {total_upserted}");
    eprintln!("  Errors:                      {total_errors}");

    if total_errors > 0 {
        anyhow::bail!("{total_errors} upsert errors (see above)");
    }

    // Spot-check: read back a few posts from SpacetimeDB.
    if total_upserted == 0 {
        eprintln!("No posts to verify.");
        return Ok(());
    }

    eprintln!();
    eprintln!("Spot-check: verifying 3 posts via get_post_by_id...");

    // Re-fetch the first 3 posts from IC to get their IDs, then verify in SpacetimeDB.
    let verify_result = post_service
        .fetch_posts(FetchPostsArgs {
            limit: 3,
            last_uuid_processed: None,
        })
        .await?;

    let get_url = format!(
        "{}/v1/database/{}/call/get_post_by_id",
        uri.trim_end_matches('/'),
        db_name,
    );

    for (i, post) in verify_result.posts.iter().enumerate() {
        let body = format!(r#"["{}"]"#, post.id);
        let resp = http
            .post(&get_url)
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await;

        match resp {
            Ok(r) if r.status().is_success() => {
                let text = r.text().await.unwrap_or_default();
                if text.contains(&post.id) {
                    eprintln!("  ✓ post[{i}] id={} found in SpacetimeDB", post.id);
                } else {
                    eprintln!("  ✗ post[{i}] id={} — ID not found in response: {}", post.id, &text[..text.len().min(100)]);
                }
            }
            Ok(r) => {
                eprintln!("  ✗ post[{i}] id={} — HTTP {}", post.id, r.status());
            }
            Err(e) => {
                eprintln!("  ✗ post[{i}] id={} — request error: {}", post.id, e);
            }
        }
    }

    Ok(())
}