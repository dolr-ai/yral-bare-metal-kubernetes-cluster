use crate::auth::init_jwt;
use crate::auth::JwtDetails;
use crate::config::AppConfig;
use crate::dragonfly::{
    get_redis_store_ca_cert_pem, get_redis_store_client_cert_pem, get_redis_store_client_key_pem,
    init_dragonfly_redis_store, DragonflyPool,
};
use crate::utils::error::{Error, Result};
use crate::utils::yral_auth_jwt::YralAuthJwt;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub dragonfly_redis_store: Arc<DragonflyPool>,
    pub jwt_details: JwtDetails,
    pub yral_auth_jwt: YralAuthJwt,
}

impl AppState {
    pub async fn new(app_config: &AppConfig) -> Result<Self> {
        let redis_store_ca_cert_bytes = get_redis_store_ca_cert_pem()?;
        let redis_store_client_cert_bytes = get_redis_store_client_cert_pem()?;
        let redis_store_client_key_bytes = get_redis_store_client_key_pem()?;

        Ok(AppState {
            dragonfly_redis_store: init_dragonfly_redis_store(
                redis_store_ca_cert_bytes,
                redis_store_client_cert_bytes,
                redis_store_client_key_bytes,
            )
            .await?,
            jwt_details: init_jwt(app_config)?,
            yral_auth_jwt: YralAuthJwt::init(app_config.yral_auth_public_key.clone())?,
        })
    }
}
