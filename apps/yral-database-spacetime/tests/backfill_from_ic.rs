//! One-time IC→SpacetimeDB backfill test.
//!
//! Run with: `cargo test --package yral_database_spacetime backfill_from_ic -- --ignored --nocapture`
//!
//! This test is `#[ignore]`d so it doesn't run during normal `cargo test`.
//! It connects to the live IC canister (`user_post_service`, mainnet
//! `gxhc3-pqaaa-aaaas-qbh3q-cai`) and the target SpacetimeDB instance,
//! reads all posts via cursor-paginated `fetch_posts`, and upserts each
//! into SpacetimeDB via the `upsert_post` reducer.
//!
//! **Idempotent:** safe to run multiple times. Re-running with the same
//! post ID updates the row instead of duplicating (the reducer deletes-then-inserts
//! by primary key). This enables the two-run migration strategy:
//! 1. Run once at staging merge → seed data.
//! 2. Run again at app-store rollout → picks up delta posts + updated view counts.
//!
//! **Principal → Identity mapping:** IC `Principal` bytes are padded to 32 bytes
//! (big-endian) and used as the SpacetimeDB `Identity`. This is a simple
//! deterministic mapping. If yral-auth (Phase C) uses a different derivation,
//! the backfilled `creator` identities will need to be remapped. For now this
//! is sufficient to get the data in.
//!
//! **Delete this file once the mobile update has shipped and the IC canister
//! is decommissioned (Phase J cleanup).**

#![cfg(test)]

use std::sync::mpsc;

use canisters_client::user_post_service::{
    FetchPostsArgs, Post as IcPost, PostStatus as IcPostStatus,
    UserPostService,
};
use candid::Principal;
use ic_agent::Agent;
use spacetimedb_sdk::{DbContext, Identity, Timestamp};
// Generated bindings — provide `DbConnection`, `Post`, `PostStatus`, `upsert_post`.
#[path = "../src/bindings/mod.rs"]
mod bindings;

use bindings::{upsert_post, DbConnection, Post, PostStatus};

/// IC canister ID for `user_post_service` on mainnet.
const IC_CANISTER_ID: &str = "gxhc3-pqaaa-aaaas-qbh3q-cai";

/// IC boundary node URL.
const IC_URL: &str = "https://ic0.app";

/// Batch size for IC `fetch_posts` pagination.
const IC_BATCH_SIZE: u64 = 1000;

/// Map an IC `Principal` to a SpacetimeDB `Identity`.
///
/// SpacetimeDB derives an `Identity` deterministically from the `iss` + `sub`
/// claims of an OIDC JWT via `Identity::from_claims(issuer, subject)`. The
/// yral-auth `id_token` has `iss` = the yral-auth server URL and `sub` = the
/// IC principal text. We use the same derivation here so the backfilled
/// `creator` identities match what users get when they log in.
///
/// The issuer URL comes from the `YRAL_AUTH_ISSUER` env var (defaults to
/// `https://auth.yral.com`).
fn principal_to_identity(principal: &Principal) -> Identity {
    let issuer = std::env::var("YRAL_AUTH_ISSUER")
        .unwrap_or_else(|_| "https://auth.yral.com".to_string());
    Identity::from_claims(&issuer, &principal.to_text())
}

/// Map an IC `PostStatus` to the SpacetimeDB `PostStatus`.
fn map_status(ic_status: &IcPostStatus) -> PostStatus {
    match ic_status {
        IcPostStatus::Uploaded => PostStatus::Uploaded,
        IcPostStatus::Transcoding => PostStatus::Transcoding,
        IcPostStatus::CheckingExplicitness => PostStatus::CheckingExplicitness,
        IcPostStatus::BannedForExplicitness => PostStatus::BannedForExplicitness,
        IcPostStatus::ReadyToView => PostStatus::ReadyToView,
        IcPostStatus::BannedDueToUserReporting => PostStatus::BannedDueToUserReporting,
        IcPostStatus::Deleted => PostStatus::Deleted,
        IcPostStatus::Draft => PostStatus::Draft,
    }
}

