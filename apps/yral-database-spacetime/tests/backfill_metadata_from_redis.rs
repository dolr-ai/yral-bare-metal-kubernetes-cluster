//! One-time Redis/Dragonfly → SpacetimeDB backfill test for usernames & emails.
//!
//! Run with: `cargo test --package yral_database_spacetime backfill_metadata_from_redis -- --ignored --nocapture`
//!
//! This test is `#[ignore]`d so it doesn't run during normal `cargo test`.
//!
//! ## Strategy
//!
//! **Streaming + resumable.** Streams IC `fetch_posts` in batches of 1000
//! (same cursor pagination as the posts backfill). For each batch of posts,
//! extracts unique `creator_principal` values not yet seen, fetches their
//! metadata (username, email) from the yral-metadata service
//! (`POST /metadata-bulk`) in batches, then fetches the existing SpacetimeDB
//! profile via REST, fills in `username`/`email`, and re-upserts via REST
//! immediately.
//!
//! **Resumable:** The posts cursor is printed every batch. If interrupted,
//! restart from a known cursor by setting `BACKFILL_START_CURSOR` env var.
//! Already-upserted profiles are idempotent (delete-then-insert by PK).
//!
//! **Prerequisite:** Run `backfill_user_info_from_ic` first to populate the
//! profile data (bio, profile picture, follower counts, etc.) from the IC
//! canister. This script only enriches those profiles with username/email
//! from the metadata service (Redis/Dragonfly).
//!
//! **Delete this file once the Redis/Dragonfly data is fully migrated and
//! the yral-metadata service is decommissioned.**

#![cfg(test)]

use std::collections::BTreeSet;

use anyhow::Context;
use candid::Principal;
use canisters_client::user_post_service::{FetchPostsArgs, UserPostService};
use ic_agent::Agent;
use serde::{Deserialize, Serialize};

/// IC canister ID for `user_post_service` on mainnet.
const IC_POSTS_CANISTER_ID: &str = "gxhc3-pqaaa-aaaas-qbh3q-cai";

/// IC boundary node URL.
const IC_URL: &str = "https://ic0.app";

/// Batch size for IC `fetch_posts` pagination.
const IC_POSTS_BATCH_SIZE: u64 = 1000;

/// Batch size for metadata bulk API calls.
const METADATA_BATCH_SIZE: usize = 200;

/// Default yral-metadata service URL.
const METADATA_BASE_URL: &str = "https://metadata.yral.com";

/// Build an anonymous IC agent.
async fn build_ic_agent() -> anyhow::Result<Agent> {
    let agent = Agent::builder().with_url(IC_URL).build()?;
    agent.fetch_root_key().await?;
    Ok(agent)
}

// ─────────────────────────────────────────────────────────────────────────
// Metadata service API types
// ─────────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct BulkGetUserMetadataRequest {
    users: Vec<String>,
}

#[derive(Deserialize)]
struct BulkMetadataResponse {
    #[serde(rename = "Ok")]
    ok: Option<std::collections::HashMap<String, Option<UserMetadataDto>>>,
    #[serde(rename = "Err")]
    err: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct UserMetadataDto {
    #[serde(rename = "user_name")]
    user_name: String,
    #[serde(default)]
    email: Option<String>,
}

/// Fetch usernames and emails from the yral-metadata service in bulk.
/// Returns a map of principal_text → (username, email).
async fn fetch_metadata_bulk(
    client: &reqwest::Client,
    base_url: &str,
    principals: &[String],
) -> anyhow::Result<std::collections::HashMap<String, (Option<String>, Option<String>)>> {
    let url = format!("{}/metadata-bulk", base_url.trim_end_matches('/'));

    let req = BulkGetUserMetadataRequest {
        users: principals.to_vec(),
    };

    let resp = client
        .post(&url)
        .json(&req)
        .send()
        .await
        .context("metadata-bulk request failed")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("metadata-bulk returned {status}: {body}");
    }

    let parsed: BulkMetadataResponse =
        resp.json().await.context("metadata-bulk parse failed")?;

    let data = parsed
        .ok
        .ok_or_else(|| anyhow::anyhow!("metadata-bulk error: {:?}", parsed.err))?;

