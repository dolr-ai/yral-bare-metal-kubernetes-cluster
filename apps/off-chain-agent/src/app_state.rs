use crate::config::AppConfig;
use crate::spacetime::{self, SpacetimeConnection};
use crate::utils::naitik_multi_service_client::NaitikMultiServiceClient;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub spacetime_conn: Option<Arc<SpacetimeConnection>>,
    pub config: AppConfig,
    pub naitik_multi_service_client: NaitikMultiServiceClient,
}

impl AppState {
    pub async fn new(app_config: AppConfig) -> Self {
        AppState {
            spacetime_conn: match spacetime::init_spacetimedb_connection().await {
                Ok(conn) => Some(conn),
                Err(e) => {
                    log::error!("Failed to connect to SpacetimeDB: {e}.");
                    None
                }
            },
            config: app_config,
            naitik_multi_service_client: NaitikMultiServiceClient::new(),
        }
    }
}
