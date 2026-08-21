//! Tests for the SpacetimeDB identity derivation from yral-auth JWTs.
//!
//! These tests verify that:
//! 1. `Identity::from_claims(iss, sub)` produces a deterministic identity
//!    from the `iss` + `sub` claims of a yral-auth JWT
//! 2. `spacetime_identity_for_principal` matches what SpacetimeDB will derive
//! 3. A JWT minted by yral-auth, when decoded, produces claims that yield
//!    the same identity when passed to `Identity::from_claims`

use spacetimedb_sdk::Identity;

use crate::spacetime::spacetime_identity_for_user_id;

#[test]
fn test_identity_derivation_is_deterministic() {
    let issuer = "https://auth.yral.com";
    let user_id = "test-user-1";

    let id1 = Identity::from_claims(issuer, user_id);
    let id2 = Identity::from_claims(issuer, user_id);

    assert_eq!(
        id1, id2,
        "Identity::from_claims should be deterministic for the same iss + sub"
    );
}

#[test]
fn test_spacetime_identity_for_user_id_matches_from_claims() {
    let issuer = "https://auth.yral.com";
    let user_id = "test-user-1";

    let expected = Identity::from_claims(issuer, user_id);
    let actual = spacetime_identity_for_user_id(issuer, user_id);

    assert_eq!(
        expected, actual,
        "spacetime_identity_for_user_id should match Identity::from_claims"
    );
}

#[test]
fn test_different_user_ids_produce_different_identities() {
    let issuer = "https://auth.yral.com";
    let user_id_a = "test-user-1";
    let user_id_b = "test-user-2";

    let id_a = spacetime_identity_for_user_id(issuer, user_id_a);
    let id_b = spacetime_identity_for_user_id(issuer, user_id_b);

    assert_ne!(
        id_a, id_b,
        "Different user IDs should produce different SpacetimeDB identities"
    );
}

#[test]
fn test_different_issuers_produce_different_identities() {
    let user_id = "test-user-1";
    let issuer_a = "https://auth.yral.com";
    let issuer_b = "http://localhost:8080";

    let id_a = spacetime_identity_for_user_id(issuer_a, user_id);
    let id_b = spacetime_identity_for_user_id(issuer_b, user_id);

    assert_ne!(
        id_a, id_b,
        "Different issuers should produce different SpacetimeDB identities"
    );
}

#[test]
fn test_jwt_minted_by_yral_auth_produces_correct_identity() {
    // This test verifies the full flow:
    // 1. Mint a JWT with yral-auth's claims (iss, sub)
    // 2. Decode it
    // 3. Verify Identity::from_claims matches spacetime_identity_for_principal
    //
    // We use a minimal JWT with just the claims SpacetimeDB cares about.
    // In production, yral-auth's generate_access_token_and_id_token_jwt
    // adds ext_delegated_identity, email, etc. — but SpacetimeDB only
    // reads iss + sub for identity derivation.

    use jsonwebtoken::{encode, decode, EncodingKey, DecodingKey, Header, Algorithm, Validation};

    let issuer = "http://localhost:8080";
    let user_id = "test-user-jwt-1";

    // Generate a test ES256 (P-256/secp256r1) key pair.
    // jsonwebtoken's ES256 requires P-256, NOT secp256k1 (k256).
    use p256::pkcs8::{EncodePrivateKey, LineEnding};
    let secret = p256::SecretKey::random(&mut rand::rngs::OsRng);
    let pkcs8 = secret.to_pkcs8_der().expect("failed to encode key as PKCS8");
    let pem = pkcs8.to_pem("PRIVATE KEY", LineEnding::LF).expect("failed to create PEM");
    let encoding_key = EncodingKey::from_ec_pem(pem.as_bytes())
        .expect("failed to create encoding key from test EC key");

    let claims = serde_json::json!({
        "iss": issuer,
        "sub": user_id,
        "aud": "test-client",
        "iat": 1700000000,
        "exp": 1800000000,
        "ext_spacetimedb_token": true,
    });

    let header = Header::new(Algorithm::ES256);
    let token = encode(&header, &claims, &encoding_key).expect("failed to encode JWT");

    // Decode the JWT with full signature verification — the prescribed mechanism.
    // We own the key pair, so there is no reason to skip verification.
    // DecodingKey::from_ec_pem expects a *public* key PEM; derive the decoding
    // key from the secret's public key coordinates (base64 x, y) instead —
    // the same approach used in production (context/server.rs).
    use base64::Engine;
    let public_key = secret.public_key();
    let affine = public_key.to_sec1_bytes();
    // SEC1 point = 0x04 || x[32] || y[32] for uncompressed P-256
    let x = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(&affine[1..33]);
    let y = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(&affine[33..65]);
    let decoding_key = DecodingKey::from_ec_components(&x, &y)
        .expect("failed to create decoding key from test EC public components");

    let mut validation = Validation::new(Algorithm::ES256);
    validation.validate_aud = false; // test token has no configured audience to check against
    validation.validate_exp = false; // fixed exp timestamp; avoids clock-skew flakes in CI

    let token_data = decode::<serde_json::Value>(&token, &decoding_key, &validation)
        .expect("failed to decode JWT");

    let decoded_iss = token_data.claims["iss"].as_str().expect("missing iss");
    let decoded_sub = token_data.claims["sub"].as_str().expect("missing sub");

    assert_eq!(decoded_iss, issuer);
    assert_eq!(decoded_sub, user_id);

    // Verify the identity matches
    let expected_identity = spacetime_identity_for_user_id(issuer, user_id);
    let actual_identity = Identity::from_claims(decoded_iss, decoded_sub);

    assert_eq!(
        expected_identity, actual_identity,
        "Identity derived from decoded JWT claims should match spacetime_identity_for_user_id"
    );
}