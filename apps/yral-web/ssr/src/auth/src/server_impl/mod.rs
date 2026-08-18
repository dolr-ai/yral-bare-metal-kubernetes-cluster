pub mod store;
#[cfg(feature = "oauth-ssr")]
pub mod yral;

use std::env;

use axum::response::IntoResponse;
use axum_extra::extract::{
    cookie::{Cookie, Key, SameSite},
    SignedCookieJar,
};
use http::header;
use leptos::prelude::*;
use leptos_axum::{extract_with_state, ResponseOptions};

use consts::auth::{ID_TOKEN_COOKIE, ID_TOKEN_MAX_AGE, ONE_HOUR_SECS, REFRESH_MAX_AGE, REFRESH_TOKEN_COOKIE};

use crate::{AnonymousIdentity, ExtractedIdentity};

/// Current time since UNIX_EPOCH.
fn current_epoch() -> web_time::Duration {
    web_time::SystemTime::now()
        .duration_since(web_time::SystemTime::UNIX_EPOCH)
        .unwrap()
}

fn set_cookies(resp: &ResponseOptions, jar: impl IntoResponse) {
    let resp_jar = jar.into_response();
    for cookie in resp_jar
        .headers()
        .get_all(header::SET_COOKIE)
        .into_iter()
        .cloned()
    {
        resp.append_header(header::SET_COOKIE, cookie);
    }
}

fn cookie_key() -> Key {
    use_context().unwrap_or_else(|| {
        // HACK: https://github.com/leptos-rs/leptos/issues/2112
        let cookie_key_str = env::var("COOKIE_KEY").expect("`COOKIE_KEY` is required!");
        let raw_key =
            hex::decode(cookie_key_str).expect("Invalid `COOKIE_KEY` (must be length 128 hex)");
        Key::from(&raw_key)
    })
}

pub fn update_user_identity(
    response_opts: &ResponseOptions,
    mut jar: SignedCookieJar,
    refresh_jwt: String,
) -> Result<(), ServerFnError> {
    let refresh_max_age = REFRESH_MAX_AGE;

    let refresh_cookie = Cookie::build((REFRESH_TOKEN_COOKIE, refresh_jwt))
        .http_only(true)
        .secure(true)
        .path("/")
        .same_site(SameSite::None)
        .partitioned(true)
        .max_age(refresh_max_age.try_into().unwrap());

    jar = jar.add(refresh_cookie);
    set_cookies(response_opts, jar);
    Ok(())
}

/// Set the ID_TOKEN cookie (non-httpOnly) so client-side WASM can read it
/// for SpacetimeDB authentication. Called during OAuth callback and token refresh.
pub fn set_id_token_cookie(
    response_opts: &ResponseOptions,
    mut jar: SignedCookieJar,
    id_token: String,
) -> Result<(), ServerFnError> {
    let id_cookie = Cookie::build((ID_TOKEN_COOKIE, id_token))
        .http_only(false)
        .secure(true)
        .path("/")
        .same_site(SameSite::None)
        .partitioned(true)
        .max_age(ID_TOKEN_MAX_AGE.try_into().unwrap());

    jar = jar.add(id_cookie);
    set_cookies(response_opts, jar);
    Ok(())
}

/// Decode the `exp` claim from a JWT without verifying the signature.
/// Returns the expiry as a Unix timestamp in seconds.
fn decode_jwt_exp(token: &str) -> Option<usize> {
    decode_jwt_claim(token, "exp").and_then(|v| v.as_u64()).map(|e| e as usize)
}

/// Decode the `sub` claim from a JWT without verifying the signature.
/// Returns the user_id (OAuth sub or UUID).
fn decode_jwt_sub(token: &str) -> Option<String> {
    decode_jwt_claim(token, "sub").and_then(|v| v.as_str().map(|s| s.to_string()))
}

