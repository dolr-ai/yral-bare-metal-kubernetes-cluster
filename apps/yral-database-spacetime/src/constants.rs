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
    // SpacetimeDB CLI publisher identity (spacetimedb-cli login)
    Identity::from_be_byte_array([
        0xc2, 0x00, 0x40, 0x07, 0xfc, 0xa1, 0x71, 0x43, 0x1b, 0xbd, 0x92, 0x0b, 0x82, 0x62, 0x2f,
        0xbb, 0xc0, 0x4b, 0xdd, 0x18, 0xa6, 0x9b, 0x6e, 0x0b, 0xee, 0x2a, 0xc4, 0x0e, 0xe0, 0x64,
        0xe1, 0xbb,
    ]),
    // TODO: add the off-chain-agent's SpacetimeDB identity here.
];
