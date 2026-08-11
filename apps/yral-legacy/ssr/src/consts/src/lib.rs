mod remote;

use candid::Principal;
pub use remote::*;

use once_cell::sync::Lazy;
use reqwest::Url;
use serde::{Deserialize, Serialize};

pub const CF_STREAM_BASE: &str = "https://customer-2p3jflss4r4hmpnz.cloudflarestream.com";
pub const FALLBACK_PROPIC_BASE: &str = "https://api.dicebear.com/7.x/big-smile/svg";
// an example URL is "https://imagedelivery.net/abXI9nS4DYYtyR1yFFtziA/gob.5/public";
pub const GOBGOB_PROPIC_URL: &str = "https://imagedelivery.net/abXI9nS4DYYtyR1yFFtziA/gob.";
pub const GOBGOB_TOTAL_COUNT: u32 = 18557;
pub const CF_WATERMARK_UID: &str = "b5588fa1516ca33a08ebfef06c8edb33";
pub const ACCOUNT_CONNECTED_STORE: &str = "account-connected-1";
pub const DEVICE_ID: &str = "device_id";
pub const CUSTOM_DEVICE_ID: &str = "custom_device_id";
pub static CF_BASE_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://api.cloudflare.com/client/v4/").unwrap());
pub const NOTIFICATIONS_ENABLED_STORE: &str = "yral-notifications-enabled";
pub const NOTIFICATION_MIGRATED_STORE: &str = "notifications-migrated";
pub const NSFW_TOGGLE_STORE: &str = "nsfw-enabled";
pub const NSFW_ENABLED_COOKIE: &str = "nsfw-enabled-cookie";
pub const REFERRER_COOKIE: &str = "referrer";
pub const USER_CANISTER_ID_STORE: &str = "user-canister-id";
pub const USER_PRINCIPAL_STORE: &str = "user-principal";
pub const USER_INTERNAL_STORE: &str = "user-internal";

pub static OFF_CHAIN_AGENT_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://offchain.yral.com").unwrap());

pub static ANALYTICS_SERVER_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://analytics.yral.com").unwrap());

pub static SMILEY_GAME_STATS_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://us-central1-yral-mobile.cloudfunctions.net").unwrap());

pub const CF_KV_ML_CACHE_NAMESPACE_ID: &str = "ea145fc839bd42f9bf2d34b950ddbda5";
pub const CLOUDFLARE_ACCOUNT_ID: &str = "a209c523d2d9646cc56227dbe6ce3ede";

pub const AUTH_UTIL_COOKIES_MAX_AGE_MS: i64 = 400 * 24 * 60 * 60 * 1000; // 400 days

pub const MAX_VIDEO_ELEMENTS_FOR_FEED: usize = 200;

pub mod social {
    pub const TELEGRAM_YRAL: &str = "https://t.me/+c-LTX0Cp-ENmMzI1";
    pub const DISCORD: &str = "https://discord.gg/GZ9QemnZuj";
    pub const TWITTER_YRAL: &str = "https://twitter.com/Yral_app";
    pub const IC_WEBSITE: &str = "https://vyatz-hqaaa-aaaam-qauea-cai.ic0.app";
}

pub mod auth {
    use web_time::Duration;

    /// Delegation Expiry, 7 days
    pub const DELEGATION_MAX_AGE: Duration = Duration::from_secs(60 * 60 * 24 * 7);
    /// Refresh expiry, 29 days
    pub const REFRESH_MAX_AGE: Duration = Duration::from_secs(60 * 60 * 24 * 29);
    pub const REFRESH_TOKEN_COOKIE: &str = "user-identity";
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LoginProvider {
    Any,
    Google,
    Apple,
}

#[cfg(feature = "oauth-ssr")]
pub mod yral_auth {
    use jsonwebtoken::DecodingKey;
    use std::sync::LazyLock;

    pub const YRAL_AUTH_AUTHORIZATION_URL: &str = "https://auth.yral.com/oauth/auth";
    pub const YRAL_AUTH_TOKEN_URL: &str = "https://auth.yral.com/oauth/token";
    pub const YRAL_AUTH_ISSUER_URL: &str = "https://auth.yral.com";

    pub static YRAL_AUTH_TRUSTED_KEY: LazyLock<DecodingKey> = LazyLock::new(|| {
        let pem = "-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEoqN3/0RNfrnrnYGxKBgy/qHnmITr
+6ucjxStx7tjA30QJZlWzo0atxmY8y9dUR+eKQI0SnbQds4xLEU8+JGm8Q==
-----END PUBLIC KEY-----";
        DecodingKey::from_ec_pem(pem.as_bytes()).unwrap()
    });

    pub const YRAL_AUTH_CLIENT_ID_ENV: &str = "YRAL_AUTH_CLIENT_ID";
}

pub const USER_ONBOARDING_STORE_KEY: &str = "user-onboarding";
#[derive(Serialize, Deserialize, Clone, PartialEq, Default)]
pub struct UserOnboardingStore {
    pub has_seen_onboarding: bool,
    pub has_seen_hon_bet_help: bool,
}
