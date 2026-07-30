//! One-time Redis → SpacetimeDB KV migration test for yral-auth data.
//!
//! Run with: `cargo test --package yral-auth --features ssr --test migrate_kv_from_redis migrate_kv_from_redis_to_spacetime -- --ignored --nocapture`
//!
//! This test is `#[ignore]`d so it doesn't run during normal `cargo test`.
//! It streams keys with the `yral-auth:` prefix from the Redis/Dragonfly
//! store (via Sentinel TLS on port 26379, using the production DragonflyKV
//! connection) and writes them to SpacetimeDB via the `kv_set` reducer.
//!
//! **Streaming / batched:** Uses Redis SCAN with COUNT 1000 to page through
//! keys. Each batch of keys is immediately fetched (HGET) and written to
//! SpacetimeDB before the next batch is fetched. Nothing is buffered in
//! memory beyond a single batch.
//!
//! **Idempotent:** `kv_set` is an upsert (delete-then-insert by primary key).
//! Safe to run multiple times, and safe to interrupt + restart — already-
//! migrated keys are simply overwritten with the same value.
//!
//! **Delete this file once the migration is confirmed and Redis is no longer
//! used for yral-auth data.**
//!
//! Required env vars (from fnox + mise.toml):
//! - `DRAGONFLY_REDIS_STORE_HOSTS` — comma-separated Sentinel hosts (from fnox)
//! - `DRAGONFLY_REDIS_STORE_PASSWORD` — Redis password (from fnox)
//! - `DRAGONFLY_REDIS_STORE_CA_CERT` — TLS CA cert (from fnox)
//! - `DRAGONFLY_REDIS_STORE_CLIENT_CERT` — TLS client cert (from fnox)
//! - `DRAGONFLY_REDIS_STORE_CLIENT_KEY` — TLS client key (from fnox)
//! - `SPACETIMEDB_URL` — SpacetimeDB server URL (from mise.toml)
//! - `SPACETIMEDB_DB_NAME` — SpacetimeDB database name (from mise.toml)
//! - `SPACETIMEDB_ADMIN_TOKEN` — yral-auth JWT for SpacetimeDB auth (from fnox)

#![cfg(test)]
#![cfg(feature = "ssr")]

use anyhow::Context;
use serde::Deserialize;

use yral_auth::kv::dragonfly_kv::{DragonflyKV, KEY_PREFIX};

/// Minimal SpacetimeDB KV client reimplemented inline (integration tests can't
/// access the crate's private `crate::kv::spacetime_kv::SpacetimeKV`).
///
/// Talks to the SpacetimeDB module via its REST API:
///  - `kv_get`  → POST .../call/kv_get  with `["key"]`, returns `{"value": Option<String>}`
///  - `kv_set`  → POST .../call/kv_set  with `["key", "value"]` (reducer, no body)
struct SpacetimeKV {
    client: reqwest::Client,
    url: String,
    db_name: String,
    token: String,
}

#[derive(Deserialize)]
struct KvGetResponse {
    value: Option<String>,
}

impl SpacetimeKV {
    fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            client: reqwest::Client::new(),
            url: std::env::var("SPACETIMEDB_URL").context("SPACETIMEDB_URL")?,
            db_name: std::env::var("SPACETIMEDB_DB_NAME").context("SPACETIMEDB_DB_NAME")?,
            token: std::env::var("SPACETIMEDB_ADMIN_TOKEN").context("SPACETIMEDB_ADMIN_TOKEN")?,
        })
    }

    fn call_url(&self, name: &str) -> String {
        format!(
            "{}/v1/database/{}/call/{}",
            self.url.trim_end_matches('/'),
            self.db_name,
            name
        )
    }

    async fn read(&self, key: String) -> anyhow::Result<Option<String>> {
        let resp = self
            .client
            .post(self.call_url("kv_get"))
            .bearer_auth(&self.token)
            .json(&serde_json::json!([key]))
            .send()
            .await
            .context("SpacetimeDB kv_get request failed")?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("SpacetimeDB kv_get returned {status}: {body}");
        }

        let parsed: KvGetResponse = resp
            .json()
            .await
            .context("SpacetimeDB kv_get response parse failed")?;
        Ok(parsed.value)
    }

    async fn write(&self, key: String, value: String) -> anyhow::Result<()> {
        let resp = self
            .client
            .post(self.call_url("kv_set"))
            .bearer_auth(&self.token)
            .json(&serde_json::json!([key, value]))
            .send()
            .await
            .context("SpacetimeDB kv_set request failed")?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("SpacetimeDB kv_set returned {status}: {body}");
        }

        Ok(())
    }
}

