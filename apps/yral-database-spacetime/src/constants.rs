//! Shared constants for all SpacetimeDB module features.

use spacetimedb::Identity;

/// Hardcoded admin identities. Shared across all modules (posts, auth_kv).
/// Add an identity by hex-decoding its 32-byte big-endian representation.
/// To get an identity's hex string, run `spacetime publish` and note the
/// publisher identity, or connect a service (off-chain-agent, backfill binary)
/// and log its `Identity::to_hex()`.
///
/// Example:
/// ```ignore
/// Identity::from_be_byte_array([
///     0xc2, 0x00, 0x..., // 32 bytes from the hex string (big-endian)
/// ])
/// ```
pub const ADMINS: &[Identity] = &[
    // TODO: add the module publisher identity here after first publish.
    // TODO: add the off-chain-agent's SpacetimeDB identity here.
    // TODO: add the backfill binary's SpacetimeDB identity here.
    // TODO: add the external Prakash/Naitik service identity here.
];