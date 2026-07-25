use candid::Principal;
use once_cell::sync::Lazy;
use reqwest::Url;

pub static NAITIK_YRAL_MULTI_SERVICES: Lazy<Url> =
    Lazy::new(|| Url::parse("https://multi-service.naitik.yral.com").unwrap());

pub static YRAL_METADATA_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://metadata.yral.com/").unwrap());

#[allow(dead_code)]
pub const RECYCLE_THRESHOLD_SECS: u64 = 15 * 24 * 60 * 60; // 15 days

#[allow(dead_code)]
pub const CLOUDFLARE_ACCOUNT_ID: &str = "a209c523d2d9646cc56227dbe6ce3ede";

#[allow(dead_code)]
pub const ICP_LEDGER_CANISTER_ID: &str = "ryjl3-tyaaa-aaaaa-aaaba-cai";

pub static OFF_CHAIN_AGENT_URL: Lazy<Url> = Lazy::new(|| {
    let url = std::env::var("OFF_CHAIN_AGENT_URL")
        .unwrap_or_else(|_| "https://offchain.yral.com/".into());
    Url::parse(&url).unwrap()
});

// Rate Limiting Constants
pub static RATE_LIMITS_CANISTER_ID: Lazy<Principal> = Lazy::new(|| {
    "h2jgv-ayaaa-aaaas-qbh4a-cai"
        .parse()
        .expect("Rate limits canister ID to be valid")
});

// User Info Service Constants
pub static USER_INFO_SERVICE_CANISTER_ID: Lazy<Principal> = Lazy::new(|| {
    "ivkka-7qaaa-aaaas-qbg3q-cai"
        .parse()
        .expect("User info service canister ID to be valid")
});

// User Post Service Constants
pub static USER_POST_SERVICE_CANISTER_ID: Lazy<Principal> = Lazy::new(|| {
    "gxhc3-pqaaa-aaaas-qbh3q-cai"
        .parse()
        .expect("User post service canister ID to be valid")
});
