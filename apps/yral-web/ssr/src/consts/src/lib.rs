mod remote;

pub use remote::*;

use once_cell::sync::Lazy;
use reqwest::Url;

/// Base URL for the self-hosted media CDN (video + thumbnails).
pub const MEDIA_CDN_BASE: &str = "https://cdn-yral-sfw.yral.com";
pub const ACCOUNT_CONNECTED_STORE: &str = "account-connected-1";
pub const DEVICE_ID: &str = "device_id";
pub const CUSTOM_DEVICE_ID: &str = "custom_device_id";
pub const NOTIFICATIONS_ENABLED_STORE: &str = "yral-notifications-enabled";
pub const NOTIFICATION_MIGRATED_STORE: &str = "notifications-migrated";
pub const NSFW_TOGGLE_STORE: &str = "nsfw-enabled";
pub const NSFW_ENABLED_COOKIE: &str = "nsfw-enabled-cookie";
pub const USER_CANISTER_ID_STORE: &str = "user-canister-id";
pub const USER_PRINCIPAL_STORE: &str = "user-principal";
pub const USER_INTERNAL_STORE: &str = "user-internal";

pub static OFF_CHAIN_AGENT_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://offchain.yral.com").unwrap());

pub static ANALYTICS_SERVER_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://analytics.yral.com").unwrap());

pub static SMILEY_GAME_STATS_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://us-central1-yral-mobile.cloudfunctions.net").unwrap());

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
    /// ID token cookie — non-httpOnly so client-side WASM can read it
    /// for SpacetimeDB authentication. Contains the yral-auth id_token JWT.
    pub const ID_TOKEN_COOKIE: &str = "id-token";
    /// Access/ID token expiry, 7 days (matches ACCESS_TOKEN_MAX_AGE in yral-auth)
    pub const ID_TOKEN_MAX_AGE: Duration = Duration::from_secs(60 * 60 * 24 * 7);
    /// Refresh threshold — if less than this remains, refresh the token
    pub const ONE_HOUR_SECS: usize = 60 * 60;
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
