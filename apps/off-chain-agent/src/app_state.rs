use crate::config::AppConfig;
use crate::consts::YRAL_METADATA_URL;
#[cfg(not(feature = "local-bin"))]
use crate::events::push_notifications::NotificationClient;
use crate::kvrocks::KvrocksClient;
use crate::rewards::RewardsModule;
use crate::scratchpad::ScratchpadClient;
use crate::types::RedisPool;
use crate::utils::naitik_multi_service_client::NaitikMultiServiceClient;
use crate::yral_auth::dragonfly::{
    get_redis_store_ca_cert, get_redis_store_client_cert, get_redis_store_client_key,
    init_dragonfly_redis_store, DragonflyPool,
};
use anyhow::{anyhow, Context, Result};
use candid::Principal;
use hyper_util::client::legacy::connect::HttpConnector;
use ic_agent::identity::Secp256k1Identity;
use ic_agent::Agent;
use std::env;
use std::sync::Arc;
use yral_metadata_client::MetadataClient;
use yup_oauth2::hyper_rustls::HttpsConnector;
use yup_oauth2::{authenticator::Authenticator, ServiceAccountAuthenticator};

#[derive(Clone)]
pub struct AppState {
    pub admin_identity: Secp256k1Identity,
    pub agent: ic_agent::Agent,
    pub yral_metadata_client: MetadataClient<true>,
    #[cfg(not(feature = "local-bin"))]
    pub auth: Authenticator<HttpsConnector<HttpConnector>>,
    /// Google Chat App authenticator (for sending messages with interactive buttons)
    #[cfg(not(feature = "local-bin"))]
    pub gchat_auth: Authenticator<HttpsConnector<HttpConnector>>,

    #[cfg(not(feature = "local-bin"))]
    pub notification_client: NotificationClient,
    #[cfg(not(feature = "local-bin"))]
    pub yral_auth_dragonfly: Arc<DragonflyPool>,
    #[cfg(not(feature = "local-bin"))]
    pub yral_redis_store_dragonfly: Arc<DragonflyPool>,
    pub leaderboard_redis_pool: RedisPool,
    #[cfg(not(feature = "local-bin"))]
    pub rewards_module: RewardsModule,
    pub service_cansister_migration_redis_pool: RedisPool,
    pub config: AppConfig,
    pub user_migration_api_key: String,
    #[cfg(not(feature = "local-bin"))]
    pub kvrocks_client: KvrocksClient,

    // This uses ds staging
    #[cfg(not(feature = "local-bin"))]
    pub scratchpad_client: ScratchpadClient,

    pub naitik_multi_service_client: NaitikMultiServiceClient,
}

impl AppState {
    pub async fn new(app_config: AppConfig) -> Self {
        let leaderboard_redis_pool = init_leaderboard_redis_pool().await;
        let agent = init_agent().await;

        //Initialize central redis data store for auth, metadata and rewards/impressions.
        #[cfg(not(feature = "local-bin"))]
        let dragonfly_redis_store = init_dragonfly_redis_store_pool().await;

        #[cfg(not(feature = "local-bin"))]
        let mut rewards_module =
            RewardsModule::new(dragonfly_redis_store.clone(), agent.clone()).await;

        // Initialize the rewards module (loads Lua scripts)
        #[cfg(not(feature = "local-bin"))]
        if let Err(e) = rewards_module.initialize().await {
            log::error!("Failed to initialize rewards module: {}", e);
        }

        #[cfg(not(feature = "local-bin"))]
        let kvrocks_client = init_kvrocks_client().await;
        #[cfg(not(feature = "local-bin"))]
        let scratchpad_client = init_scratchpad_client().await;

        AppState {
            admin_identity: init_identity(),
            yral_metadata_client: init_yral_metadata_client(&app_config),
            agent,
            #[cfg(not(feature = "local-bin"))]
            auth: init_auth().await,
            #[cfg(not(feature = "local-bin"))]
            gchat_auth: init_gchat_auth().await,
            // ml_server_grpc_channel: init_ml_server_grpc_channel().await,
            #[cfg(not(feature = "local-bin"))]
            notification_client: NotificationClient::new(
                env::var("YRAL_METADATA_NOTIFICATION_API_KEY").unwrap_or_default(),
            ),
            #[cfg(not(feature = "local-bin"))]
            yral_auth_dragonfly: dragonfly_redis_store.clone(),
            #[cfg(not(feature = "local-bin"))]
            yral_redis_store_dragonfly: dragonfly_redis_store,
            leaderboard_redis_pool,
            #[cfg(not(feature = "local-bin"))]
            rewards_module,
            config: app_config,
            service_cansister_migration_redis_pool: init_service_canister_migration_redis_pool()
                .await,
            user_migration_api_key: env::var("YRAL_OFF_CHAIN_USER_MIGRATION_API_KEY")
                .expect("YRAL_OFF_CHAIN_USER_MIGRATION_API_KEY is not set"),
            #[cfg(not(feature = "local-bin"))]
            kvrocks_client,
            #[cfg(not(feature = "local-bin"))]
            scratchpad_client,
            naitik_multi_service_client: NaitikMultiServiceClient::new(),
        }
    }

