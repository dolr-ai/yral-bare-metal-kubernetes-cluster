pub mod redb_kv;
pub mod spacetime_kv;

use enum_dispatch::enum_dispatch;
use thiserror::Error;

/// Key prefix for yral-auth entries in the KV store.
pub const KEY_PREFIX: &str = "yral-auth";

/// Format a key with the given prefix.
pub fn format_to_dragonfly_key(key_prefix: &str, key: &str) -> String {
    format!("{}:{}", key_prefix, key)
}

#[derive(Error, Debug)]
pub enum KVError {
    #[error("deserialization err: {0}")]
    Deser(#[from] serde_json::Error),
    #[error(transparent)]
    ReDB(#[from] Box<redb::Error>),
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

#[enum_dispatch]
pub(crate) trait KVStore: Send {
    async fn read(&self, key: String) -> Result<Option<String>, KVError>;
    async fn write(&self, key: String, value: String) -> Result<(), KVError>;
    async fn has_key(&self, key: String) -> Result<bool, KVError>;
}

#[derive(Clone)]
#[enum_dispatch(KVStore)]
pub enum KVStoreImpl {
    ReDB(redb_kv::ReDBKV),
    Spacetime(spacetime_kv::SpacetimeKV),
}
