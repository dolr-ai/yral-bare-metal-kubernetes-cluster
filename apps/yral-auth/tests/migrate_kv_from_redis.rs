//! One-time Redis → SpacetimeDB KV migration test for yral-auth data.
//!
//! Run with: `cargo test --package yral-auth --features ssr --test migrate_kv_from_redis migrate_kv_from_redis_to_spacetime -- --ignored --nocapture`
//!
//! This test is `#[ignore]`d so it doesn't run during normal `cargo test`.
//! It reads ALL keys with the `yral-auth:` prefix from the Redis/Dragonfly
//! store (via Sentinel TLS on port 26379, using the production DragonflyKV
//! connection) and writes them to SpacetimeDB via the `kv_set` reducer.
//!
//! **Idempotent:** uses `kv_set` which is an upsert (delete-then-insert by
//! primary key). Safe to run multiple times.
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

use std::collections::HashMap;

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
            url: std::env::var("SPACETIMEDB_URL")
                .context("SPACETIMEDB_URL")?,
            db_name: std::env::var("SPACETIMEDB_DB_NAME")
                .context("SPACETIMEDB_DB_NAME")?,
            token: std::env::var("SPACETIMEDB_ADMIN_TOKEN")
                .context("SPACETIMEDB_ADMIN_TOKEN")?,
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

/// Scan Redis for all keys with the `yral-auth:` prefix and return them as
/// a HashMap<String, String>.
///
/// Uses the production `DragonflyKV` (which connects via Redis Sentinel on
/// port 26379 with mutual TLS) to get a connection, then SCANs for keys and
/// reads each value via `HGET key auth` (the hash field format used by
/// DragonflyKV).
async fn scan_all_redis_keys() -> anyhow::Result<HashMap<String, String>> {
    use redis::AsyncCommands;

    eprintln!("Initializing DragonflyKV (Sentinel TLS connection)...");
    let dragonfly = DragonflyKV::new().await?;

    // Get a raw multiplexed connection from the pool for SCAN operations.
    // DragonflyKV only exposes read/write/has_key (HGET/HSET/EXISTS), so we
    // need a direct connection to run SCAN.
    let pool = dragonfly.pool_for_scan();
    let mut conn = pool.get().await?;

    // SCAN all keys with the yral-auth: prefix.
    let pattern = format!("{}:*", KEY_PREFIX);
    eprintln!("Scanning for keys matching pattern: {pattern}");

    let mut all_data: HashMap<String, String> = HashMap::new();
    let mut cursor: u64 = 0;

    loop {
        let (next_cursor, keys): (u64, Vec<String>) =
            redis::cmd("SCAN").arg(cursor).arg("MATCH").arg(&pattern).query_async(&mut conn).await?;

        for key in &keys {
            // DragonflyKV stores values as HGET key "auth"
            let value: Option<String> = conn.hget(key, "auth").await?;
            if let Some(v) = value {
                all_data.insert(key.clone(), v);
            }
        }

        cursor = next_cursor;
        if cursor == 0 {
            break;
        }
    }

    eprintln!("Found {} keys in Redis with prefix '{}:'", all_data.len(), KEY_PREFIX);
    Ok(all_data)
}

#[tokio::test]
#[ignore]
async fn migrate_kv_from_redis_to_spacetime() -> anyhow::Result<()> {
    eprintln!("=== Redis → SpacetimeDB KV Migration ===");

    // Phase 1: Read all data from Redis.
    eprintln!("Phase 1: Reading all KV data from Redis...");
    let redis_data = scan_all_redis_keys().await?;

    if redis_data.is_empty() {
        eprintln!("No data to migrate. Exiting.");
        return Ok(());
    }

    // Spot-check: print first 5 keys.
    for (i, (key, value)) in redis_data.iter().take(5).enumerate() {
        let value_preview = if value.len() > 80 {
            format!("{}... ({} bytes)", &value[..80], value.len())
        } else {
            value.clone()
        };
        eprintln!("  [{i}] key={key} value={value_preview}");
    }

    // Phase 2: Write all data to SpacetimeDB.
    eprintln!();
    eprintln!("Phase 2: Writing {} keys to SpacetimeDB...", redis_data.len());

    let spacetime = SpacetimeKV::from_env()?;
    let mut success_count = 0usize;
    let mut error_count = 0usize;

    for (key, value) in &redis_data {
        match spacetime.write(key.clone(), value.clone()).await {
            Ok(()) => {
                success_count += 1;
                if success_count % 50 == 0 {
                    eprintln!("  Progress: {success_count}/{total} keys written", total = redis_data.len());
                }
            }
            Err(e) => {
                error_count += 1;
                if error_count <= 5 {
                    eprintln!("  ERROR writing key '{key}': {e}");
                }
            }
        }
    }

    eprintln!();
    eprintln!("=== Migration Complete ===");
    eprintln!("  Total keys read from Redis: {}", redis_data.len());
    eprintln!("  Successfully written to SpacetimeDB: {success_count}");
    eprintln!("  Errors: {error_count}");

    if error_count > 0 {
        anyhow::bail!("{error_count} keys failed to migrate");
    }

    // Phase 3: Verify — read back a few keys from SpacetimeDB and compare.
    eprintln!();
    eprintln!("Phase 3: Verifying (spot-check 5 keys)...");
    let mut verified = 0;
    for (key, expected_value) in redis_data.iter().take(5) {
        match spacetime.read(key.clone()).await {
            Ok(Some(actual)) if actual == *expected_value => {
                verified += 1;
                eprintln!("  ✓ {key}");
            }
            Ok(Some(actual)) => {
                eprintln!("  ✗ {key} — value mismatch! (expected {} bytes, got {} bytes)", expected_value.len(), actual.len());
            }
            Ok(None) => {
                eprintln!("  ✗ {key} — key not found in SpacetimeDB!");
            }
            Err(e) => {
                eprintln!("  ✗ {key} — read error: {e}");
            }
        }
    }

    eprintln!("  Verified: {verified}/5 spot-checks");
    Ok(())
}