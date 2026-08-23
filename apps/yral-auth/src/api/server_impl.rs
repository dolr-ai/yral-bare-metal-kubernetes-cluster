//! Token grant handlers — thin wrappers over pure JWT logic.
//!
//! All JWT claim construction and validation is in pure functions
//! (`oauth/jwt/generate.rs`, `oauth/jwt/mod.rs`). This module handles
//! only the I/O: reading from the KV store, decoding/encoding JWTs,
//! and returning the token grant response.

use axum::{
    http::HeaderMap,
    response::{IntoResponse, Response},
    Extension, Form, Json,
};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use url::Url;
use web_time::Duration;

use crate::{
    api::ai_accounts::get_ai_account_ids_for_user,
    context::server::ServerCtx,
    kv::KVStore,
    oauth::{
        client_validation::{ClientIdValidator, OAuthClientType, ValidationRes},
        jwt::{
            generate::{generate_access_token_and_id_token_jwt, generate_refresh_token_jwt},
            AuthCodeClaims, RefreshTokenClaims,
        },
        AuthGrantQuery, PartialOIDCConfig, TokenGrantError, TokenGrantErrorKind, TokenGrantRes,
        TokenGrantResult,
    },
    utils::{
        server_url::{self, get_server_url_from_headers, get_server_url_from_request},
        user_id::generate_user_id,
    },
};

// ─────────────────────────────────────────────────────────────────────────
// Pure functions
// ─────────────────────────────────────────────────────────────────────────

/// Build the token grant response from claims data.
/// Pure — no I/O, just JWT encoding.
#[allow(clippy::too_many_arguments)]
fn build_token_grant(
    encoding_key: &jsonwebtoken::EncodingKey,
    user_id: &str,
    client_id: &str,
    nonce: Option<String>,
    is_anonymous: bool,
    access_max_age: Duration,
    refresh_max_age: Duration,
    email: Option<String>,
    ai_account_ids: Vec<String>,
    server_url: &str,
) -> TokenGrantRes {
    let (access_token, id_token) = generate_access_token_and_id_token_jwt(
        encoding_key,
        user_id,
        client_id,
        nonce.clone(),
        is_anonymous,
        access_max_age,
        email.clone(),
        ai_account_ids,
        server_url,
    );
    let refresh_token = generate_refresh_token_jwt(
        encoding_key,
        user_id,
        client_id,
        nonce,
        is_anonymous,
        refresh_max_age,
        email,
        server_url,
    );
    TokenGrantRes::new(access_token, id_token, refresh_token)
}

/// KV key for storing/retrieving a user's existence marker.
fn user_existence_key(user_id: &str) -> String {
    format!("user:{user_id}")
}

/// KV key for backend service user ID lookup.
fn backend_service_lookup_key(client_id: &str) -> String {
    format!("internal-login-{client_id}")
}

/// Verify PKCE code challenge against the verifier.
/// Pure — no I/O.
fn verify_pkce(code_verifier: &str, challenge: &[u8; 32]) -> bool {
    let mut hash = Sha256::new();
    hash.update(code_verifier.as_bytes());
    let hash: [u8; 32] = hash.finalize().into();
    hash == *challenge
}

// ─────────────────────────────────────────────────────────────────────────
// Thin wrappers (I/O only)
// ─────────────────────────────────────────────────────────────────────────

async fn verify_client_secret(
    ctx: &ServerCtx,
    client_id: &str,
    client_secret: Option<String>,
    redirect_uri: Option<&Url>,
) -> Result<ValidationRes, TokenGrantError> {
    ctx.validator
        .full_validation(
            &ctx.jwk_pairs.client_tokens.decoding_key,
            client_id,
            redirect_uri,
            client_secret.as_deref(),
        )
        .await
        .map_err(|e| TokenGrantError {
            error: TokenGrantErrorKind::InvalidClient,
            error_description: e.to_string(),
        })
}

impl IntoResponse for TokenGrantResult {
    fn into_response(self) -> Response {
        match self {
            Self::Ok(res) => Json(res).into_response(),
            Self::Err(e) => {
                let status_code = e.error.status_code();
                let mut res = Json(e).into_response();
                *res.status_mut() = status_code;
                res
            }
        }
    }
}

pub async fn handle_well_known_jwks(Extension(ctx): Extension<Arc<ServerCtx>>) -> Response {
    Json(ctx.jwk_pairs.well_known_jwks.clone()).into_response()
}

