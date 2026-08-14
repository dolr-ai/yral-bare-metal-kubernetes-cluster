pub mod store;
#[cfg(feature = "oauth-ssr")]
pub mod yral;

use std::env;

use axum::response::IntoResponse;
use axum_extra::extract::{
    cookie::{Cookie, Key, SameSite},
    SignedCookieJar,
};
use candid::Principal;
use http::header;
use ic_agent::{identity::Secp256k1Identity, Identity};
use k256::elliptic_curve::JwkEcKey;
use leptos::prelude::*;
use leptos_axum::{extract_with_state, ResponseOptions};
use rand_chacha::rand_core::OsRng;

use consts::auth::{ID_TOKEN_COOKIE, ID_TOKEN_MAX_AGE, ONE_HOUR_SECS, REFRESH_MAX_AGE, REFRESH_TOKEN_COOKIE};

use crate::{delegate_identity, AnonymousIdentity};

use self::store::{KVStore, KVStoreImpl};

/// Current time since UNIX_EPOCH.
fn current_epoch() -> web_time::Duration {
    web_time::SystemTime::now()
        .duration_since(web_time::SystemTime::UNIX_EPOCH)
        .unwrap()
}
use types::delegated_identity::DelegatedIdentityWire;

use super::RefreshTokenLegacy;

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

pub fn extract_principal_from_cookie_legacy(
    jar: &SignedCookieJar,
) -> Result<Option<Principal>, ServerFnError> {
    let Some(cookie) = jar.get(REFRESH_TOKEN_COOKIE) else {
        return Ok(None);
    };
    let token: RefreshTokenLegacy = serde_json::from_str(cookie.value())?;
    if current_epoch().as_millis() > token.expiry_epoch_ms {
        return Ok(None);
    }
    Ok(Some(token.principal))
}

async fn fetch_identity_from_kv(
    kv: &KVStoreImpl,
    principal: Principal,
) -> Result<Option<k256::SecretKey>, ServerFnError> {
    let Some(identity_jwk) = kv.read(principal.to_text()).await? else {
        return Ok(None);
    };

    Ok(Some(k256::SecretKey::from_jwk_str(&identity_jwk)?))
}

pub async fn try_extract_identity_legacy(
    jar: &SignedCookieJar,
    kv: &KVStoreImpl,
) -> Result<Option<k256::SecretKey>, ServerFnError> {
    let Some(principal) = extract_principal_from_cookie_legacy(jar)? else {
        return Ok(None);
    };
    fetch_identity_from_kv(kv, principal).await
}

async fn generate_and_save_identity_legacy(
    kv: &KVStoreImpl,
) -> Result<Secp256k1Identity, ServerFnError> {
    let base_identity_key = k256::SecretKey::random(&mut OsRng);
    let base_identity = Secp256k1Identity::from_private_key(base_identity_key.clone());
    let principal = base_identity.sender().unwrap();

    let base_jwk = base_identity_key.to_jwk_string();
    kv.write(principal.to_text(), base_jwk.to_string()).await?;
    Ok(base_identity)
}