/// Decode a specific claim from a JWT payload without verifying the signature.
fn decode_jwt_claim(token: &str, key: &str) -> Option<serde_json::Value> {
    use base64::Engine;
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(parts[1]))
        .ok()?;
    let claims: serde_json::Value = serde_json::from_slice(&payload).ok()?;
    claims.get(key).cloned()
}

/// Current time as seconds since UNIX_EPOCH.
fn current_epoch_secs() -> usize {
    current_epoch().as_secs() as usize
}

/// Get the current id_token from the ID_TOKEN cookie.
/// If the token has < 1h remaining, transparently refreshes it.
/// Returns None for anonymous users.
pub async fn get_id_token_impl() -> Result<Option<String>, ServerFnError> {
    let key = cookie_key();
    let jar: SignedCookieJar = extract_with_state(&key).await?;

    let Some(id_cookie) = jar.get(ID_TOKEN_COOKIE) else {
        return Ok(None);
    };
    let id_token = id_cookie.value().to_string();

    // Check if < 1h left — if so, refresh transparently
    if let Some(exp) = decode_jwt_exp(&id_token) {
        if exp < current_epoch_secs() + ONE_HOUR_SECS {
            return refresh_id_token_impl().await;
        }
    }

    Ok(Some(id_token))
}

/// Force-refresh the id_token using the httpOnly refresh_token cookie.
/// Exchanges the refresh_token at yral-auth's /oauth/token endpoint,
/// updates both cookies (ID_TOKEN + REFRESH_TOKEN), and returns the new id_token.
/// Returns None if not logged in.
pub async fn refresh_id_token_impl() -> Result<Option<String>, ServerFnError> {
    #[cfg(feature = "oauth-ssr")]
    {
        use openidconnect::{OAuth2TokenResponse, RefreshToken};
        use yral::YralOAuthClient;

        let key = cookie_key();
        let jar: SignedCookieJar = extract_with_state(&key).await?;

        let Some(refresh_cookie) = jar.get(REFRESH_TOKEN_COOKIE) else {
            return Ok(None);
        };

        // Try refreshing via the OAuth client (same mechanism as extract_identity_impl)
        let oauth2: YralOAuthClient = expect_context();
        let http_client = openidconnect::reqwest::Client::new();
        let token_res = oauth2
            .exchange_refresh_token(&RefreshToken::new(refresh_cookie.value().to_string()))
            .request_async(&http_client)
            .await
            .map_err(|e| ServerFnError::new(format!("Token refresh failed: {e}")))?;

        let id_token = token_res
            .extra_fields()
            .id_token()
            .ok_or_else(|| ServerFnError::new("yral-auth did not return an ID token"))?;

        // Get the raw JWT string of the id_token
        let id_token_str = id_token.to_string();

        let new_refresh_token = token_res
            .refresh_token()
            .map(|t| t.secret().clone())
            .unwrap_or_else(|| refresh_cookie.value().to_string());

        let resp: ResponseOptions = expect_context();
        // Update both cookies
        update_user_identity(&resp, jar.clone(), new_refresh_token)?;
        set_id_token_cookie(&resp, jar, id_token_str.clone())?;

        Ok(Some(id_token_str))
    }

    #[cfg(not(feature = "oauth-ssr"))]
    {
        Ok(None)
    }
}

/// Get the user's identifier (IC Principal text) from the ID_TOKEN JWT.
/// Decodes the `sub` claim without reconstructing any IC identity.
/// Returns None for anonymous users.
pub async fn get_user_identifier_impl() -> Result<Option<String>, ServerFnError> {
    use base64::Engine;

    // Use get_id_token_impl which handles cookie reading + refresh
    let id_token = get_id_token_impl().await?;

    let Some(id_token) = id_token else {
        return Ok(None);
    };

    // Decode the JWT payload (no signature verification needed —
    // the token was already verified when set by the OAuth callback)
    let parts: Vec<&str> = id_token.split('.').collect();
    if parts.len() < 2 {
        return Ok(None);
    }
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(parts[1]))
        .map_err(|e| ServerFnError::new(format!("Failed to decode JWT payload: {e}")))?;
    let claims: serde_json::Value = serde_json::from_slice(&payload)
        .map_err(|e| ServerFnError::new(format!("Failed to parse JWT claims: {e}")))?;

    // The `sub` claim is the IC Principal text
    let user_identifier = claims["sub"]
        .as_str()
        .map(|s| s.to_string());

    Ok(user_identifier)
}

