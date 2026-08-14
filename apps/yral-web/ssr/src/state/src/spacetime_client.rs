//! Client-side (WASM) SpacetimeDB connection setup for yral-web.
//!
//! On the client (hydrate), we create a per-user `DbConnection` using the
//! id_token from the `ID_TOKEN` cookie (non-httpOnly). The connection runs
//! a WebSocket to SpacetimeDB and provides typed table access, reducers,
//! and subscriptions via the generated Rust bindings.
//!
//! Token lifecycle:
//! 1. Read `ID_TOKEN` cookie on page load
//! 2. If token has < 1h remaining, call `refresh_id_token` server function
//! 3. Create `DbConnection::builder().with_token(id_token)`
//! 4. On disconnect (token expired), re-fetch token and reconnect
//!
//! Anonymous users (no ID_TOKEN cookie) connect with `with_token(None)`.

#![cfg(feature = "hydrate")]

use std::sync::Arc;

use base64::Engine;
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use yral_database_spacetime_bindings::DbConnection;

use consts::auth::ID_TOKEN_COOKIE;

/// Read a cookie value by name from `document.cookie`.
fn get_cookie(name: &str) -> Option<String> {
    let window = web_sys::window()?;
    let document = window.document()?;
    let html_document: web_sys::HtmlDocument = document.dyn_into().ok()?;
    let cookies = html_document.cookie().ok()?;
    for cookie in cookies.split(';') {
        let cookie = cookie.trim();
        if let Some(rest) = cookie.strip_prefix(&format!("{name}=")) {
            return Some(rest.to_string());
        }
    }
    None
}

/// Decode the `exp` claim from a JWT without verifying the signature.
/// Returns the expiry as a Unix timestamp in seconds.
fn decode_jwt_exp(token: &str) -> Option<u64> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(parts[1]))
        .ok()?;
    let claims: serde_json::Value = serde_json::from_slice(&payload).ok()?;
    claims["exp"].as_u64()
}

/// Current time as seconds since UNIX_EPOCH.
fn current_epoch_secs() -> u64 {
    web_time::SystemTime::now()
        .duration_since(web_time::SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Check if a JWT token has less than 1 hour remaining.
fn is_token_expiring_soon(token: &str) -> bool {
    match decode_jwt_exp(token) {
        Some(exp) => exp < current_epoch_secs() + 3600,
        None => true, // Can't decode — treat as expired
    }
}

/// Initialize the client-side SpacetimeDB connection.
/// Called during hydration. Reads the id_token cookie, refreshes if needed,
/// and creates a `DbConnection`. Stores it in Leptos context.
pub async fn init_client_spacetime() -> Option<Arc<DbConnection>> {
    let spacetime_url =
        std::env::var("SPACETIMEDB_URL").unwrap_or_else(|_| "wss://maincloud.spacetimedb.com".to_string());
    let db_name = std::env::var("SPACETIMEDB_DB_NAME")
        .unwrap_or_else(|_| "yral-database-spacetime-4lbo7".to_string());

    // Read id_token from cookie (non-httpOnly, so JS/WASM can access it)
    let mut id_token = get_cookie(ID_TOKEN_COOKIE);

    // If token is expiring soon (< 1h), refresh via server function
    if let Some(ref token) = id_token {
        if is_token_expiring_soon(token) {
            match auth::refresh_id_token().await {
                Ok(Some(new_token)) => id_token = Some(new_token),
                Ok(None) => id_token = None,
                Err(e) => log::warn!("Failed to refresh id_token: {e}"),
            }
        }
    }

    // If still no token, try fetching it (handles case where cookie exists but wasn't read)
    if id_token.is_none() {
        match auth::get_id_token().await {
            Ok(Some(token)) => id_token = Some(token),
            Ok(None) => {}
            Err(e) => log::warn!("Failed to get id_token: {e}"),
        }
    }

    log::info!(
        "Connecting to SpacetimeDB at {spacetime_url}, database: {db_name}, authenticated: {}",
        id_token.is_some()
    );

    let conn = DbConnection::builder()
        .with_uri(spacetime_url)
        .with_database_name(db_name)
        .with_token(id_token)
        .on_connect(move |_ctx, identity, _token| {
            log::info!("SpacetimeDB connected. Identity: {}", identity.to_hex());
        })
        .on_connect_error(|_ctx, err| {
            log::error!("SpacetimeDB connection error: {err}");
        })
        .on_disconnect(|_ctx, _err| {
            log::warn!("SpacetimeDB disconnected — token may have expired");
        })
        .build()
        .await
        .map_err(|e| log::error!("SpacetimeDB connection failed: {e}"))
        .ok()?;

    // The browser SDK runs the WebSocket message loop internally —
    // no explicit run_async() needed.
    Some(Arc::new(conn))
}

/// Get the client-side SpacetimeDB connection from Leptos context.
/// Returns None if not yet initialized (e.g., during SSR).
pub fn client_spacetime_conn() -> Option<Arc<DbConnection>> {
    use_context()
}