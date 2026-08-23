//! SSR-only implementation for account deletion.
//!
//! The `#[server]` function declarations live in the page files
//! (`page/account/account.rs` and `page/account/oauth_callback.rs`)
//! so that client-side stubs are available on hydrate. This module
//! provides the SSR implementations that those server functions delegate to.

use std::sync::Arc;

use axum::response::IntoResponse;
use axum_extra::extract::{
    cookie::{Cookie, SameSite},
    PrivateCookieJar,
};
use leptos::prelude::*;
use leptos_axum::{extract_with_state, ResponseOptions};
use web_time::Duration;

use crate::{
    consts::OFF_CHAIN_AGENT_URL,
    context::server::{expect_server_ctx, ServerCtx},
    kv::KVStore,
};

/// Cookie name for the account session (stores the user's ID).
pub const DELETE_ACCOUNT_SESSION_COOKIE: &str = "delete-account-session";

/// Cookie max age: 10 minutes.
const SESSION_COOKIE_MAX_AGE: Duration = Duration::from_secs(10 * 60);

/// Self-service OAuth client ID (registered in the whitelist).
pub const SELF_SERVICE_CLIENT_ID: &str = "7a2f3b8c-1d4e-4f5a-9b6c-7d8e9f0a1b2c";

// ---------------------------------------------------------------------------
// Session cookie helpers
// ---------------------------------------------------------------------------

/// Reads the authenticated user ID from the encrypted session cookie.
pub async fn read_session_user_id() -> Result<Option<String>, ServerFnError> {
    let ctx = expect_server_ctx();
    let jar: PrivateCookieJar = extract_with_state(&ctx.cookie_key)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to extract cookie jar: {e:?}")))?;

    Ok(jar
        .get(DELETE_ACCOUNT_SESSION_COOKIE)
        .map(|c| c.value().to_string()))
}

/// Stores the user ID in the encrypted session cookie.
async fn set_session_user_id(user_id: &str) -> Result<(), ServerFnError> {
    use axum::http::header;

    let ctx = expect_server_ctx();
    let jar: PrivateCookieJar = extract_with_state(&ctx.cookie_key)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to extract cookie jar: {e:?}")))?;

    let cookie_life = SESSION_COOKIE_MAX_AGE.try_into().unwrap();
    let cookie = Cookie::build((DELETE_ACCOUNT_SESSION_COOKIE, user_id.to_string()))
        .same_site(SameSite::Lax)
        .secure(true)
        .path("/")
        .max_age(cookie_life)
        .http_only(true)
        .build();

    let jar = jar.add(cookie);
    let resp: ResponseOptions = expect_context();
    let resp_jar = jar.into_response();
    for cookie in resp_jar
        .headers()
        .get_all(header::SET_COOKIE)
        .into_iter()
        .cloned()
    {
        resp.append_header(header::SET_COOKIE, cookie);
    }
    Ok(())
}

/// Clears the session cookie.
pub async fn clear_session_principal() -> Result<(), ServerFnError> {
    use axum::http::header;

    let ctx = expect_server_ctx();
    let jar: PrivateCookieJar = extract_with_state(&ctx.cookie_key)
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to extract cookie jar: {e:?}")))?;

    // Build an explicit removal cookie with matching Path to ensure the browser
    // deletes the original cookie. Browsers require Path to match.
    let removal_cookie = Cookie::build((
        DELETE_ACCOUNT_SESSION_COOKIE,
        "",
    ))
        .same_site(SameSite::Lax)
        .secure(true)
        .path("/")
        .max_age(Duration::ZERO.try_into().unwrap())
        .http_only(true)
        .build();

    let jar = jar.add(removal_cookie);
    let resp: ResponseOptions = expect_context();
    let resp_jar = jar.into_response();
    for cookie in resp_jar
        .headers()
        .get_all(header::SET_COOKIE)
        .into_iter()
        .cloned()
    {
        resp.append_header(header::SET_COOKIE, cookie);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Public impl functions (called from #[server] fns in page files)
// ---------------------------------------------------------------------------

/// Completes the OAuth login by decoding the auth code JWT and storing
/// the user ID in an encrypted session cookie.
pub async fn complete_account_login_impl(code: String) -> Result<(), ServerFnError> {
    use crate::oauth::jwt::AuthCodeClaims;

    let ctx = expect_server_ctx();

    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::ES256);
    validation.set_audience(&[SELF_SERVICE_CLIENT_ID]);
    validation.set_issuer(&["https://auth.yral.com"]);

    let auth_code = jsonwebtoken::decode::<AuthCodeClaims>(
        &code,
        &ctx.jwk_pairs.auth_tokens.decoding_key,
        &validation,
    )
    .map_err(|e| ServerFnError::new(format!("Failed to decode auth code: {e}")))?;

    let user_id = auth_code.claims.sub;

    set_session_user_id(&user_id).await?;

    Ok(())
}

/// Deletes the user's account.
///
/// Reads the user ID from the session cookie, generates an access token,
/// and calls the off-chain agent's `DELETE /api/v1/user` endpoint with
/// the token as a Bearer header.
pub async fn delete_account_impl() -> Result<(), ServerFnError> {
    let ctx = expect_context::<Arc<ServerCtx>>();

    // 1. Read the user ID from the session cookie
    let user_id = read_session_user_id()
        .await?
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;

    // 2. Generate a short-lived access token for the user
    let server_url = crate::utils::server_url::get_server_url_from_request()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to get server URL: {e}")))?;

    let access_token = crate::oauth::jwt::generate::generate_access_token_and_id_token_jwt(
        &ctx.jwk_pairs.auth_tokens.encoding_key,
        &user_id,
        SELF_SERVICE_CLIENT_ID,
        None,
        false,
        Duration::from_secs(10 * 60),
        None,
        Vec::new(),
        &server_url,
    ).0; // Take only the access token

    // 3. Call the off-chain agent's delete endpoint with Bearer token
    let client = reqwest::Client::new();
    let url = OFF_CHAIN_AGENT_URL.join("api/v1/user").unwrap();

    let response = client
        .delete(url)
        .bearer_auth(&access_token)
        .send()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to call delete API: {e}")))?;

    if response.status().is_success() {
        // 4. Delete user data from KV
        let existence_key = format!("user:{user_id}");
        ctx.kv_store
            .write(existence_key, "".to_string())
            .await
            .map_err(|e| ServerFnError::new(format!("KV error: {e}")))?;

        // 5. Clear the session cookie
        clear_session_principal().await?;
        Ok(())
    } else {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        Err(ServerFnError::new(format!(
            "Delete user failed with status {status}: {body}"
        )))
    }
}