    pub async fn get_access_token(&self, scopes: &[&str]) -> String {
        #[cfg(feature = "local-bin")]
        {
            "localtoken".into()
        }

        #[cfg(not(feature = "local-bin"))]
        {
            let auth = &self.auth;
            let token = auth.token(scopes).await.unwrap();

            match token.token() {
                Some(t) => t.to_string(),
                _ => panic!("No access token found"),
            }
        }
    }

    /// Get access token for Google Chat API using the yral-mobile service account
    pub async fn get_gchat_access_token(&self) -> String {
        #[cfg(feature = "local-bin")]
        {
            "localtoken".into()
        }

        #[cfg(not(feature = "local-bin"))]
        {
            let auth = &self.gchat_auth;
            let token = auth
                .token(&["https://www.googleapis.com/auth/chat.bot"])
                .await
                .expect("Failed to get Google Chat access token");

            match token.token() {
                Some(t) => t.to_string(),
                _ => panic!("No Google Chat access token found"),
            }
        }
    }

    pub async fn get_individual_canister_by_user_principal(
        &self,
        user_principal: Principal,
    ) -> Result<Principal> {
        let meta = self
            .yral_metadata_client
            .get_user_metadata_v2(user_principal.to_string())
            .await
            .context("Failed to get user_metadata from yral_metadata_client")?;

        match meta {
            Some(meta) => Ok(meta.user_canister_id),
            None => Err(anyhow!(
                "user metadata does not exist in yral_metadata_service"
            )),
        }
    }
}

pub fn init_yral_metadata_client(conf: &AppConfig) -> MetadataClient<true> {
    MetadataClient::with_base_url(YRAL_METADATA_URL.clone())
        .with_jwt_token(conf.yral_metadata_token.clone())
}

pub fn init_identity() -> ic_agent::identity::Secp256k1Identity {
    #[cfg(not(any(feature = "local-bin", feature = "use-local-agent")))]
    {
        let pk = env::var("BACKEND_ADMIN_IDENTITY").expect("$BACKEND_ADMIN_IDENTITY is not set");
        match ic_agent::identity::Secp256k1Identity::from_pem(pk.as_str()) {
            Ok(identity) => identity,
            Err(err) => {
                panic!("Unable to create identity, error: {err:?}");
            }
        }
    }

    #[cfg(any(feature = "use-local-agent", feature = "local-bin"))]
    {
        use k256::elliptic_curve::{rand_core, SecretKey};
        use rand::rng;

        let mut rng = rand_core::OsRng {};

        ic_agent::identity::Secp256k1Identity::from_private_key(SecretKey::random(&mut rng))
    }
}

pub async fn init_agent() -> Agent {
    #[cfg(not(any(feature = "local-bin", feature = "use-local-agent")))]
    {
        let pk = env::var("BACKEND_ADMIN_IDENTITY").expect("$BACKEND_ADMIN_IDENTITY is not set");

        let identity = match ic_agent::identity::Secp256k1Identity::from_pem(pk.as_str()) {
            Ok(identity) => identity,
            Err(err) => {
                panic!("Unable to create identity, error: {err:?}");
            }
        };

        match Agent::builder()
            .with_url("https://a4gq6-oaaaa-aaaab-qaa4q-cai.raw.ic0.app/") // https://a4gq6-oaaaa-aaaab-qaa4q-cai.raw.ic0.app/
            .with_identity(identity)
            .build()
        {
            Ok(agent) => agent,
            Err(err) => {
                panic!("Unable to create agent, error: {err:?}");
            }
        }
    }

    #[cfg(feature = "local-bin")]
    {
        let agent = Agent::builder()
            .with_url("https://ic0.app")
            .build()
            .unwrap();

        // agent.fetch_root_key().await.unwrap();

        agent
    }

    #[cfg(feature = "use-local-agent")]
    {
        let pk = env::var("BACKEND_ADMIN_IDENTITY").expect("$BACKEND_ADMIN_IDENTITY is not set");

        let identity = match ic_agent::identity::Secp256k1Identity::from_pem(pk.as_str()) {
            Ok(identity) => identity,
            Err(err) => {
                panic!("Unable to create identity, error: {:?}", err);
            }
        };

        match Agent::builder()
            .with_url("https://ic0.app") // https://a4gq6-oaaaa-aaaab-qaa4q-cai.raw.ic0.app/
            .with_identity(identity)
            .build()
        {
            Ok(agent) => agent,
            Err(err) => {
                panic!("Unable to create agent, error: {:?}", err);
            }
        }
    }
}

