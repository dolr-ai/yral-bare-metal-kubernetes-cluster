//! One-time IC→SpacetimeDB backfill test for user_info (profiles + follows).
//!
//! Run with: `cargo test --package yral_database_spacetime backfill_user_info_from_ic -- --ignored --nocapture`
//!
//! This test is `#[ignore]`d so it doesn't run during normal `cargo test`.
//!
//! ## Strategy
//!
//! 1. **Enumerate user principals** from a file (`BACKFILL_PRINCIPALS_FILE`
//!    env var — one IC Principal per line).
//! 2. **Fetch profiles** from IC `user_info_service` canister in batches of 100
//!    via `get_users_profile_details` (V7).
//! 3. **Upsert to SpacetimeDB** via the generated SDK bindings — typed
//!    `conn.reducers.upsert_user_profile_batch(entries)` fire-and-forget call.
//!    No manual JSON, no REST — native Rust structs all the way.
//!
//! **Idempotent:** safe to run multiple times (delete-then-insert by PK).
//! **Streaming:** nothing buffered beyond a single batch in memory.
//!
//! **Delete this file once the mobile update has shipped and the IC canister
//! is decommissioned (Phase J cleanup).**

#![cfg(test)]

use anyhow::Context;
use candid::Principal;
use canisters_client::user_info_service::{
    Result9, UserProfileDetailsForFrontendV7, UserInfoService,
};
use ic_agent::Agent;
use spacetimedb_sdk::{DbContext, Timestamp};
use yral_database_spacetime_bindings::{
    DbConnection,
    upsert_user_profile_batch, UserProfileBatchEntry, SubscriptionPlan, YralProSubscription,
};

/// IC canister ID for `user_info_service` on mainnet.
const IC_CANISTER_ID: &str = "ivkka-7qaaa-aaaas-qbg3q-cai";

/// IC boundary node URL.
const IC_URL: &str = "https://ic0.app";

/// Batch size for IC `get_users_profile_details` (max 100 per call).
const IC_PROFILE_BATCH_SIZE: usize = 100;

/// Build an anonymous IC agent (no identity needed — `get_users_profile_details` is a query).
async fn build_ic_agent() -> anyhow::Result<Agent> {
    let agent = Agent::builder().with_url(IC_URL).build()?;
    agent.fetch_root_key().await?;
    Ok(agent)
}

/// Map an IC `UserProfileDetailsForFrontendV7` to a SpacetimeDB `UserProfileBatchEntry`.
fn ic_profile_to_entry(principal: &Principal, profile: &UserProfileDetailsForFrontendV7) -> UserProfileBatchEntry {
    let pic = profile.profile_picture.as_ref();
    UserProfileBatchEntry {
        principal_text: principal.to_text(),
        bio: profile.bio.clone().unwrap_or_default(),
        website_url: profile.website_url.clone().unwrap_or_default(),
        profile_picture_url: pic.map(|p| p.url.clone()).unwrap_or_default(),
        followers_count: profile.followers_count,
        following_count: profile.following_count,
        subscription_plan: match &profile.subscription_plan {
            canisters_client::user_info_service::SubscriptionPlan::Free => SubscriptionPlan::Free,
            canisters_client::user_info_service::SubscriptionPlan::Pro(sub) => {
                SubscriptionPlan::Pro(YralProSubscription {
                    free_video_credits_left: sub.free_video_credits_left,
                    total_video_credits_alloted: sub.total_video_credits_alloted,
                })
            }
        },
        is_ai_influencer: profile.is_ai_influencer,
        is_nsfw: pic.map(|p| p.nsfw_info.is_nsfw).unwrap_or(false),
        nsfw_ec: pic.map(|p| p.nsfw_info.nsfw_ec.clone()).unwrap_or_default(),
        nsfw_gore: pic.map(|p| p.nsfw_info.nsfw_gore.clone()).unwrap_or_default(),
        csam_detected: pic.map(|p| p.nsfw_info.csam_detected).unwrap_or(false),
        last_access_time: Timestamp::UNIX_EPOCH,
    }
}

/// Enumerate all user principals from a file (one IC Principal per line).
fn enumerate_user_principals() -> anyhow::Result<Vec<Principal>> {
    eprintln!("Enumerating user principals from file...");

    let principals_file = std::env::var("BACKFILL_PRINCIPALS_FILE")
        .context("BACKFILL_PRINCIPALS_FILE not set — provide a file with one IC Principal per line")?;

    let content = std::fs::read_to_string(&principals_file)
        .with_context(|| format!("Failed to read {}", principals_file))?;

    let principals: Vec<Principal> = content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| Principal::from_text(l).ok())
        .collect();

    eprintln!("  Found {} principals", principals.len());
    Ok(principals)
}

