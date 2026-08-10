//! SpacetimeDB connection for the off-chain-agent.
//!
//! Establishes a persistent WebSocket connection to the SpacetimeDB database
//! using the shared `SPACETIMEDB_ADMIN_TOKEN` (a yral-auth admin JWT). The
//! connection is kept alive via `run_background_task()` and the handle is
//! stored in `AppState` for fire-and-forget reducer calls.
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
use spacetimedb_sdk::DbConnection;
use spacetimedb_sdk::Identity;
use yral_database_spacetime::bindings;

pub type SpacetimeConnection = DbConnection;

/// Initialize a persistent SpacetimeDB connection.
///
/// Reads `SPACETIMEDB_URL`, `SPACETIMEDB_DB_NAME`, and `SPACETIMEDB_ADMIN_TOKEN`
/// from environment variables. The connection is kept alive via
/// `run_background_task()` — a background tokio task handles the WebSocket
/// message loop.
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
        .with_module_name(db_name)
        .with_token(token)
        .build()?;

    // Keep the connection alive in a background task.
    conn.run_background_task();

    // Log the derived identity for debugging and for adding to ADMINS.
    let identity = conn.identity();
    log::info!("SpacetimeDB connected. Admin identity: {}", identity.to_hex());

    Ok(Arc::new(conn))
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
        .add_view_details(post_id, bindings::PostViewDetailsFromFrontend {
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
        .delete_post(post_id)
        .context("Failed to send delete_post reducer call")?;
    Ok(())
}