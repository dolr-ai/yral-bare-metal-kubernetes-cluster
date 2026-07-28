//! Auth key-value store — migrated from Redis/Dragonfly.
//!
//! yral-auth stores 5 families of key-value data (user identity keys,
//! OAuth mappings, AI account keys, AI account reverse lookups, backend
//! service identities). Previously in Dragonfly/Redis; now in SpacetimeDB.
//!
//! This is a simple String→String KV store. The table is private (not public)
//! — only yral-auth (via the admin identity) reads and writes it. All
//! reducers check `ADMINS`. Procedures are not used (yral-auth calls reducers
//! via REST for writes, and a procedure for reads).
//!
//! ## Usage
//! yral-auth calls these via the SpacetimeDB REST API:
//! - `POST /v1/database/{db}/call/kv_set` — write a key-value pair
//! - `POST /v1/database/{db}/call/kv_get` — read a value by key (procedure)
//! - `POST /v1/database/{db}/call/kv_delete` — delete a key
//!
//! Key existence checks use `kv_get` and check for `None`. No separate
//! `kv_has` procedure is needed.
//!
//! All calls are authenticated with yral-auth's admin JWT (the yral-auth
//! `id_token` passed as the SpacetimeDB token).

use spacetimedb::{Identity, ReducerContext, SpacetimeType, Table};

/// A simple key-value entry for auth data.
/// Key is the logical key (e.g. `<principal>`, `{provider}-login-{sub_id}`).
/// Value is the stored string (JWK, principal text, etc.).
#[spacetimedb::table(accessor = auth_kv, private)]
pub struct AuthKvEntry {
    #[primary_key]
    pub key: String,
    pub value: String,
}

/// Result of `kv_get` — `None` if the key doesn't exist.
#[derive(SpacetimeType, Clone, Debug)]
pub struct KvGetResult {
    pub value: Option<String>,
}

/// Set a key-value pair. Admin-only (yral-auth is the only writer).
#[spacetimedb::reducer]
pub fn kv_set(ctx: &ReducerContext, key: String, value: String) -> Result<(), String> {
    if !crate::constants::ADMINS.contains(&ctx.sender()) {
        return Err("Unauthorized".to_string());
    }
    // Upsert: delete if exists, then insert.
    ctx.db.auth_kv().key().delete(key.clone());
    ctx.db.auth_kv().insert(AuthKvEntry { key, value });
    Ok(())
}

/// Delete a key. Admin-only.
#[spacetimedb::reducer]
pub fn kv_delete(ctx: &ReducerContext, key: String) -> Result<(), String> {
    if !crate::constants::ADMINS.contains(&ctx.sender()) {
        return Err("Unauthorized".to_string());
    }
    ctx.db.auth_kv().key().delete(key);
    Ok(())
}

/// Get a value by key. Returns `KvGetResult { value: Option<String> }`.
/// Called via REST by yral-auth.
#[spacetimedb::procedure]
pub fn kv_get(ctx: &mut spacetimedb::ProcedureContext, key: String) -> KvGetResult {
    ctx.with_tx(|tx| {
        let value = tx
            .db
            .auth_kv()
            .key()
            .find(key.clone())
            .map(|entry| entry.value);
        KvGetResult { value }
    })
}