fn identity_from_jwk(id: &JwkEcKey) -> Result<Secp256k1Identity, ServerFnError> {
    let base_identity_key = k256::SecretKey::from_jwk(id)?;
    let base_identity: Secp256k1Identity =
        Secp256k1Identity::from_private_key(base_identity_key.clone());
    Ok(base_identity)
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
    claims["exp"].as_u64().map(|e| e as usize)
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

async fn extract_identity_legacy(
    jar: &SignedCookieJar,
    refresh_token: &Cookie<'static>,
) -> Result<Option<DelegatedIdentityWire>, ServerFnError> {
    if serde_json::from_str::<RefreshTokenLegacy>(refresh_token.value()).is_err() {
        return Ok(None);
    }

    let kv: KVStoreImpl = expect_context();
    let Some(id) = try_extract_identity_legacy(jar, &kv).await? else {
        return Ok(None);
    };
    let base_identity = Secp256k1Identity::from_private_key(id);

    let id = delegate_identity(&base_identity);

    Ok(Some(id))
}

pub async fn extract_identity_impl() -> Result<Option<DelegatedIdentityWire>, ServerFnError> {
    let key = cookie_key();
    let jar: SignedCookieJar = extract_with_state(&key).await?;

    #[cfg(not(feature = "oauth-ssr"))]
    {
        let kv: KVStoreImpl = expect_context();
        let base_identity = if let Some(identity) = try_extract_identity_legacy(&jar, &kv).await? {
            Secp256k1Identity::from_private_key(identity)
        } else {
            return Ok(None);
        };

        Ok(Some(delegate_identity(&base_identity)))
    }

    #[cfg(feature = "oauth-ssr")]
    {
        use openidconnect::RefreshToken;
        use yral::YralOAuthClient;

        let Some(refresh_token) = jar.get(REFRESH_TOKEN_COOKIE) else {
            return Ok(None);
        };

        if let Some(id) = extract_identity_legacy(&jar, &refresh_token).await? {
            return Ok(Some(id));
        }

        let oauth2: YralOAuthClient = expect_context();
        let http_client = openidconnect::reqwest::Client::new();
        let token_res = oauth2
            .exchange_refresh_token(&RefreshToken::new(refresh_token.value().to_string()))
            .request_async(&http_client)
            .await?;

        let id_token = token_res
            .extra_fields()
            .id_token()
            .expect("Yral Auth V2 must return an ID token");
        let id_claims = id_token.claims(&yral::token_verifier(), yral::no_op_nonce_verifier)?;
        let identity = id_claims.additional_claims().ext_delegated_identity.clone();

        Ok(Some(identity))
    }
}

pub async fn logout_identity_impl() -> Result<DelegatedIdentityWire, ServerFnError> {
    let key = cookie_key();
    let jar: SignedCookieJar = extract_with_state(&key).await?;
    let resp: ResponseOptions = expect_context();

    #[cfg(not(feature = "oauth-ssr"))]
    {
        let kv: KVStoreImpl = expect_context();
        let identity = generate_and_save_identity_legacy(&kv).await?;

        let refresh_token = serde_json::to_string(&RefreshTokenLegacy {
            principal: identity.sender().unwrap(),
            expiry_epoch_ms: (current_epoch() + REFRESH_MAX_AGE).as_millis(),
        })
        .unwrap();

        update_user_identity(&resp, jar.clone(), refresh_token)?;

        let delegated = delegate_identity(&identity);

        // Set anonymous id_token cookie (non-httpOnly)
        let anon_id_token_str = serde_json::to_string(&RefreshTokenLegacy {
            principal: identity.sender().unwrap(),
            expiry_epoch_ms: (current_epoch() + REFRESH_MAX_AGE).as_millis(),
        })
        .unwrap();
        set_id_token_cookie(&resp, jar, anon_id_token_str)?;

        Ok(delegated)
    }

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

        let id_claims = id_token.claims(&yral::token_verifier(), yral::no_op_nonce_verifier)?;
        let identity = id_claims.additional_claims().ext_delegated_identity.clone();
        let id_token_str = id_token.to_string();
        update_user_identity(&resp, jar.clone(), refresh_token.secret().clone())?;
        set_id_token_cookie(&resp, jar, id_token_str)?;

        Ok(identity)
    }
}

pub async fn generate_anonymous_identity_if_required_impl(
) -> Result<Option<AnonymousIdentity>, ServerFnError> {
    let key = cookie_key();
    let jar: SignedCookieJar = extract_with_state(&key).await?;
    #[cfg(not(feature = "oauth-ssr"))]
    {
        if extract_principal_from_cookie_legacy(&jar)?.is_some() {
            return Ok(None);
        }

        let kv: KVStoreImpl = expect_context();
        let identity = generate_and_save_identity_legacy(&kv).await?;
        Ok(Some(AnonymousIdentity {
            identity: delegate_identity(&identity).into(),
            refresh_token: serde_json::to_string(&RefreshTokenLegacy {
                principal: identity.sender().unwrap(),
                expiry_epoch_ms: (current_epoch() + REFRESH_MAX_AGE).as_millis(),
            })
            .unwrap(),
        }))
    }

    #[cfg(feature = "oauth-ssr")]
    {
        if jar.get(REFRESH_TOKEN_COOKIE).is_some() {
            return Ok(None);
        }

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

        let id_token = token
            .extra_fields()
            .id_token()
            .expect("Yral Auth V2 must return an ID token");
        let refresh_token = token
            .refresh_token()
            .expect("Yral Auth V2 must return a refresh token");

        let id_claims = id_token.claims(&yral::token_verifier(), yral::no_op_nonce_verifier)?;
        let identity = id_claims.additional_claims().ext_delegated_identity.clone();

        Ok(Some(AnonymousIdentity {
            identity,
            refresh_token: refresh_token.secret().to_string(),
        }))
    }
}

pub async fn set_anonymous_identity_cookie_impl(refresh_jwt: String) -> Result<(), ServerFnError> {
    let key = cookie_key();
    let jar: SignedCookieJar = extract_with_state(&key).await?;

    let resp: ResponseOptions = expect_context();

    update_user_identity(&resp, jar, refresh_jwt)?;

    Ok(())
}
