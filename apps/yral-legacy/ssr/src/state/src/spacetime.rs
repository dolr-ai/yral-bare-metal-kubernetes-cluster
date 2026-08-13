//! SpacetimeDB connection setup for yral-legacy SSR.
//!
//! A shared WebSocket connection is established at server startup and
//! provided as Leptos context (`Arc<DbConnection>`). Call sites use the
//! generated bindings directly — inline `oneshot::channel` + `_then`
//! callback for one-shot reads, or subscriptions for reactive data.
//!
//! For hydrate (client-side), the SDK connection + subscriptions replace
//! the SSR one-shot reads with reactive local cache access.

#![cfg(feature = "ssr")]

use std::sync::Arc;

use anyhow::Context;
use leptos::prelude::*;
use spacetimedb_sdk::DbContext;
use yral_database_spacetime_bindings::DbConnection;

/// Initialize a SpacetimeDB connection from environment variables.
/// The connection runs on a background thread (`run_threaded`).
pub fn init_spacetime() -> anyhow::Result<Arc<DbConnection>> {
    let url = std::env::var("SPACETIMEDB_URL").context("SPACETIMEDB_URL is not set")?;
    let db_name = std::env::var("SPACETIMEDB_DB_NAME")
        .unwrap_or_else(|_| "yral-database-spacetime-4lbo7".to_string());
    let token =
        std::env::var("SPACETIMEDB_ADMIN_TOKEN").context("SPACETIMEDB_ADMIN_TOKEN is not set")?;

    log::info!("Connecting to SpacetimeDB at {url}, database: {db_name}");

    let conn = DbConnection::builder()
        .with_uri(url)
        .with_database_name(db_name)
        .with_token(Some(token))
        .on_connect(move |_ctx, identity, _token| {
            log::info!("SpacetimeDB connected. Identity: {}", identity.to_hex());
        })
        .build()?;

    let conn = Arc::new(conn);

    // Run the connection's message loop on the tokio runtime.
    let conn_loop = conn.clone();
    tokio::spawn(async move {
        loop {
            if conn_loop.run_async().await.is_err() {
                break;
            }
        }
    });

    Ok(conn)
}

/// Get the SpacetimeDB connection from Leptos context.
pub fn spacetime_conn() -> Arc<DbConnection> {
    expect_context()
}
