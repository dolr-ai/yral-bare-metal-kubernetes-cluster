//! End-to-end integration test: JWT → SpacetimeDB identity.
//!
//! This test connects to a **running** local SpacetimeDB server (started via
//! `mise run dev-start` or `pitchfork start spacetimedb-dev`). It requires:
//!
//! - `SPACETIMEDB_URL` env var (e.g. `http://127.0.0.1:3000`)
//! - `SPACETIMEDB_DB_NAME` env var (e.g. `yral-database-spacetime-4lbo7`)
//! - `JWT_EC_PEM` env var (the ES256 private key — from fnox)
//! - The local SpacetimeDB must be started with `--jwt-pub-key-path` pointing
//!   to the matching public key (JWT_PUB_EC_PEM). The `spacetimedb-dev` pitchfork
//!   daemon does this automatically.
//!
//! Run with:
//!   mise run dev-start                       # start the dev servers
//!   fnox exec -- cargo test -p yral-auth --features ssr spacetime::integration_tests -- --test-threads=1
//!   mise run dev-stop                        # stop when done
//!
//! Or via the mise task:
//!   mise run test-spacetime-jwt

#![cfg(feature = "ssr")]
#![cfg(test)]

use std::sync::{Arc, Mutex};

use candid::Principal;
use spacetimedb_sdk::DbContext;
use yral_database_spacetime_bindings::DbConnection;

use crate::spacetime::spacetime_identity_for_principal;

/// Mint a yral-auth-style ES256 JWT with the given issuer + principal.
///
/// Uses the `JWT_EC_PEM` env var (the same key yral-auth uses to mint id_tokens).
/// The resulting JWT is a valid SpacetimeDB token — SpacetimeDB derives an
/// `Identity` from the `iss` + `sub` claims.
fn mint_jwt(issuer: &str, principal: &Principal) -> String {
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};

    let jwt_pem = std::env::var("JWT_EC_PEM")
        .expect("`JWT_EC_PEM` must be set (run via `fnox exec --` or set manually)");
    let encoding_key =
        EncodingKey::from_ec_pem(jwt_pem.as_bytes()).expect("invalid `JWT_EC_PEM` — not a valid EC PEM");

    let claims = serde_json::json!({
        "iss": issuer,
        "sub": principal.to_text(),
        "aud": "yral-client",
        "iat": 1700000000,
        "exp": 9999999999i64, // far future — won't expire during test
        "ext_spacetimedb_token": true,
    });

    let mut header = Header::new(Algorithm::ES256);
    header.kid = Some("default".to_string());

    encode(&header, &claims, &encoding_key).expect("failed to encode JWT")
}

/// Connect to SpacetimeDB with a JWT and return the identity the server assigns.
///
/// Uses `on_connect` callback to capture the identity, then runs the message
/// loop until the callback fires (or times out).
fn connect_and_get_identity(token: &str) -> spacetimedb_sdk::Identity {
    // Always connect to the local dev server — SPACETIMEDB_URL in mise.toml
    // points to Maincloud (prod), which won't verify our local JWT keys.
    let url = std::env::var("SPACETIMEDB_DEV_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:3000".to_string());
    let db_name = std::env::var("SPACETIMEDB_DB_NAME")
        .unwrap_or_else(|_| "yral-database-spacetime-4lbo7".to_string());

    let identity_result: Arc<Mutex<Option<spacetimedb_sdk::Identity>>> = Arc::new(Mutex::new(None));
    let identity_result_clone = identity_result.clone();

    let conn = DbConnection::builder()
        .with_uri(&url)
        .with_database_name(&db_name)
        .with_token(Some(token))
        .on_connect(move |_ctx, identity, _token| {
            *identity_result_clone.lock().unwrap() = Some(identity);
        })
        .build()
        .expect("failed to connect to SpacetimeDB — is the dev server running? (mise run dev-start)");

    // Run the message loop on a background thread until on_connect fires.
    let handle = conn.run_threaded();

    // Poll for the identity (up to 10 seconds).
    let timeout = std::time::Duration::from_secs(10);
    let start = std::time::Instant::now();
    let identity = loop {
        if let Some(id) = identity_result.lock().unwrap().take() {
            break id;
        }
        if start.elapsed() > timeout {
            panic!("timed out waiting for SpacetimeDB connection / identity assignment");
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    };

    // Disconnect cleanly.
    let _ = conn.disconnect();
    let _ = handle.join();

    identity
}

#[test]
fn test_jwt_produces_matching_spacetime_identity() {
    // The issuer must match what yral-auth uses in dev mode.
    // The yral-auth-dev daemon runs on localhost:8080, so the issuer
    // (server_url) is "http://localhost:8080" — yral-auth sets iss = server_url.
    let issuer = "http://localhost:8080";
    let principal = Principal::self_authenticating(&[1, 2, 3, 4, 5, 6]);

    // 1. Compute the expected identity using our derivation function.
    let expected_identity = spacetime_identity_for_principal(issuer, &principal);

    // 2. Mint a JWT with the same iss + sub claims.
    let jwt = mint_jwt(issuer, &principal);

    // 3. Connect to SpacetimeDB with the JWT and get the server-assigned identity.
    let actual_identity = connect_and_get_identity(&jwt);

    // 4. Verify they match.
    assert_eq!(
        expected_identity, actual_identity,
        "SpacetimeDB identity from JWT connection should match spacetime_identity_for_principal.\n\
         Expected: {}\n\
         Actual:   {}",
        expected_identity.to_hex(),
        actual_identity.to_hex()
    );
}

#[test]
fn test_different_principals_get_different_spacetime_identities() {
    let issuer = "http://localhost:8080";
    let principal_a = Principal::self_authenticating(&[1, 2, 3]);
    let principal_b = Principal::self_authenticating(&[4, 5, 6]);

    let jwt_a = mint_jwt(issuer, &principal_a);
    let jwt_b = mint_jwt(issuer, &principal_b);

    let id_a = connect_and_get_identity(&jwt_a);
    let id_b = connect_and_get_identity(&jwt_b);

    assert_ne!(
        id_a, id_b,
        "Different principals should get different SpacetimeDB identities"
    );
}

#[test]
fn test_identity_matches_for_anonymous_principal() {
    let issuer = "http://localhost:8080";
    let principal = Principal::anonymous();

    let expected = spacetime_identity_for_principal(issuer, &principal);
    let jwt = mint_jwt(issuer, &principal);
    let actual = connect_and_get_identity(&jwt);

    assert_eq!(
        expected, actual,
        "Anonymous principal identity should match.\nExpected: {}\nActual:   {}",
        expected.to_hex(),
        actual.to_hex()
    );
}