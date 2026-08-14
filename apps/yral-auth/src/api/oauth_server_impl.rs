use crate::{
    api::identity_provider::{try_extract_user_id_from_oauth_sub, user_id_from_oauth_or_create},
    context::server::expect_server_ctx,
    error::AuthErrorKind,
    kv::{KVStore, KVStoreImpl},
    oauth::{jwt::generate::generate_code_grant_jwt, AuthQuery, SupportedOAuthProviders},
    oauth_provider::OAuthProvider,
    utils::server_url::get_server_url_from_request,
};
use axum::{
    extract::Form,
    http::header,
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::{
    cookie::{Cookie, SameSite},
    PrivateCookieJar,
};
use base64::{prelude::BASE64_URL_SAFE, Engine};
use leptos::prelude::{expect_context, ServerFnError};
use leptos_axum::{extract_with_state, ResponseOptions};
use openidconnect::{
    core::CoreAuthenticationFlow, AuthorizationCode, CsrfToken, Nonce, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl,
};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use web_time::Duration;

const PKCE_VERIFIER_COOKIE: &str = "oauth-pkce-verifier";
const CSRF_TOKEN_COOKIE: &str = "oauth-csrf-token";

#[derive(Serialize, Deserialize)]
struct OAuthState {
    pub csrf_token: CsrfToken,
    pub provider: SupportedOAuthProviders,
    pub client_state: String,
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

pub async fn get_oauth_url_impl(
    provider: SupportedOAuthProviders,
    client_state: String,
) -> Result<String, ServerFnError> {
    let ctx = expect_server_ctx();

    let oauth_provider = ctx
        .oauth_providers
        .get(&provider)
        .ok_or_else(|| ServerFnError::new("unsupported provider"))?
        .get_client();

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let oauth_state = OAuthState {
        csrf_token: CsrfToken::new_random(),
        provider,
        client_state,
    };
    let oauth_state_raw = postcard::to_stdvec(&oauth_state)
        .map_err(|_| ServerFnError::new("failed to serialize oauth state"))?;
    let oauth_state_b64 = BASE64_URL_SAFE.encode(oauth_state_raw);

    let server_url = get_server_url_from_request().await.map_err(|e| {
        let err_msg = format!("failed to get server url: {:?}", e);
        log::error!("{err_msg}");
        e
    })?;

    let redirect_uri = RedirectUrl::new(format!("{server_url}/oauth_callback"))
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let authorize_builder = oauth_provider
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            move || CsrfToken::new(oauth_state_b64),
            Nonce::new_random,
        )
        .set_redirect_uri(Cow::Owned(redirect_uri));

    #[cfg(feature = "apple-oauth")]
    let authorize_builder = if provider != SupportedOAuthProviders::Apple {
        authorize_builder.set_pkce_challenge(pkce_challenge)
    } else {
        // Apple doesn't support PKCE, so we skip setting the challenge
        authorize_builder
    };
    #[cfg(not(feature = "apple-oauth"))]
    let authorize_builder = authorize_builder.set_pkce_challenge(pkce_challenge);

    #[cfg(feature = "google-oauth")]
    let authorize_builder = {
        use SupportedOAuthProviders::Google;
        if provider == Google {
            authorize_builder.add_scope(Scope::new("email".to_string()))
        } else {
            authorize_builder
        }
    };

    #[cfg(feature = "apple-oauth")]
    let authorize_builder = {
        if provider == SupportedOAuthProviders::Apple {
            authorize_builder
                .add_scope(Scope::new("email".to_string()))
                .add_scope(Scope::new("name".to_string()))
                .add_extra_param("response_mode", "form_post")
        } else {
            authorize_builder
        }
    };

    #[cfg(not(feature = "google-oauth"))]
    let authorize_builder = authorize_builder;

    let (auth_url, oauth_csrf_token, _) = authorize_builder.url();

    let mut jar: PrivateCookieJar = extract_with_state(&ctx.cookie_key).await.map_err(|e| {
        let err_msg = format!("failed to extract cookie jar: {:?}", e);
        log::error!("{err_msg}");
        e
    })?;

    let cookie_life = Duration::from_secs(60 * 10).try_into().unwrap(); // 10 minutes
    let pkce_cookie = Cookie::build((PKCE_VERIFIER_COOKIE, pkce_verifier.secret().clone()))
        .same_site(SameSite::None)
        .secure(true)
        .path("/")
        .max_age(cookie_life)
        .http_only(true)
        .build();
    jar = jar.add(pkce_cookie);

    let csrf_cookie = Cookie::build((CSRF_TOKEN_COOKIE, oauth_csrf_token.secret().clone()))
        .same_site(SameSite::None)
        .secure(true)
        .path("/")
        .max_age(cookie_life)
        .http_only(true)
        .build();
    jar = jar.add(csrf_cookie);

    let resp: ResponseOptions = expect_context();

    set_cookies(&resp, jar);

    Ok(auth_url.to_string())
}

async fn generate_oauth_login_code(
    code: String,
    pkce_verifier: PkceCodeVerifier,
    provider: SupportedOAuthProviders,
    query: AuthQuery,
    server_url: String,
) -> Result<String, AuthErrorKind> {
    log::info!("generate_oauth_login_code called for provider {}", provider);
    let ctx = expect_server_ctx();
    let oauth_impl = ctx
        .oauth_providers
        .get(&provider)
        .ok_or_else(|| AuthErrorKind::unexpected(format!("provider unavailable: {provider}")))?;
    let oauth2 = oauth_impl.get_client();

    let redirect_uri = RedirectUrl::new(format!("{server_url}/oauth_callback"))
        .map_err(|e| AuthErrorKind::unexpected(e.to_string()))?;

    log::info!(
        "Exchanging code with redirect URI: {}",
        redirect_uri.as_str()
    );
    let exchange = oauth2
        .exchange_code(AuthorizationCode::new(code))
        .map_err(AuthErrorKind::unexpected)?
        .set_redirect_uri(Cow::Owned(redirect_uri));

    #[cfg(feature = "apple-oauth")]
    let exchange = if provider != SupportedOAuthProviders::Apple {
        exchange.set_pkce_verifier(pkce_verifier)
    } else {
        // Apple doesn't support PKCE, so we skip setting the verifier
        exchange
    };
    #[cfg(not(feature = "apple-oauth"))]
    let exchange = exchange.set_pkce_verifier(pkce_verifier);

    let token_res = exchange
        .request_async(&ctx.oauth_http_client)
        .await
        .map_err(|e| {
            log::error!("Exchange code request failed: {}", e);
            AuthErrorKind::unexpected(e)
        })?;

    let id_token = token_res
        .extra_fields()
        .id_token()
        .ok_or_else(|| AuthErrorKind::unexpected("Provider did not return an ID token"))?;

    // we don't use a nonce
    let claims = oauth_impl.verify_id_token(&oauth2, id_token).map_err(|e| {
        log::error!("ID token verification failed: {}", e);
        e
    })?;
    let sub_id = claims.subject();
    let email = claims.email().map(|e| String::from(e.clone()));

    log::info!(
        "ID token verified for subject: {}, email: {:?}",
        sub_id.as_str(),
        email
    );

    let user_id =
        user_id_from_oauth_or_create(provider, &ctx.kv_store, sub_id.as_str(), email.as_deref())
            .await?;

    let server_url = get_server_url_from_request().await.map_err(|e| {
        log::error!("Failed to get server url for code_grant: {}", e);
        AuthErrorKind::Unexpected(e.to_string())
    })?;

    let code_grant = generate_code_grant_jwt(
        &ctx.jwk_pairs.auth_tokens.encoding_key,
        &user_id,
        &server_url,
        query,
        email,
    );

    Ok(code_grant)
}

pub async fn perform_oauth_login_impl(
    code: String,
    state: String,
) -> Result<String, ServerFnError> {
    let ctx = expect_server_ctx();
    let mut jar: PrivateCookieJar = extract_with_state(&ctx.cookie_key).await.map_err(|e| {
        log::error!("Failed to extract jar: {}", e);
        log::error!("OAuth error occurred: {e}");
        e
    })?;
    let server_url = get_server_url_from_request().await.map_err(|e| {
        log::error!("Failed to get server url: {}", e);
        log::error!("OAuth error occurred: {e}");
        e
    })?;

    let csrf_cookie = jar.get(CSRF_TOKEN_COOKIE).ok_or_else(|| {
        let err_msg = "csrf token not found";
        log::error!("{err_msg}");
        ServerFnError::new(err_msg)
    })?;
    if state != csrf_cookie.value() {
        let err_msg = "CSRF token mismatch";
        log::error!("{err_msg}");
        return Err(ServerFnError::new(err_msg));
    }

    let pkce_cookie = jar.get(PKCE_VERIFIER_COOKIE).ok_or_else(|| {
        let err_msg = "pkce verifier not found";
        log::error!("{err_msg}");
        ServerFnError::new(err_msg)
    })?;
    let pkce_verifier = PkceCodeVerifier::new(pkce_cookie.value().to_owned());

    jar = jar.remove(PKCE_VERIFIER_COOKIE);
    jar = jar.remove(CSRF_TOKEN_COOKIE);
    let resp: ResponseOptions = expect_context();
    set_cookies(&resp, jar);

    let state_raw = BASE64_URL_SAFE
        .decode(&state)
        .map_err(|_| ServerFnError::new("failed to decode state"))?;
    let state: OAuthState = postcard::from_bytes(&state_raw)
        .map_err(|_| ServerFnError::new("failed to deserialize state"))?;
    let query_raw = BASE64_URL_SAFE
        .decode(&state.client_state)
        .map_err(|_| ServerFnError::new("failed to decode client state"))?;

    let query: AuthQuery = postcard::from_bytes(&query_raw)
        .map_err(|_| ServerFnError::new("failed to deserialize query"))?;
    let req_state = query.state.clone();
    let mut redirect_uri = query.redirect_uri.clone();

    log::info!(
        "Successfully validated cookies, generating oauth login code for provider: {}",
        state.provider
    );

    let res =
        generate_oauth_login_code(code, pkce_verifier, state.provider, query, server_url).await;
    match res {
        Ok(grant) => redirect_uri
            .query_pairs_mut()
            .clear()
            .append_pair("code", &grant)
            .append_pair("state", &req_state),

        Err(e) => {
            let err_msg = e.to_string();
            log::error!("OAuth error occurred: {err_msg}");
            redirect_uri
                .query_pairs_mut()
                .clear()
                .append_pair("error", &e.to_string())
                .append_pair("state", &req_state)
        }
    };

    Ok(redirect_uri.to_string())
}

#[derive(Debug, Deserialize)]
pub struct AppleOAuthCallbackForm {
    code: Option<String>,
    state: Option<String>,
    user: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

pub async fn handle_apple_oauth_form_post(Form(form): Form<AppleOAuthCallbackForm>) -> Redirect {
    Redirect::to(&apple_oauth_callback_redirect_path(&form))
}

fn apple_oauth_callback_redirect_path(form: &AppleOAuthCallbackForm) -> String {
    if form.user.is_some() {
        log::debug!("Apple OAuth form post included user payload");
    }

    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    if let Some(error) = form.error.as_deref() {
        serializer.append_pair("error", error);
        if let Some(error_description) = form.error_description.as_deref() {
            serializer.append_pair("error_description", error_description);
        }
        if let Some(state) = form.state.as_deref() {
            serializer.append_pair("state", state);
        }
    } else if let (Some(code), Some(state)) = (form.code.as_deref(), form.state.as_deref()) {
        serializer
            .append_pair("code", code)
            .append_pair("state", state);
    } else {
        serializer
            .append_pair("error", "invalid_request")
            .append_pair("error_description", "Missing code or state");
        if let Some(state) = form.state.as_deref() {
            serializer.append_pair("state", state);
        }
    }

    format!("/oauth_callback?{}", serializer.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apple_oauth_callback_redirect_path_preserves_success_form_fields() {
        let form = AppleOAuthCallbackForm {
            code: Some("auth-code".to_string()),
            state: Some("client-state".to_string()),
            user: Some("{}".to_string()),
            error: None,
            error_description: None,
        };

        let redirect = apple_oauth_callback_redirect_path(&form);

        assert_eq!(
            "/oauth_callback?code=auth-code&state=client-state",
            redirect
        );
    }

    #[test]
    fn apple_oauth_callback_redirect_path_preserves_error_form_fields() {
        let form = AppleOAuthCallbackForm {
            code: None,
            state: Some("client-state".to_string()),
            user: None,
            error: Some("access_denied".to_string()),
            error_description: Some("User cancelled".to_string()),
        };

        let redirect = apple_oauth_callback_redirect_path(&form);

        assert_eq!(
            "/oauth_callback?error=access_denied&error_description=User+cancelled&state=client-state",
            redirect
        );
    }
}
