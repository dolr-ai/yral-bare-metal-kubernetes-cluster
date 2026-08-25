use std::env;
use std::sync::OnceLock;

use axum::http::HeaderMap;
use jsonwebtoken::{Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

/// JWT access-token claims issued by yral-auth.
/// The `sub` field is the user_id (OAuth sub or UUID for AI accounts).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessTokenClaims {
    pub aud: String,
    pub exp: usize,
    pub iat: usize,
    pub iss: String,
    pub sub: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
    pub ext_is_anonymous: bool,
}

/// Decoding key loaded once from `JWT_PUB_EC_PEM` env var.
/// This is the EC P256 public key corresponding to yral-auth's `JWT_EC_PEM`.
static DECODING_KEY: OnceLock<DecodingKey> = OnceLock::new();

fn decoding_key() -> Result<&'static DecodingKey, anyhow::Error> {
    if let Some(key) = DECODING_KEY.get() {
        return Ok(key);
    }

    let pub_pem = env::var("JWT_PUB_EC_PEM").map_err(|_| {
        anyhow::anyhow!("JWT_PUB_EC_PEM is not set — cannot verify access tokens")
    })?;

    let key = DecodingKey::from_ec_pem(pub_pem.as_bytes()).map_err(|e| {
        anyhow::anyhow!("Invalid JWT_PUB_EC_PEM — not a valid EC public key: {e}")
    })?;

    // get_or_init is race-safe; the first writer wins.
    Ok(DECODING_KEY.get_or_init(|| key))
}

/// Verify a JWT access token and return the user_id (`sub` claim).
///
/// The token is issued by yral-auth (ES256, EC P256). The public key is
/// loaded from `JWT_PUB_EC_PEM`. Validation checks `exp` and algorithm.
pub fn verify_access_token(jwt: &str) -> Result<AccessTokenClaims, anyhow::Error> {
    let key = decoding_key()?;

    let mut validation = Validation::default();
    validation.algorithms = vec![Algorithm::ES256];
    // We don't pin `aud` here — multiple clients may use different audiences.
    // The `exp` check is still enforced by jsonwebtoken.
    validation.validate_aud = false;

    let token_data = jsonwebtoken::decode::<AccessTokenClaims>(jwt, key, &validation)?;
    Ok(token_data.claims)
}

/// Extract and verify a Bearer JWT from the `Authorization` header.
/// Returns the user_id (`sub` claim) on success.
pub fn extract_user_id_from_headers(
    headers: &HeaderMap,
) -> Result<String, (String, u16)> {
    let jwt = headers
        .get("Authorization")
        .ok_or_else(|| ("missing Authorization header".to_string(), 401))?;

    let jwt = jwt
        .to_str()
        .map_err(|_| ("invalid Authorization header".to_string(), 401))?;

    if !jwt.starts_with("Bearer ") {
        return Err(("invalid Authorization header — expected Bearer token".to_string(), 401));
    }

    let jwt = &jwt[7..];
    let claims = verify_access_token(jwt).map_err(|e| {
        (format!("invalid JWT: {e}"), 401)
    })?;

    Ok(claims.sub)
}