pub async fn handle_oidc_configuration(headers: HeaderMap) -> Response {
    let server_url = server_url::get_server_url_from_headers(&headers);
    let jwks_uri = format!("{}/.well-known/jwks.json", server_url);
    Json(PartialOIDCConfig { jwks_uri }).into_response()
}

pub async fn healthz() -> Response {
    Json(serde_json::json!({"status": "ok"})).into_response()
}

pub async fn handle_oauth_token_grant(
    Extension(ctx): Extension<Arc<ServerCtx>>,
    headers: HeaderMap,
    Form(req): Form<AuthGrantQuery>,
) -> Response {
    let server_url = get_server_url_from_headers(&headers);
    let res = match req {
        AuthGrantQuery::AuthorizationCode {
            code,
            redirect_uri,
            code_verifier,
            client_id,
            client_secret,
        } => {
            handle_authorization_code_grant(
                &ctx,
                code,
                redirect_uri,
                code_verifier,
                client_id,
                client_secret,
                &server_url,
            )
            .await
        }
        AuthGrantQuery::RefreshToken {
            refresh_token,
            client_id,
            client_secret,
        } => {
            handle_refresh_token_grant(&ctx, refresh_token, client_id, client_secret, &server_url)
                .await
        }
        AuthGrantQuery::ClientCredentials {
            client_id,
            client_secret,
        } => handle_client_credentials_grant(&ctx, client_id, client_secret, &server_url).await,
    };

    match res {
        Ok(grant) => Json(grant).into_response(),
        Err(e) => {
            let status_code = e.error.status_code();
            let mut res = Json(e).into_response();
            *res.status_mut() = status_code;
            res
        }
    }
}

/// Generate access/ID/refresh tokens for a known user ID.
/// Thin wrapper — reads AI account IDs from KV, delegates to pure `build_token_grant`.
#[allow(clippy::too_many_arguments)]
async fn generate_access_token(
    ctx: &ServerCtx,
    user_id: &str,
    client_id: &str,
    nonce: Option<String>,
    is_anonymous: bool,
    validation_res: ValidationRes,
    email: Option<String>,
    server_url: &str,
) -> Result<TokenGrantRes, TokenGrantError> {
    let exists = ctx
        .kv_store
        .has_key(user_existence_key(user_id))
        .await
        .map_err(|e| TokenGrantError {
            error: TokenGrantErrorKind::ServerError,
            error_description: e.to_string(),
        })?;

    if !exists {
        return Err(TokenGrantError {
            error: TokenGrantErrorKind::ServerError,
            error_description: format!("unknown user {user_id}"),
        });
    }

    let ai_account_ids = get_ai_account_ids_for_user(ctx, user_id)
        .await
        .map_err(|e| TokenGrantError {
            error: TokenGrantErrorKind::ServerError,
            error_description: format!("Failed to fetch AI accounts: {}", e),
        })?;

    Ok(build_token_grant(
        &ctx.jwk_pairs.auth_tokens.encoding_key,
        user_id,
        client_id,
        nonce,
        is_anonymous,
        validation_res.access_max_age,
        validation_res.refresh_max_age,
        email,
        ai_account_ids,
        server_url,
    ))
}

async fn handle_authorization_code_grant(
    ctx: &ServerCtx,
    code: String,
    redirect_uri: Url,
    code_verifier: String,
    client_id: String,
    client_secret: Option<String>,
    server_url: &str,
) -> Result<TokenGrantRes, TokenGrantError> {
    let validation_res =
        verify_client_secret(ctx, &client_id, client_secret, Some(&redirect_uri)).await?;

    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::ES256);
    validation.set_audience(&[&client_id]);
    validation.set_issuer(&[server_url]);

    let auth_code = jsonwebtoken::decode::<AuthCodeClaims>(
        &code,
        &ctx.jwk_pairs.auth_tokens.decoding_key,
        &validation,
    )
    .map_err(|e| TokenGrantError {
        error: TokenGrantErrorKind::InvalidGrant,
        error_description: e.to_string(),
    })?;

    let code_claims = auth_code.claims;

    if code_claims.ext_redirect_uri != redirect_uri {
        return Err(TokenGrantError {
            error: TokenGrantErrorKind::InvalidGrant,
            error_description: "Invalid redirect URI".to_string(),
        });
    }

    if !verify_pkce(&code_verifier, &code_claims.ext_code_challenge_s256.0) {
        return Err(TokenGrantError {
            error: TokenGrantErrorKind::InvalidGrant,
            error_description: "Invalid code verifier".to_string(),
        });
    }

    generate_access_token(
        ctx,
        &code_claims.sub,
        &client_id,
        code_claims.nonce.clone(),
        false,
        validation_res,
        code_claims.ext_email,
        server_url,
    )
    .await
}