/// Map an IC `SystemTime` to a SpacetimeDB `Timestamp`.
fn map_timestamp(secs: u64, nanos: u32) -> Timestamp {
    let micros = (secs as i64) * 1_000_000 + (nanos as i64) / 1_000;
    Timestamp::from_micros_since_unix_epoch(micros)
}

/// Map an IC `Post` to a SpacetimeDB `Post`.
fn map_post(ic_post: &IcPost) -> Post {
    Post {
        id: ic_post.id.clone(),
        creator: principal_to_identity(&ic_post.creator_principal),
        video_uid: ic_post.video_uid.clone(),
        description: ic_post.description.clone(),
        hashtags: ic_post.hashtags.clone(),
        status: map_status(&ic_post.status),
        created_at: map_timestamp(
            ic_post.created_at.secs_since_epoch,
            ic_post.created_at.nanos_since_epoch,
        ),
        share_count: ic_post.share_count,
        view_total_count: ic_post.view_stats.total_view_count,
        view_threshold_count: ic_post.view_stats.threshold_view_count,
        view_average_watch_percentage: ic_post.view_stats.average_watch_percentage,
    }
}

/// Build an anonymous IC agent (no identity needed — `fetch_posts` is a query).
async fn build_ic_agent() -> anyhow::Result<Agent> {
    let agent = Agent::builder()
        .with_url(IC_URL)
        .build()?;
    agent.fetch_root_key().await?;
    Ok(agent)
}

/// Scan all posts from the IC canister via cursor-paginated `fetch_posts`.
async fn fetch_all_ic_posts(agent: &Agent) -> anyhow::Result<Vec<IcPost>> {
    let canister_id = Principal::from_text(IC_CANISTER_ID)?;
    let post_service = UserPostService(canister_id, agent);

    let mut all_posts = Vec::new();
    let mut last_uuid: Option<String> = None;

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

        let batch_size = result.posts.len();
        last_uuid = result.last_post_id_fetched.clone();
        all_posts.extend(result.posts);

        eprintln!(
            "Fetched {} posts (total so far: {}), cursor: {:?}",
            batch_size,
            all_posts.len(),
            last_uuid
        );

        if last_uuid.is_none() {
            break;
        }
    }

    Ok(all_posts)
}