    let result = data
        .into_iter()
        .filter_map(|(principal_text, metadata)| {
            let meta = metadata?;
            let username = if meta.user_name.is_empty() {
                None
            } else {
                Some(meta.user_name)
            };
            let email = meta.email.filter(|e| !e.is_empty());
            Some((principal_text, (username, email)))
        })
        .collect();

    Ok(result)
}

// ─────────────────────────────────────────────────────────────────────────
// SpacetimeDB REST helpers
// ─────────────────────────────────────────────────────────────────────────

/// Escape a string for JSON.
fn json_escape(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| format!("\"{}\"", s.replace('"', "\\\"")))
}

/// Fetch an existing profile from SpacetimeDB via REST `get_user_profile_details_v7`.
/// Returns the raw JSON of the profile if found, or `None` if not found.
async fn fetch_existing_profile_json(
    http_client: &reqwest::Client,
    spacetime_url: &str,
    spacetime_db: &str,
    spacetime_token: &str,
    principal_text: &str,
) -> anyhow::Result<Option<serde_json::Value>> {
    let url = format!(
        "{}/v1/database/{}/call/get_user_profile_details_v_7",
        spacetime_url.trim_end_matches('/'),
        spacetime_db,
    );

    let resp = http_client
        .post(&url)
        .bearer_auth(spacetime_token)
        .header("Content-Type", "application/json")
        .body(format!(r#"["{}"]"#, principal_text))
        .send()
        .await
        .context("SpacetimeDB get_user_profile_details_v_7 request failed")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("SpacetimeDB get_user_profile_details_v_7 returned {status}: {body}");
    }

    let body: serde_json::Value = resp.json().await?;

    // The REST API returns the Option variant directly: [0, {...}] for Some, [1, []] for None.
    let variant = body
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("unexpected response shape: no variant array"))?;

    if variant.len() < 2 {
        anyhow::bail!("unexpected variant shape: too few elements");
    }

    let tag = variant
        .first()
        .and_then(|v| v.as_u64())
        .ok_or_else(|| anyhow::anyhow!("unexpected variant tag"))?;

    if tag == 1 {
        return Ok(None); // None — profile doesn't exist
    }

    // Tag 0 — payload is the UserProfileDetailsV7 object
    let profile_json = variant
        .get(1)
        .ok_or_else(|| anyhow::anyhow!("missing profile payload"))?;

    Ok(Some(profile_json.clone()))
}

