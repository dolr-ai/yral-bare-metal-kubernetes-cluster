//! SpacetimeDB identity integration for yral-auth.
//!
//! ## How it works
//!
//! SpacetimeDB derives an `Identity` from the `iss` (issuer) and `sub`
//! (subject) claims of any OpenID Connect compliant JWT passed to
//! `DbConnection::builder().with_token(jwt)`. The derivation is
//! deterministic: the same `iss` + `sub` always produces the same
//! `Identity` via `Identity::from_claims(issuer, subject)`.
//!
//! yral-auth already mints ES256 JWTs (`id_token`) with:
//! - `iss` = the yral-auth server URL (e.g. `https://auth.yral.com`)
//! - `sub` = the user's IC Principal (as text)
//!
//! Therefore, the existing yral-auth `id_token` IS the SpacetimeDB token.
//! No separate token minting, no KV storage, no Dragonfly/Redis needed.
//! The client passes the yral-auth `id_token` to `with_token()` and
//! SpacetimeDB derives a stable identity from the claims.
//!
//! ## No vendor lock-in
//!
//! User auth details (Google/Apple/WhatsApp OAuth, IC identity keys) live
//! in yral-auth — fully under your control. The JWT is signed by yral-auth's
//! own ES256 key. If you migrate from Maincloud to self-hosted SpacetimeDB,
//! the same JWTs still work — just point clients at the new server URL.
//!
//! ## What this module provides
//!
//! - `spacetime_identity_for_principal`: compute the SpacetimeDB `Identity`
//!   for a given IC principal + yral-auth issuer URL. Used by the backfill
//!   test (to map IC principal → SpacetimeDB identity) and the `ADMINS`
//!   constant in the SpacetimeDB module.
//! - `EXT_SPACETIMEDB_TOKEN_CLAIM`: the JWT claim key that signals to clients
//!   that the `id_token` can be used as a SpacetimeDB token.

use candid::Principal;
use spacetimedb_sdk::Identity;

/// The JWT claim key that tells clients "this id_token is also a valid
/// SpacetimeDB token — pass it to `DbConnection::builder().with_token()`".
///
/// The value is a boolean `true` (the token itself is the `id_token` — no
/// separate token field needed). Clients check for this claim's presence
/// to know they should use the `id_token` for SpacetimeDB auth.
pub const EXT_SPACETIMEDB_TOKEN_CLAIM: &str = "ext_spacetimedb_token";

/// Compute the SpacetimeDB `Identity` that SpacetimeDB will derive for a
/// given IC principal + yral-auth issuer URL.
///
/// This mirrors SpacetimeDB's `Identity::from_claims(issuer, subject)`
/// which hashes the `iss` + `sub` claims to produce a deterministic
/// identity. The same principal + issuer always produces the same identity.
///
/// Used by:
/// - The backfill test (to map IC principal → SpacetimeDB identity)
/// - Logging/debugging in yral-auth
/// - The `ADMINS` constant in the SpacetimeDB module (to compute admin
///   identities from their IC principals)
pub fn spacetime_identity_for_principal(issuer: &str, principal: &Principal) -> Identity {
    Identity::from_claims(issuer, &principal.to_text())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
#[cfg(feature = "ssr")]
mod integration_tests;