/// Batch size for SCAN + migrate.
const SCAN_COUNT: u64 = 1000;

#[tokio::test]
#[ignore]
async fn migrate_kv_from_redis_to_spacetime() -> anyhow::Result<()> {
    use redis::AsyncCommands;

    eprintln!("=== Redis → SpacetimeDB KV Migration (streaming) ===");

    // Initialize both stores.
    eprintln!("Initializing DragonflyKV (Sentinel TLS)...");
    let dragonfly = DragonflyKV::new().await?;
    let pool = dragonfly.pool_for_scan();
    let mut conn = pool.get().await?;

    eprintln!("Initializing SpacetimeDB KV...");
    let spacetime = SpacetimeKV::from_env()?;

    let pattern = format!("{}:*", KEY_PREFIX);
    eprintln!("Streaming keys matching pattern: {pattern} (COUNT {SCAN_COUNT})");

    let mut cursor: u64 = 0;
    let mut total_scanned: u64 = 0;
    let mut total_migrated: u64 = 0;
    let mut total_errors: u64 = 0;
    let mut batch_num: u64 = 0;

    // Stream: SCAN → HGET batch → write to SpacetimeDB → next batch.
    loop {
        let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg(&pattern)
            .arg("COUNT")
            .arg(SCAN_COUNT)
            .query_async(&mut conn)
            .await?;

        batch_num += 1;
        total_scanned += keys.len() as u64;

        // Process each key in this batch: HGET value → write to SpacetimeDB.
        let mut batch_ok = 0u64;
        let mut batch_err = 0u64;
        for key in &keys {
            // DragonflyKV stores values as HGET key "auth"
            let value: Option<String> = conn.hget(key, "auth").await?;
            let Some(value) = value else { continue };

            match spacetime.write(key.clone(), value).await {
                Ok(()) => batch_ok += 1,
                Err(e) => {
                    batch_err += 1;
                    if total_errors + batch_err <= 10 {
                        eprintln!("  ERROR writing key '{key}': {e}");
                    }
                }
            }
        }

        total_migrated += batch_ok;
        total_errors += batch_err;

        eprintln!(
            "  Batch {batch_num}: {} keys scanned, {batch_ok} migrated, {batch_err} errors | Cumulative: {total_migrated} migrated, {total_errors} errors, cursor={next_cursor}",
            keys.len(),
        );

        cursor = next_cursor;
        if cursor == 0 {
            break;
        }
    }

    eprintln!();
    eprintln!("=== Migration Complete ===");
    eprintln!("  Total keys scanned: {total_scanned}");
    eprintln!("  Successfully migrated: {total_migrated}");
    eprintln!("  Errors: {total_errors}");

    if total_errors > 0 {
        anyhow::bail!("{total_errors} keys failed to migrate");
    }

    // Spot-check: verify 5 keys by reading back from SpacetimeDB.
    if total_migrated == 0 {
        eprintln!("No data migrated — nothing to verify.");
        return Ok(());
    }

    eprintln!();
    eprintln!("Phase 2: Verifying (re-scan + spot-check 5 keys)...");
    let mut verify_cursor: u64 = 0;
    let mut verified = 0;
    let mut checked = 0;
    'outer: loop {
        let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
            .arg(verify_cursor)
            .arg("MATCH")
            .arg(&pattern)
            .arg("COUNT")
            .arg(SCAN_COUNT)
            .query_async(&mut conn)
            .await?;

        for key in &keys {
            let redis_value: Option<String> = conn.hget(key, "auth").await?;
            let Some(redis_value) = redis_value else { continue };

            match spacetime.read(key.clone()).await {
                Ok(Some(actual)) if actual == redis_value => {
                    verified += 1;
                    eprintln!("  ✓ {key}");
                }
                Ok(Some(actual)) => {
                    eprintln!(
                        "  ✗ {key} — mismatch (redis {} bytes, spacetime {} bytes)",
                        redis_value.len(),
                        actual.len()
                    );
                }
                Ok(None) => {
                    eprintln!("  ✗ {key} — not found in SpacetimeDB!");
                }
                Err(e) => {
                    eprintln!("  ✗ {key} — read error: {e}");
                }
            }

            checked += 1;
            if checked >= 5 {
                break 'outer;
            }
        }

        verify_cursor = next_cursor;
        if verify_cursor == 0 {
            break;
        }
    }

    eprintln!("  Verified: {verified}/{checked} spot-checks");
    Ok(())
}