/// Establish a SpacetimeDB connection via the SDK bindings.
async fn connect_spacetimedb() -> anyhow::Result<DbConnection> {
    let url = std::env::var("SPACETIMEDB_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
    let db_name = std::env::var("SPACETIMEDB_DB_NAME")
        .unwrap_or_else(|_| "yral-database-spacetime-4lbo7".to_string());
    let token = std::env::var("SPACETIMEDB_ADMIN_TOKEN")
        .context("SPACETIMEDB_ADMIN_TOKEN")?;

    eprintln!("Connecting to SpacetimeDB at {url}, database: {db_name}");

    let conn = DbConnection::builder()
        .with_uri(url)
        .with_database_name(db_name)
        .with_token(Some(token))
        .build()?;

    // Keep the connection alive in a background thread.
    conn.run_threaded();

    let identity = conn.identity();
    eprintln!("SpacetimeDB connected. Identity: {}", identity.to_hex());

    Ok(conn)
}

#[tokio::test]
#[ignore]
async fn backfill_user_info_from_ic() -> anyhow::Result<()> {
    eprintln!("=== IC → SpacetimeDB User Info Backfill (SDK bindings) ===");

    // --- IC agent ---
    eprintln!("Connecting to IC canister {}...", IC_CANISTER_ID);
    let agent = build_ic_agent().await?;
    let canister_id = Principal::from_text(IC_CANISTER_ID)?;
    let user_info_service = UserInfoService(canister_id, &agent);

    // --- SpacetimeDB SDK connection ---
    let conn = connect_spacetimedb().await?;

    // --- Enumerate user principals ---
    let principals = enumerate_user_principals()?;
    if principals.is_empty() {
        eprintln!("No principals found. Nothing to backfill.");
        return Ok(());
    }

    // --- Stream: fetch profiles from IC in batches → upsert to SpacetimeDB via SDK ---
    let mut total_read: u64 = 0;
    let mut total_upserted: u64 = 0;
    let mut total_errors: u64 = 0;
    let mut total_skipped: u64 = 0;
    let mut batch_num: u64 = 0;

    for chunk in principals.chunks(IC_PROFILE_BATCH_SIZE) {
        batch_num += 1;

        // Fetch profiles from IC (batch lookup)
        let result = match user_info_service
            .get_users_profile_details(chunk.to_vec())
            .await
        {
            Ok(r) => r,
            Err(e) => {
                eprintln!("  Batch {batch_num}: IC agent error: {e}");
                total_errors += chunk.len() as u64;
                continue;
            }
        };

        let profiles: Vec<(Principal, UserProfileDetailsForFrontendV7)> = match result {
            Result9::Ok(profiles) => {
                // IC silently skips not-found users. Match by principal_id.
                profiles
                    .into_iter()
                    .filter_map(|p| {
                        if chunk.contains(&p.principal_id) {
                            Some((p.principal_id, p))
                        } else {
                            None
                        }
                    })
                    .collect()
            }
            Result9::Err(e) => {
                eprintln!("  Batch {batch_num}: IC error: {e}");
                total_errors += chunk.len() as u64;
                continue;
            }
        };

        let skipped = chunk.len() - profiles.len();
        total_read += chunk.len() as u64;
        total_skipped += skipped as u64;

        if profiles.is_empty() {
            eprintln!(
                "  Batch {batch_num}: {} principals, all skipped (not found in IC)",
                chunk.len()
            );
            continue;
        }

        // Map IC profiles → SpacetimeDB UserProfileBatchEntry (typed Rust structs)
        let entries: Vec<UserProfileBatchEntry> = profiles
            .iter()
            .map(|(p, v)| ic_profile_to_entry(p, v))
            .collect();

        // Upsert to SpacetimeDB via SDK bindings (fire-and-forget, typed)
        match conn.reducers.upsert_user_profile_batch(entries.clone()) {
            Ok(()) => {
                total_upserted += entries.len() as u64;
                eprintln!(
                    "  Batch {batch_num}: {} principals → {} profiles, {} upserted, {} skipped | Cumulative: {} read, {} upserted, {} skipped, {} errors",
                    chunk.len(),
                    profiles.len(),
                    entries.len(),
                    skipped,
                    total_read,
                    total_upserted,
                    total_skipped,
                    total_errors,
                );
            }
            Err(e) => {
                eprintln!("  Batch {batch_num}: SpacetimeDB upsert error: {e}");
                total_errors += entries.len() as u64;
            }
        }

        // Small delay between batches to avoid overwhelming the connection
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    eprintln!();
    eprintln!("=== User Info Backfill Complete ===");
    eprintln!("  Total principals read:        {total_read}");
    eprintln!("  Profiles upserted:           {total_upserted}");
    eprintln!("  Skipped (not found in IC):    {total_skipped}");
    eprintln!("  Errors:                       {total_errors}");

    if total_errors > 0 {
        anyhow::bail!("{total_errors} errors (see above)");
    }

    Ok(())
}