//! Tests for the SpacetimeDB identity derivation from yral-auth JWTs.
//!
//! These tests verify that:
//! 1. `Identity::from_claims(iss, sub)` produces a deterministic identity
//!    from the `iss` + `sub` claims of a yral-auth JWT
//! 2. `spacetime_identity_for_principal` matches what SpacetimeDB will derive
//! 3. A JWT minted by yral-auth, when decoded, produces claims that yield
//!    the same identity when passed to `Identity::from_claims`

use candid::Principal;
use spacetimedb_sdk::Identity;

use crate::spacetime::spacetime_identity_for_principal;

#[test]
fn test_identity_derivation_is_deterministic() {
    let issuer = "https://auth.yral.com";
    let principal = Principal::anonymous();

    let id1 = Identity::from_claims(issuer, &principal.to_text());
    let id2 = Identity::from_claims(issuer, &principal.to_text());

    assert_eq!(
        id1, id2,
        "Identity::from_claims should be deterministic for the same iss + sub"
    );
}

#[test]
fn test_spacetime_identity_for_principal_matches_from_claims() {
    let issuer = "https://auth.yral.com";
    let principal = Principal::anonymous();

    let expected = Identity::from_claims(issuer, &principal.to_text());
    let actual = spacetime_identity_for_principal(issuer, &principal);

    assert_eq!(
        expected, actual,
        "spacetime_identity_for_principal should match Identity::from_claims"
    );
}

#[test]
fn test_different_principals_produce_different_identities() {
    let issuer = "https://auth.yral.com";
    // Use self_authenticating principals — anonymous + "2vxsx-fae" resolve
    // to the same thing (anonymous). Self-authenticating principals have
    // a derivable public key prefix that makes them distinct.
    let principal_a = Principal::self_authenticating(&[1, 2, 3]);
    let principal_b = Principal::self_authenticating(&[4, 5, 6]);

    let id_a = spacetime_identity_for_principal(issuer, &principal_a);
    let id_b = spacetime_identity_for_principal(issuer, &principal_b);

    assert_ne!(
        id_a, id_b,
        "Different principals should produce different SpacetimeDB identities"
    );
}

#[test]
fn test_different_issuers_produce_different_identities() {
    let principal = Principal::anonymous();
    let issuer_a = "https://auth.yral.com";
    let issuer_b = "http://localhost:8080";

    let id_a = spacetime_identity_for_principal(issuer_a, &principal);
    let id_b = spacetime_identity_for_principal(issuer_b, &principal);

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

    use jsonwebtoken::{encode, EncodingKey, Header, Algorithm, decode, DecodingKey, Validation};

    let issuer = "http://localhost:8080";
    let principal = Principal::self_authenticating(&[1, 2, 3]);
    let principal_text = principal.to_text();

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
        "sub": principal_text,
        "aud": "test-client",
        "iat": 1700000000,
        "exp": 1800000000,
        "ext_spacetimedb_token": true,
    });

    let header = Header::new(Algorithm::ES256);
    let token = encode(&header, &claims, &encoding_key).expect("failed to encode JWT");

    // Decode the JWT (without verifying signature — we only care about claims)
    let mut validation = Validation::new(Algorithm::ES256);
    validation.validate_exp = false;
    validation.validate_aud = false;
    validation.insecure_disable_signature_validation();

    let token_data = decode::<serde_json::Value>(&token, &DecodingKey::from_secret(&[]), &validation)
        .expect("failed to decode JWT");

    let decoded_iss = token_data.claims["iss"].as_str().expect("missing iss");
    let decoded_sub = token_data.claims["sub"].as_str().expect("missing sub");

    assert_eq!(decoded_iss, issuer);
    assert_eq!(decoded_sub, principal_text);

    // Verify the identity matches
    let expected_identity = spacetime_identity_for_principal(issuer, &principal);
    let actual_identity = Identity::from_claims(decoded_iss, decoded_sub);

    assert_eq!(
        expected_identity, actual_identity,
        "Identity derived from decoded JWT claims should match spacetime_identity_for_principal"
    );
}