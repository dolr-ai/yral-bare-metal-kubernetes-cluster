//! SpacetimeDB connection for the off-chain-agent.
//!
//! Establishes a persistent WebSocket connection to the SpacetimeDB database
//! using the shared `SPACETIMEDB_ADMIN_TOKEN` (a yral-auth admin JWT). The
//! connection is kept alive via `run_async()` spawned as a background tokio
//! task, and the handle is stored in `AppState` for fire-and-forget reducer
//! calls.
//!
//! The off-chain-agent uses the generated `spacetimedb-sdk` bindings (not REST)
//! per the AGENTS.md rule: "Rust services interact via generated `spacetimedb-sdk`
//! bindings ... Never send raw SQL strings from Rust."
//!
//! Auth model: SpacetimeDB derives an `Identity` from the JWT's `iss` + `sub`
//! claims via `Identity::from_claims(issuer, subject)`. The admin token's
//! derived identity must be in `constants.rs:ADMINS` for admin reducers
//! (e.g. `delete_post`) to work. View-count updates (`add_view_details`)
//! require no admin check — any caller can invoke them.

use std::env;
use std::sync::Arc;

use anyhow::{Context, Result};
use yral_database_spacetime_bindings::{
    self as bindings, DbConnection,
    add_view_details, delete_post,
};

pub type SpacetimeConnection = DbConnection;

/// Initialize a persistent SpacetimeDB connection.
///
/// Reads `SPACETIMEDB_URL`, `SPACETIMEDB_DB_NAME`, and `SPACETIMEDB_ADMIN_TOKEN`
/// from environment variables. The connection is kept alive via `run_async()`
/// spawned as a background tokio task that pumps the WebSocket message loop.
pub async fn init_spacetimedb_connection() -> Result<Arc<SpacetimeConnection>> {
    let url = env::var("SPACETIMEDB_URL")
        .context("SPACETIMEDB_URL is not set")?;
    let db_name = env::var("SPACETIMEDB_DB_NAME")
        .context("SPACETIMEDB_DB_NAME is not set")?;
    let token = env::var("SPACETIMEDB_ADMIN_TOKEN")
        .context("SPACETIMEDB_ADMIN_TOKEN is not set");

    let token = match token {
        Ok(t) => Some(t),
        Err(_) => {
            log::warn!("SPACETIMEDB_ADMIN_TOKEN not set — connecting anonymously (view-count calls will work, admin reducers will fail)");
            None
        }
    };

    log::info!("Connecting to SpacetimeDB at {url}, database: {db_name}");

    let conn = DbConnection::builder()
        .with_uri(url)
        .with_database_name(db_name)
        .with_token(token)
        .on_connect(move |_ctx, identity, _token| {
            log::info!("SpacetimeDB connected. Admin identity: {}", identity.to_hex());
        })
        .build()?;

    let conn = Arc::new(conn);

    // Spawn a background tokio task to pump the WebSocket message loop.
    // `run_async()` internally clones the connection handle, so we can pass
    // a reference here while still returning the original `Arc` to the caller.
    // The task runs concurrently with the axum server without blocking it.
    tokio::spawn({
        let conn = conn.clone();
        async move {
            if let Err(e) = conn.run_async().await {
                log::error!("SpacetimeDB background task ended with error: {e:?}");
            }
        }
    });

    Ok(conn)
}

/// Send a fire-and-forget view-count update to SpacetimeDB.
/// Mirrors the IC `update_post_add_view_details` call.
///
/// The IC version used an enum (`WatchedPartially`/`WatchedMultipleTimes`);
/// SpacetimeDB uses a flat struct `{ percentage_watched, watch_count }`.
/// Both branches of the IC enum produce the same struct fields with
/// different values, so we just construct the struct directly.
pub fn send_view_details(
    conn: &SpacetimeConnection,
    post_id: String,
    percentage_watched: u8,
    watch_count: u8,
) -> Result<()> {
    conn.reducers
        .add_view_details(post_id.clone(), bindings::PostViewDetailsFromFrontend {
            percentage_watched,
            watch_count,
        })
        .context("Failed to send add_view_details reducer call")?;
    Ok(())
}

/// Send a fire-and-forget post delete to SpacetimeDB.
/// Uses the admin identity (from `SPACETIMEDB_ADMIN_TOKEN`). The off-chain-agent
/// verifies ownership via HTTP middleware before calling this.
pub fn send_delete_post(conn: &SpacetimeConnection, post_id: String) -> Result<()> {
    conn.reducers
        .delete_post(post_id.clone())
        .context("Failed to send delete_post reducer call")?;
    Ok(())
}