pub async fn extract_identity_impl() -> Result<Option<ExtractedIdentity>, ServerFnError> {
    let key = cookie_key();
    let jar: SignedCookieJar = extract_with_state(&key).await?;

    let Some(refresh_token) = jar.get(REFRESH_TOKEN_COOKIE) else {
        return Ok(None);
    };
    let refresh_token = refresh_token.value().to_string();

    // Get the id_token (refresh if needed)
    let id_token = match get_id_token_impl().await? {
        Some(token) => token,
        None => return Ok(None),
    };

    // Extract user_id from the JWT sub claim
    let user_id = match decode_jwt_sub(&id_token) {
        Some(sub) => sub,
        None => return Ok(None),
    };

    Ok(Some(ExtractedIdentity {
        user_id,
        id_token,
        refresh_token,
    }))
}

pub async fn logout_identity_impl() -> Result<(), ServerFnError> {
    let key = cookie_key();
    let jar: SignedCookieJar = extract_with_state(&key).await?;
    let resp: ResponseOptions = expect_context();

    #[cfg(feature = "oauth-ssr")]
    {
        use openidconnect::OAuth2TokenResponse;
        let oauth_client: yral::YralOAuthClient = expect_context();
        let http_client = openidconnect::reqwest::Client::new();
        let token = oauth_client
            .exchange_client_credentials()
            .request_async(&http_client)
            .await?;

        let id_token = token
            .extra_fields()
            .id_token()
            .expect("Yral Auth V2 must return an ID token");
        let refresh_token = token
            .refresh_token()
            .expect("Yral Auth V2 must return a refresh token");

        let id_token_str = id_token.to_string();
        update_user_identity(&resp, jar.clone(), refresh_token.secret().clone())?;
        set_id_token_cookie(&resp, jar, id_token_str)?;
    }

    Ok(())
}

pub async fn generate_anonymous_identity_if_required_impl(
) -> Result<Option<AnonymousIdentity>, ServerFnError> {
    let key = cookie_key();
    let jar: SignedCookieJar = extract_with_state(&key).await?;

    if jar.get(REFRESH_TOKEN_COOKIE).is_some() {
        return Ok(None);
    }

    #[cfg(feature = "oauth-ssr")]
    {
        use openidconnect::OAuth2TokenResponse;
        let oauth_client: yral::YralOAuthClient = expect_context();
        let http_client = openidconnect::reqwest::Client::new();
        let token = oauth_client
            .exchange_client_credentials()
            .request_async(&http_client)
            .await;
        let token = match token {
            Ok(token) => token,
            Err(e) => {
                eprintln!("Request token error {e:?}");
                return Err(ServerFnError::new(format!(
                    "Failed to exchange client credentials: {e}",
                )));
            }
        };

        let refresh_token = token
            .refresh_token()
            .expect("Yral Auth V2 must return a refresh token");

        Ok(Some(AnonymousIdentity {
            refresh_token: refresh_token.secret().to_string(),
        }))
    }

    #[cfg(not(feature = "oauth-ssr"))]
    {
        Ok(None)
    }
}

pub async fn set_anonymous_identity_cookie_impl(refresh_jwt: String) -> Result<(), ServerFnError> {
    let key = cookie_key();
    let jar: SignedCookieJar = extract_with_state(&key).await?;

    let resp: ResponseOptions = expect_context();

    update_user_identity(&resp, jar, refresh_jwt)?;

    Ok(())
}
