use crate::auth::init_jwt;
use crate::auth::JwtDetails;
use crate::config::AppConfig;
use crate::utils::error::{Error, Result};
use crate::utils::yral_auth_jwt::YralAuthJwt;
use reqwest::Client;
use std::sync::Arc;

/// SpacetimeDB REST client for metadata reads/writes.
/// Replaces the Redis/Dragonfly connection pool.
#[derive(Clone)]
pub struct SpacetimeClient {
    http: Client,
    url: String,
    db_name: String,
    token: String,
}

impl SpacetimeClient {
    pub fn new(url: String, db_name: String, token: String) -> Self {
        Self {
            http: Client::new(),
            url,
            db_name,
            token,
        }
    }

    /// Build the call URL for a procedure/reducer.
    fn call_url(&self, name: &str) -> String {
        format!(
            "{}/v1/database/{}/call/{}",
            self.url.trim_end_matches('/'),
            self.db_name,
            name
        )
    }

    /// Call a SpacetimeDB procedure (read) and return the raw JSON response.
    pub async fn call_procedure<T: serde::de::DeserializeOwned>(
        &self,
        name: &str,
        args: serde_json::Value,
    ) -> Result<T> {
        let resp = self
            .http
            .post(self.call_url(name))
            .bearer_auth(&self.token)
            .json(&args)
            .send()
            .await
            .map_err(|e| Error::Unknown(format!("SpacetimeDB {name} request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(Error::Unknown(format!(
                "SpacetimeDB {name} returned {status}: {body}"
            )));
        }

        resp.json()
            .await
            .map_err(|e| Error::Unknown(format!("SpacetimeDB {name} parse failed: {e}")))
    }

    /// Call a SpacetimeDB reducer (write). Returns OK on success.
    pub async fn call_reducer(
        &self,
        name: &str,
        args: serde_json::Value,
    ) -> Result<()> {
        let resp = self
            .http
            .post(self.call_url(name))
            .bearer_auth(&self.token)
            .json(&args)
            .send()
            .await
            .map_err(|e| Error::Unknown(format!("SpacetimeDB {name} request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(Error::Unknown(format!(
                "SpacetimeDB {name} returned {status}: {body}"
            )));
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct AppState {
    pub spacetime: Arc<SpacetimeClient>,
    pub jwt_details: JwtDetails,
    pub yral_auth_jwt: YralAuthJwt,
}

impl AppState {
    pub async fn new(app_config: &AppConfig) -> Result<Self> {
        let spacetime_url =
            std::env::var("SPACETIMEDB_URL").map_err(|_| Error::Unknown("SPACETIMEDB_URL not set".into()))?;
        let spacetime_db = std::env::var("SPACETIMEDB_DB_NAME")
            .map_err(|_| Error::Unknown("SPACETIMEDB_DB_NAME not set".into()))?;
        let spacetime_token = std::env::var("SPACETIMEDB_ADMIN_TOKEN")
            .map_err(|_| Error::Unknown("SPACETIMEDB_ADMIN_TOKEN not set".into()))?;

        Ok(AppState {
            spacetime: Arc::new(SpacetimeClient::new(spacetime_url, spacetime_db, spacetime_token)),
            jwt_details: init_jwt(app_config)?,
            yral_auth_jwt: YralAuthJwt::init(app_config.yral_auth_public_key.clone())?,
        })
    }
}
