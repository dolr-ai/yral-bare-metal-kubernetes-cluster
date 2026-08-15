use crate::config::AppConfig;
use crate::consts::YRAL_METADATA_URL;
use crate::events::push_notifications::NotificationClient;
use crate::spacetime::{self, SpacetimeConnection};
use crate::utils::naitik_multi_service_client::NaitikMultiServiceClient;
use anyhow::{Context, Result};
use std::env;
use std::sync::Arc;
use yral_metadata_client::MetadataClient;

#[derive(Clone)]
pub struct AppState {
    pub yral_metadata_client: MetadataClient<true>,
    pub spacetime_conn: Option<Arc<SpacetimeConnection>>,

    pub notification_client: NotificationClient,
    pub config: AppConfig,
    pub user_migration_api_key: String,

    pub naitik_multi_service_client: NaitikMultiServiceClient,
}

impl AppState {
    pub async fn new(app_config: AppConfig) -> Self {
        AppState {
            yral_metadata_client: init_yral_metadata_client(&app_config),
            spacetime_conn: match spacetime::init_spacetimedb_connection().await {
                Ok(conn) => Some(conn),
                Err(e) => {
                    log::error!("Failed to connect to SpacetimeDB: {e}. View-count and delete calls will fall back to IC.");
                    None
                }
            },
            notification_client: NotificationClient::new(
                env::var("YRAL_METADATA_NOTIFICATION_API_KEY").unwrap_or_default(),
            ),
            config: app_config,
            user_migration_api_key: env::var("YRAL_OFF_CHAIN_USER_MIGRATION_API_KEY")
                .expect("YRAL_OFF_CHAIN_USER_MIGRATION_API_KEY is not set"),
            naitik_multi_service_client: NaitikMultiServiceClient::new(),
        }
    }

    pub async fn get_individual_canister_by_user_principal(
        &self,
        _user_principal: String,
    ) -> Result<String> {
        // IC canisters decommissioned — this method is kept for API compatibility
        // but returns an error since canisters no longer exist.
        Err(anyhow::anyhow!("IC canisters decommissioned"))
    }
}

pub fn init_yral_metadata_client(conf: &AppConfig) -> MetadataClient<true> {
    MetadataClient::with_base_url(YRAL_METADATA_URL.clone())
        .with_jwt_token(conf.yral_metadata_token.clone())
}
