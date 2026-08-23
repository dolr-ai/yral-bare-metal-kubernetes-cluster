use once_cell::sync::Lazy;
use reqwest::Url;

pub static NAITIK_YRAL_MULTI_SERVICES: Lazy<Url> =
    Lazy::new(|| Url::parse("https://multi-service.naitik.yral.com").unwrap());