async fn handle_refresh_token_grant(
    ctx: &ServerCtx,
    refresh_token: String,
    client_id: String,
    client_secret: Option<String>,
    server_url: &str,
) -> Result<TokenGrantRes, TokenGrantError> {
    let validation_res = verify_client_secret(ctx, &client_id, client_secret, None).await?;

    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::ES256);
    validation.set_audience(&[&client_id]);
    validation.set_issuer(&[server_url, "https://auth.yral.com"]);

    let refresh_token = jsonwebtoken::decode::<RefreshTokenClaims>(
        &refresh_token,
        &ctx.jwk_pairs.auth_tokens.decoding_key,
        &validation,
    )
    .map_err(|e| TokenGrantError {
        error: TokenGrantErrorKind::InvalidGrant,
        error_description: e.to_string(),
    })?;

    let refresh_claims = refresh_token.claims;

    generate_access_token(
        ctx,
        &refresh_claims.sub,
        &client_id,
        None,
        refresh_claims.ext_is_anonymous,
        validation_res,
        refresh_claims.ext_email,
        server_url,
    )
    .await
}

async fn client_credentials_grant_for_backend(
    ctx: &ServerCtx,
    client_id: String,
    res: ValidationRes,
) -> Result<TokenGrantRes, TokenGrantError> {
    let server_url = match get_server_url_from_request().await {
        Ok(url) => url,
        Err(e) => {
            return Err(TokenGrantError {
                error: TokenGrantErrorKind::ServerError,
                error_description: e.to_string(),
            });
        }
    };

    let lookup_key = backend_service_lookup_key(&client_id);

    let existing_user_id = ctx
        .kv_store
        .read(lookup_key.clone())
        .await
        .map_err(|e| TokenGrantError {
            error: TokenGrantErrorKind::ServerError,
            error_description: e.to_string(),
        })?;

    if let Some(user_id) = existing_user_id {
        return generate_access_token(
            ctx,
            &user_id,
            &client_id,
            None,
            false,
            res,
            None,
            &server_url,
        )
        .await;
    }

    let new_user_id = generate_user_id();

    ctx.kv_store
        .write(lookup_key, new_user_id.clone())
        .await
        .map_err(|e| TokenGrantError {
            error: TokenGrantErrorKind::ServerError,
            error_description: e.to_string(),
        })?;

    ctx.kv_store
        .write(user_existence_key(&new_user_id), "1".to_string())
        .await
        .map_err(|e| TokenGrantError {
            error: TokenGrantErrorKind::ServerError,
            error_description: e.to_string(),
        })?;

    Ok(build_token_grant(
        &ctx.jwk_pairs.auth_tokens.encoding_key,
        &new_user_id,
        &client_id,
        None,
        false,
        res.access_max_age,
        res.refresh_max_age,
        None,
        Vec::new(),
        &server_url,
    ))
}

async fn handle_client_credentials_grant(
    ctx: &ServerCtx,
    client_id: String,
    client_secret: Option<String>,
    server_url: &str,
) -> Result<TokenGrantRes, TokenGrantError> {
    let validation_res = verify_client_secret(ctx, &client_id, client_secret, None).await?;
    if validation_res.kind == OAuthClientType::BackendService {
        return client_credentials_grant_for_backend(ctx, client_id, validation_res).await;
    }

    let new_user_id = generate_user_id();

    ctx.kv_store
        .write(user_existence_key(&new_user_id), "1".to_string())
        .await
        .map_err(|e| TokenGrantError {
            error: TokenGrantErrorKind::ServerError,
            error_description: e.to_string(),
        })?;

    Ok(build_token_grant(
        &ctx.jwk_pairs.auth_tokens.encoding_key,
        &new_user_id,
        &client_id,
        None,
        true,
        validation_res.access_max_age,
        validation_res.refresh_max_age,
        None,
        Vec::new(),
        server_url,
    ))
}