/// Build the JSON for a `UserProfileBatchEntry` from an existing profile JSON
/// with username and email filled in.
///
/// The existing profile JSON from `get_user_profile_details_v_7` has fields
/// like `principal_id`, `bio`, `website_url`, `profile_picture`, etc.
/// We need to transform it into `UserProfileBatchEntry` format and inject
/// `username` and `email`.
fn build_enriched_entry_json(
    profile_json: &serde_json::Value,
    username: &Option<String>,
    email: &Option<String>,
) -> String {
    let get_str = |field: &str| -> String {
        profile_json
            .get(field)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };

    let get_u64 = |field: &str| -> u64 {
        profile_json
            .get(field)
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
    };

    let get_bool = |field: &str| -> bool {
        profile_json
            .get(field)
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    };

    let principal_text = profile_json
        .get("principal_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let bio = get_str("bio");
    let website_url = get_str("website_url");
    let followers_count = get_u64("followers_count");
    let following_count = get_u64("following_count");
    let is_ai_influencer = get_bool("is_ai_influencer");

    let pic = profile_json.get("profile_picture");
    let profile_picture_url = pic
        .and_then(|p| p.get("url"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let nsfw_info = pic.and_then(|p| p.get("nsfw_info"));
    let is_nsfw = nsfw_info
        .and_then(|n| n.get("is_nsfw"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let nsfw_ec = nsfw_info
        .and_then(|n| n.get("nsfw_ec"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let nsfw_gore = nsfw_info
        .and_then(|n| n.get("nsfw_gore"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let csam_detected = nsfw_info
        .and_then(|n| n.get("csam_detected"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // subscription_plan: {"free":{}} or {"pro":{...}}
    let subscription_plan_json = match profile_json.get("subscription_plan") {
        Some(sp) if sp.get("Free").is_some() => r#"{"free":{}}"#.to_string(),
        Some(sp) if sp.get("free").is_some() => r#"{"free":{}}"#.to_string(),
        Some(sp) if sp.get("Pro").is_some() => {
            let pro = sp.get("Pro").or_else(|| sp.get("pro")).unwrap();
            let credits_left = pro
                .get("free_video_credits_left")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let credits_alloted = pro
                .get("total_video_credits_alloted")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            format!(
                r#"{{"pro":{{"free_video_credits_left":{},"total_video_credits_alloted":{}}}}}"#,
                credits_left, credits_alloted
            )
        }
        _ => r#"{"free":{}}"#.to_string(),
    };

    // username: Option<String> → [0, "value"] for Some, [1, []] for None
    let username_json = match username {
        Some(u) => format!("[0,{}]", json_escape(u)),
        None => "[1,[]]".to_string(),
    };

    // email: Option<String>
    let email_json = match email {
        Some(e) => format!("[0,{}]", json_escape(e)),
        None => "[1,[]]".to_string(),
    };

    format!(
        r#"{{"principal_text":{},"bio":{},"website_url":{},"profile_picture_url":{},"followers_count":{},"following_count":{},"subscription_plan":{},"is_ai_influencer":{},"is_nsfw":{},"nsfw_ec":{},"nsfw_gore":{},"csam_detected":{},"last_access_time":[0],"username":{},"email":{}}}"#,
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
        username_json,
        email_json,
    )
}

/// Upsert a batch of enriched profiles to SpacetimeDB via REST.
async fn upsert_profile_batch(
    http: &reqwest::Client,
    call_url: &str,
    token: &str,
    entries_json: &[String],
) -> (usize, usize) {
    if entries_json.is_empty() {
        return (0, 0);
    }

    let body = format!("[[{}]]", entries_json.join(","));

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
                    tokio::time::sleep(std::time::Duration::from_secs(1 << attempt)).await;
                    continue;
                }
                eprintln!("  FAILED metadata profile batch after 3 retries: {}", e);
                return (0, entries_json.len());
            }
        };

        let status = resp.status();
        if status.is_success() {
            return (entries_json.len(), 0);
        }

        if (status.is_server_error() || status.as_u16() == 429) && attempt < 2 {
            let resp_body = resp.text().await.unwrap_or_default();
            eprintln!("  retry {}/3 metadata batch ({} {})", attempt + 1, status, resp_body);
            tokio::time::sleep(std::time::Duration::from_secs(1 << attempt)).await;
            continue;
        }

        let resp_body = resp.text().await.unwrap_or_default();
        eprintln!("  FAILED metadata batch: {} {}", status, resp_body);
        return (0, entries_json.len());
    }

    (0, entries_json.len())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn backfill_metadata_from_redis() -> anyhow::Result<()> {
    eprintln!("=== Redis/Dragonfly → SpacetimeDB Metadata Backfill (streaming, resumable) ===");

    let metadata_url = std::env::var("METADATA_BASE_URL")
        .unwrap_or_else(|_| METADATA_BASE_URL.to_string());
    eprintln!("Metadata service: {metadata_url}");

    // --- IC agent (for principal collection) ---
    eprintln!("Connecting to IC...");
    let agent = build_ic_agent().await?;
    let posts_canister_id = Principal::from_text(IC_POSTS_CANISTER_ID)?;
    let post_service = UserPostService(posts_canister_id, &agent);

    // --- SpacetimeDB REST config ---
    let db_name = std::env::var("SPACETIMEDB_DB_NAME")
        .unwrap_or_else(|_| "yral-database-spacetime-4lbo7".to_string());
    let spacetime_url = std::env::var("SPACETIMEDB_URL")
        .unwrap_or_else(|_| "https://maincloud.spacetimedb.com".to_string());
    let spacetime_token = std::env::var("SPACETIMEDB_ADMIN_TOKEN")
        .context("SPACETIMEDB_ADMIN_TOKEN")?;

    let http = reqwest::Client::new();
    let upsert_url = format!(
        "{}/v1/database/{}/call/upsert_user_profile_batch",
        spacetime_url.trim_end_matches('/'),
        db_name,
    );

    // --- Resumable: start from a known cursor if provided ---
    let mut last_uuid: Option<String> =
        std::env::var("BACKFILL_START_CURSOR").ok().filter(|s| !s.is_empty());

    let mut seen_principals: BTreeSet<String> = BTreeSet::new();

    let mut total_posts_read: u64 = 0;
    let mut total_with_username: u64 = 0;
    let mut total_with_email: u64 = 0;
    let mut total_upserted: u64 = 0;
    let mut total_skipped_no_profile: u64 = 0;
    let mut total_errors: u64 = 0;
    let mut batch_num: u64 = 0;

    eprintln!("=== Streaming IC posts → fetching metadata → enriching & upserting ===");

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
        let new_principals: Vec<String> = result
            .posts
            .iter()
            .map(|p| p.creator_principal.to_text())
            .filter(|p| seen_principals.insert(p.clone()))
            .collect();

        let new_count = new_principals.len();

        if new_count > 0 {
            // Fetch metadata from yral-metadata service in sub-batches
            for meta_chunk in new_principals.chunks(METADATA_BATCH_SIZE) {
                let metadata_map =
                    match fetch_metadata_bulk(&http, &metadata_url, meta_chunk).await {
                        Ok(m) => m,
                        Err(e) => {
                            eprintln!("  metadata fetch error: {e}");
                            total_errors += meta_chunk.len() as u64;
                            continue;
                        }
                    };

                // For each principal with metadata, fetch existing SpacetimeDB profile,
                // enrich with username/email, and upsert
                let mut entries_json: Vec<String> = Vec::new();

                for principal_text in meta_chunk {
                    let (username, email) = match metadata_map.get(principal_text) {
                        Some((u, e)) => (u.clone(), e.clone()),
                        None => continue,
                    };

                    if username.is_none() && email.is_none() {
                        continue;
                    }

                    if username.is_some() {
                        total_with_username += 1;
                    }
                    if email.is_some() {
                        total_with_email += 1;
                    }

                    // Fetch existing profile from SpacetimeDB
                    match fetch_existing_profile_json(
                        &http,
                        &spacetime_url,
                        &db_name,
                        &spacetime_token,
                        principal_text,
                    )
                    .await
                    {
                        Ok(Some(profile_json)) => {
                            let entry_json = build_enriched_entry_json(
                                &profile_json,
                                &username,
                                &email,
                            );
                            entries_json.push(entry_json);
                        }
                        Ok(None) => {
                            total_skipped_no_profile += 1;
                        }
                        Err(e) => {
                            eprintln!("  fetch profile error for {principal_text}: {e}");
                            total_errors += 1;
                        }
                    }
                }

                if !entries_json.is_empty() {
                    let (ok, err) =
                        upsert_profile_batch(&http, &upsert_url, &spacetime_token, &entries_json)
                            .await;
                    total_upserted += ok as u64;
                    total_errors += err as u64;
                }
            }
        }

        eprintln!(
            "  Batch {batch_num}: {batch_len} posts, {new_count} new principals | Cumulative: {total_posts_read} posts, {total_seen} principals, {total_upserted} upserted, {total_with_username} with username, {total_with_email} with email, {total_skipped_no_profile} skipped, {total_errors} errors, cursor={cursor:?}",
            total_seen = seen_principals.len(),
            cursor = last_uuid,
        );

        if last_uuid.is_none() {
            break;
        }
    }

    eprintln!();
    eprintln!("=== Metadata Backfill Complete ===");
    eprintln!("  Total posts streamed:              {total_posts_read}");
    eprintln!("  Unique principals found:           {}", seen_principals.len());
    eprintln!("  Principals with username:          {total_with_username}");
    eprintln!("  Principals with email:             {total_with_email}");
    eprintln!("  Profiles upserted (enriched):      {total_upserted}");
    eprintln!("  Skipped (no SpacetimeDB profile):  {total_skipped_no_profile}");
    eprintln!("  Errors:                            {total_errors}");

    if total_errors > 0 {
        anyhow::bail!("{total_errors} errors (see above)");
    }

    Ok(())
}