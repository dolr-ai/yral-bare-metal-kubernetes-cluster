use serde::Deserialize;

use super::{KVError, KVStore};

/// SpacetimeDB-backed KV store implementation.
///
/// Talks to the SpacetimeDB module via its REST API, calling `kv_get`, `kv_set`,
/// and `kv_delete` procedures/reducers. The SpacetimeDB KV table uses the
/// raw logical key as the primary key — no `yral-auth:` prefix is applied.
#[derive(Clone)]
pub struct SpacetimeKV {
    client: reqwest::Client,
    url: String,
    db_name: String,
    token: String,
}

/// SpacetimeDB REST API wraps procedure results in an outer array.
/// `kv_get` returns `KvGetResult { value: Option<String> }`, which the REST
/// API serializes as `[[<variant_index>, <payload>]]`:
/// - `Some(value)` → `[[0, "the-value"]]`
/// - `None`        → `[[1, []]]`
///
/// We deserialize the outer wrapper and then extract the `Option<String>`.
/// The outer array contains a single element which is the `KvGetResult`
/// encoded as a 2-element array: `[variant_index, payload]`.
#[derive(Deserialize)]
#[serde(untagged)]
enum KvGetRestResponse {
    /// `Some(value)` case: `[0, "value"]`
    Some((u8, String)),
    /// `None` case: `[1, []]` — the payload is an empty array
    None((u8, serde_json::Value)),
}

impl SpacetimeKV {
    /// Build a `SpacetimeKV` from the following env vars:
    /// - `SPACETIMEDB_URL`         — e.g. `https://maincloud.spacetimedb.com` or `http://127.0.0.1:3000`
    /// - `SPACETIMEDB_DB_NAME`      — e.g. `yral-database-spacetime-4lbo7`
    /// - `SPACETIMEDB_ADMIN_TOKEN`  — the yral-auth JWT used as the SpacetimeDB auth token
    pub fn new() -> Result<Self, anyhow::Error> {
        Self::from_env(
            std::env::var("SPACETIMEDB_URL")?,
            std::env::var("SPACETIMEDB_DB_NAME")?,
            std::env::var("SPACETIMEDB_ADMIN_TOKEN")?,
        )
    }

    pub fn from_env(
        url: String,
        db_name: String,
        token: String,
    ) -> Result<Self, anyhow::Error> {
        Ok(Self {
            client: reqwest::Client::new(),
            url,
            db_name,
            token,
        })
    }

    /// Build the call URL for a given procedure/reducer name.
    fn call_url(&self, name: &str) -> String {
        format!(
            "{}/v1/database/{}/call/{}",
            self.url.trim_end_matches('/'),
            self.db_name,
            name
        )
    }

    /// POST a JSON array of positional arguments to a procedure and return the
    /// parsed response.
    async fn call_procedure<T: serde::de::DeserializeOwned>(
        &self,
        name: &str,
        args: serde_json::Value,
    ) -> Result<T, KVError> {
        let resp = self
            .client
            .post(self.call_url(name))
            .bearer_auth(&self.token)
            .json(&args)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("SpacetimeDB {name} request failed: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "SpacetimeDB {name} returned {status}: {body}"
            )
            .into());
        }

        let parsed: T = resp
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("SpacetimeDB {name} response parse failed: {e}"))?;

        Ok(parsed)
    }

    /// POST a JSON array of positional arguments to a reducer. Reducers return
    /// no meaningful body, so we only check for HTTP success.
    async fn call_reducer(
        &self,
        name: &str,
        args: serde_json::Value,
    ) -> Result<(), KVError> {
        let resp = self
            .client
            .post(self.call_url(name))
            .bearer_auth(&self.token)
            .json(&args)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("SpacetimeDB {name} request failed: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "SpacetimeDB {name} returned {status}: {body}"
            )
            .into());
        }

        Ok(())
    }
}

impl KVStore for SpacetimeKV {
    async fn read(&self, key: String) -> Result<Option<String>, KVError> {
        let resp: Vec<KvGetRestResponse> =
            self.call_procedure("kv_get", serde_json::json!([key])).await?;
        // The REST API wraps the result in an outer array. We expect exactly
        // one element. Extract the `Option<String>` from the variant.
        let result = resp
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("kv_get returned empty array"))?;
        Ok(match result {
            KvGetRestResponse::Some((_, value)) => Some(value),
            KvGetRestResponse::None(_) => None,
        })
    }

    async fn write(&self, key: String, value: String) -> Result<(), KVError> {
        self.call_reducer("kv_set", serde_json::json!([key, value]))
            .await
    }

    async fn has_key(&self, key: String) -> Result<bool, KVError> {
        let resp: Vec<KvGetRestResponse> =
            self.call_procedure("kv_get", serde_json::json!([key])).await?;
        let result = resp
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("kv_get returned empty array"))?;
        Ok(match result {
            KvGetRestResponse::Some(_) => true,
            KvGetRestResponse::None(_) => false,
        })
    }
}