/// Connect to SpacetimeDB and upsert all posts.
///
/// Env vars (from `mise.toml [env]`):
/// - `SPACETIMEDB_DB_NAME` — database name (e.g. `yral-database-spacetime-4lbo7`)
/// - `SPACETIMEDB_URL` — server URL (e.g. `http://127.0.0.1:3000` for local,
///   `https://maincloud.spacetimedb.com` for Maincloud)
/// - `SPACETIMEDB_ADMIN_TOKEN` — admin auth token (from fnox for prod; for local,
///   use the token from `spacetime publish` output or `spacetime login`)
fn upsert_all_posts(posts: Vec<Post>) -> anyhow::Result<()> {
    let db_name = std::env::var("SPACETIMEDB_DB_NAME")
        .unwrap_or_else(|_| "yral-database-spacetime-4lbo7".to_string());
    let uri = std::env::var("SPACETIMEDB_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
    let token = std::env::var("SPACETIMEDB_ADMIN_TOKEN").ok();

    eprintln!("Connecting to SpacetimeDB: {} (db: {})", uri, db_name);

    // Use a channel to wait for the connect callback before proceeding.
    let (tx, rx) = mpsc::channel::<bool>();

    let conn = DbConnection::builder()
        .with_uri(uri)
        .with_database_name(db_name)
        .with_token(token)
        .on_connect(move |_conn, _identity, _token| {
            eprintln!("Connected to SpacetimeDB as identity: {}", _identity);
            let _ = tx.send(true);
        })
        .on_connect_error(|_ctx, err| {
            eprintln!("SpacetimeDB connect error: {err:?}");
        })
        .on_disconnect(|_ctx, err| {
            eprintln!("SpacetimeDB disconnected: {err:?}");
        })
        .build()?;

    // Drive the WebSocket on a background thread.
    let _handle = conn.run_threaded();

    // Wait for the connect callback.
    rx.recv_timeout(std::time::Duration::from_secs(10))
        .map_err(|e| anyhow::anyhow!("SpacetimeDB connection timeout: {e}"))?;

    eprintln!("Connected. Upserting {} posts...", posts.len());

    // Use a channel to track completion of all upsert calls.
    let total = posts.len();
    let (done_tx, done_rx) =
        mpsc::channel::<(usize, Result<Result<(), String>, spacetimedb_sdk::__codegen::InternalError>)>();

    for (i, post) in posts.into_iter().enumerate() {
        let done_tx = done_tx.clone();
        // Use the `_then` variant to get the reducer result.
        conn.reducers
            .upsert_post_then(post, move |_ctx, result| {
                let _ = done_tx.send((i, result));
            })
            .map_err(|e| anyhow::anyhow!("Failed to send upsert_post: {e}"))?;
    }
    drop(done_tx);

    // Collect all results.
    let mut errors = Vec::new();
    let mut success_count = 0usize;
    for (i, result) in done_rx {
        match result {
            Ok(Ok(())) => success_count += 1,
            Ok(Err(msg)) => {
                errors.push(format!("post[{}]: reducer error: {}", i, msg));
            }
            Err(internal_err) => {
                errors.push(format!("post[{}]: internal error: {}", i, internal_err));
            }
        }
        if (i + 1) % 100 == 0 {
            eprintln!("Progress: {}/{} upserts processed", i + 1, total);
        }
    }

    eprintln!("Upsert complete: {} succeeded, {} errors", success_count, errors.len());
    if !errors.is_empty() {
        eprintln!("First 10 errors:");
        for err in errors.iter().take(10) {
            eprintln!("  {err}");
        }
    }

    conn.disconnect()?;
    // Give the background thread a moment to flush.
    std::thread::sleep(std::time::Duration::from_millis(200));

    if !errors.is_empty() {
        anyhow::bail!("{} upsert errors (see above)", errors.len());
    }

    Ok(())
}

#[tokio::test]
#[ignore]
async fn backfill_from_ic() -> anyhow::Result<()> {
    eprintln!("=== IC → SpacetimeDB Backfill ===");
    eprintln!();

    // Phase 1: Read all posts from the IC canister.
    eprintln!("Phase 1: Reading posts from IC canister ({})...", IC_CANISTER_ID);
    let agent = build_ic_agent().await?;
    let ic_posts = fetch_all_ic_posts(&agent).await?;
    eprintln!("Read {} posts from IC", ic_posts.len());

    if ic_posts.is_empty() {
        eprintln!("No posts to backfill. Exiting.");
        return Ok(());
    }

    // Spot-check: print first 3 posts.
    for (i, post) in ic_posts.iter().take(3).enumerate() {
        eprintln!(
            "  [{}] id={}, creator={}, status={:?}, views={}",
            i, post.id, post.creator_principal, post.status, post.view_stats.total_view_count
        );
    }

    // Phase 2: Map IC posts to SpacetimeDB posts.
    eprintln!();
    eprintln!("Phase 2: Mapping {} IC posts → SpacetimeDB Post structs...", ic_posts.len());
    let st_posts: Vec<Post> = ic_posts.iter().map(map_post).collect();

    // Phase 3: Upsert into SpacetimeDB.
    eprintln!();
    eprintln!("Phase 3: Upserting into SpacetimeDB...");
    upsert_all_posts(st_posts)?;

    // Phase 4: Summary.
    eprintln!();
    eprintln!("=== Backfill Complete ===");
    eprintln!("  IC posts read:   {}", ic_posts.len());
    eprintln!("  SpacetimeDB upserts: completed");
    eprintln!();
    eprintln!("To validate, run: spacetime sql \"SELECT count(*) FROM posts\"");

    Ok(())
}