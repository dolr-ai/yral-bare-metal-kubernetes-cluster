use once_cell::sync::Lazy;
use reqwest::Url;

pub static NAITIK_YRAL_MULTI_SERVICES: Lazy<Url> =
    Lazy::new(|| Url::parse("https://multi-service.naitik.yral.com").unwrap());

#[allow(dead_code)]
pub const RECYCLE_THRESHOLD_SECS: u64 = 15 * 24 * 60 * 60; // 15 days

pub static OFF_CHAIN_AGENT_URL: Lazy<Url> = Lazy::new(|| {
    let url = std::env::var("OFF_CHAIN_AGENT_URL")
        .unwrap_or_else(|_| "https://offchain.yral.com/".into());
    Url::parse(&url).unwrap()
});