pub async fn init_auth() -> Authenticator<HttpsConnector<HttpConnector>> {
    let sa_key_file = env::var("GOOGLE_SA_KEY").expect("GOOGLE_SA_KEY is required");

    // Load your service account key
    let sa_key = yup_oauth2::parse_service_account_key(sa_key_file).expect("GOOGLE_SA_KEY.json");

    ServiceAccountAuthenticator::builder(sa_key)
        .build()
        .await
        .unwrap()
}

/// Initialize Google Chat App authenticator using YRAL_MOBILE_SERVICE_ACCOUNT_KEY
/// This is needed to send messages as the Chat App (so interactive buttons work)
pub async fn init_gchat_auth() -> Authenticator<HttpsConnector<HttpConnector>> {
    let sa_key_file = env::var("YRAL_MOBILE_SERVICE_ACCOUNT_KEY")
        .expect("YRAL_MOBILE_SERVICE_ACCOUNT_KEY is required");

    let sa_key = yup_oauth2::parse_service_account_key(sa_key_file)
        .expect("Invalid YRAL_MOBILE_SERVICE_ACCOUNT_KEY");

    ServiceAccountAuthenticator::builder(sa_key)
        .build()
        .await
        .expect("Failed to build Google Chat authenticator")
}

async fn init_leaderboard_redis_pool() -> RedisPool {
    let redis_url =
        std::env::var("LEADERBOARD_REDIS_URL").expect("Either LEADERBOARD_REDIS_URL must be set");

    let manager = bb8_redis::RedisConnectionManager::new(redis_url.clone())
        .expect("failed to open connection to redis");
    RedisPool::builder().build(manager).await.unwrap()
}

async fn init_service_canister_migration_redis_pool() -> RedisPool {
    let redis_url = std::env::var("SERVICE_CANISTER_MIGRATION_REDIS_URL")
        .expect("SERVICE_CANISTER_MIGRATION_REDIS_URL is not set");

    let manager = bb8_redis::RedisConnectionManager::new(redis_url.clone())
        .expect("failed to open connection to redis");
    RedisPool::builder().build(manager).await.unwrap()
}

async fn init_dragonfly_redis_store_pool() -> Arc<DragonflyPool> {
    let ca_cert_bytes =
        get_redis_store_ca_cert().expect("Failed to read DRAGONFLY_REDIS_STORE_CA_CERT");
    let client_cert_bytes =
        get_redis_store_client_cert().expect("Failed to read DRAGONFLY_REDIS_STORE_CLIENT_CERT");
    let client_key_bytes =
        get_redis_store_client_key().expect("Failed to read DRAGONFLY_REDIS_STORE_CLIENT_KEY");

    let dragonfly_pool: Arc<DragonflyPool> =
        init_dragonfly_redis_store(ca_cert_bytes, client_cert_bytes, client_key_bytes)
            .await
            .expect("failed to initalize DragonflyPool");
    dragonfly_pool
}

const KVROCKS_MAX_RETRIES: u32 = 10;

async fn init_kvrocks_client() -> KvrocksClient {
    let mut attempt = 0u32;
    loop {
        match crate::kvrocks::init_kvrocks_client().await {
            Ok(client) => return client,
            Err(e) => {
                attempt += 1;
                if attempt >= KVROCKS_MAX_RETRIES {
                    log::error!(
                        "Failed to connect to kvrocks after {attempt} attempts: {e}. Retrying..."
                    );
                }
                let delay = std::time::Duration::from_secs(2u64.pow(attempt.min(7)).min(120));
                tokio::time::sleep(delay).await;
            }
        }
    }
}

async fn init_scratchpad_client() -> ScratchpadClient {
    crate::scratchpad::init_scratchpad_client()
        .await
        .expect("Failed to connect to scratchpad Dragonfly - this is required for the application to function")
}
