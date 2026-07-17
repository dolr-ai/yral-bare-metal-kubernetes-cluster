use candid::Principal;
use once_cell::sync::Lazy;
use reqwest::Url;

/// with nsfw detection v2, nsfw probablity greater or equal to this is considered nsfw
#[allow(dead_code)]
pub const NSFW_THRESHOLD: f32 = 0.4;

pub static NAITIK_YRAL_MULTI_SERVICES: Lazy<Url> =
    Lazy::new(|| Url::parse("https://multi-service.naitik.yral.com").unwrap());

pub static YRAL_METADATA_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://metadata.yral.com/").unwrap());

#[allow(dead_code)]
pub const RECYCLE_THRESHOLD_SECS: u64 = 15 * 24 * 60 * 60; // 15 days

pub static GOOGLE_CHAT_REPORT_SPACE_URL: Lazy<String> = Lazy::new(|| {
    std::env::var("GOOGLE_CHAT_REPORT_SPACE_URL").expect("GOOGLE_CHAT_REPORT_SPACE_URL must be set")
});

#[allow(dead_code)]
pub const CLOUDFLARE_ACCOUNT_ID: &str = "a209c523d2d9646cc56227dbe6ce3ede";

#[allow(dead_code)]
pub const ICP_LEDGER_CANISTER_ID: &str = "ryjl3-tyaaa-aaaaa-aaaba-cai";

pub static OFF_CHAIN_AGENT_URL: Lazy<Url> = Lazy::new(|| {
    let url = std::env::var("OFF_CHAIN_AGENT_URL")
        .unwrap_or_else(|_| "https://offchain.yral.com/".into());
    Url::parse(&url).unwrap()
});

pub static STORJ_INTERFACE_URL: Lazy<Url> =
    Lazy::new(|| Url::parse("https://storage-interface.prakash.yral.com").unwrap());

pub static STORJ_INTERFACE_TOKEN: Lazy<String> =
    Lazy::new(|| std::env::var("STORJ_INTERFACE_TOKEN").expect("STORJ_INTERFACE_TOKEN to be set"));

// Storj Public Bucket URLs
pub const STORJ_SFW_BUCKET_URL: &str =
    "https://link.storjshare.io/raw/jxepcyfzxbj5mk4d676jhsfjpg5a/yral-sfw";
pub const STORJ_NSFW_BUCKET_URL: &str =
    "https://link.storjshare.io/raw/jxflpcetc5iwtfu6y6co2iugaewa/yral-nsfw-videos";

pub fn get_storj_video_url(publisher_user_id: &str, video_id: &str, is_nsfw: bool) -> String {
    let bucket_url = if is_nsfw {
        STORJ_NSFW_BUCKET_URL
    } else {
        STORJ_SFW_BUCKET_URL
    };
    format!("{}/{}/{}.mp4", bucket_url, publisher_user_id, video_id)
}

// Cloudflare Stream URL
pub const CLOUDFLARE_STREAM_CUSTOMER_SUBDOMAIN: &str = "customer-2p3jflss4r4hmpnz";

pub fn get_cloudflare_stream_url(video_id: &str) -> String {
    format!(
        "https://{}.cloudflarestream.com/{}/watch",
        CLOUDFLARE_STREAM_CUSTOMER_SUBDOMAIN, video_id
    )
}

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
