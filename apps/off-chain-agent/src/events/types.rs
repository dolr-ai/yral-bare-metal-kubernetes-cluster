use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

pub fn string_or_number<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrNumber {
        String(String),
        Number(u64),
    }

    match StringOrNumber::deserialize(deserializer)? {
        StringOrNumber::String(s) => Ok(s),
        StringOrNumber::Number(n) => Ok(n.to_string()),
    }
}

#[derive(Serialize, Clone, Debug, ToSchema)]
#[serde(tag = "event")]
pub enum AnalyticsEvent {
    VideoWatched(VideoWatched),
    VideoDurationWatched(VideoDurationWatched),
    VideoStarted(VideoStarted),
    LikeVideo(LikeVideo),
}

// open issues for tagged and untagged enums - https://github.com/serde-rs/json/issues/1046 and https://github.com/serde-rs/json/issues/1108
impl<'de> Deserialize<'de> for AnalyticsEvent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // First deserialize to a generic Value to handle arbitrary_precision issues
        let value = Value::deserialize(deserializer)?;

        // Then try to deserialize from the Value to our enum
        match value.get("event").and_then(|v| v.as_str()) {
            Some("VideoWatched") => {
                let video_watched: VideoWatched =
                    serde_json::from_value(value).map_err(serde::de::Error::custom)?;
                Ok(AnalyticsEvent::VideoWatched(video_watched))
            }
            Some("VideoDurationWatched") => {
                let video_duration_watched: VideoDurationWatched =
                    serde_json::from_value(value).map_err(serde::de::Error::custom)?;
                Ok(AnalyticsEvent::VideoDurationWatched(video_duration_watched))
            }
            Some("VideoStarted") => {
                let video_started: VideoStarted =
                    serde_json::from_value(value).map_err(serde::de::Error::custom)?;
                Ok(AnalyticsEvent::VideoStarted(video_started))
            }
            Some("LikeVideo") => {
                let like_video: LikeVideo =
                    serde_json::from_value(value).map_err(serde::de::Error::custom)?;
                Ok(AnalyticsEvent::LikeVideo(like_video))
            }
            Some(event_type) => Err(serde::de::Error::custom(format!(
                "Unknown event type: {event_type}"
            ))),
            None => Err(serde::de::Error::custom("Missing 'event' field")),
        }
    }
}

impl AnalyticsEvent {
    pub fn tag(&self) -> String {
        match self {
            AnalyticsEvent::VideoWatched(_) => "video_watched".to_string(),
            AnalyticsEvent::VideoDurationWatched(_) => "video_duration_watched".to_string(),
            AnalyticsEvent::VideoStarted(_) => "video_started".to_string(),
            AnalyticsEvent::LikeVideo(_) => "like_video".to_string(),
        }
    }

    pub fn user_id(&self) -> Option<String> {
        match self {
            AnalyticsEvent::VideoWatched(e) => e.user_id.clone(),
            AnalyticsEvent::VideoDurationWatched(e) => Some(e.user_id.clone()),
            AnalyticsEvent::VideoStarted(e) => e.payload.user_id.clone(),
            AnalyticsEvent::LikeVideo(e) => Some(e.user_id.clone()),
        }
    }

    pub fn params(&self) -> Value {
        match self {
            AnalyticsEvent::VideoWatched(e) => serde_json::to_value(e).unwrap(),
            AnalyticsEvent::VideoDurationWatched(e) => serde_json::to_value(e).unwrap(),
            AnalyticsEvent::VideoStarted(e) => serde_json::to_value(e).unwrap(),
            AnalyticsEvent::LikeVideo(e) => serde_json::to_value(e).unwrap(),
        }
    }
}

// --------------------------------------------------
// VideoWatched (legacy V1)
// --------------------------------------------------

/// Wrapper for the VideoWatched analytics event.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VideoWatched {
    #[serde(rename = "user_id")]
    pub user_id: Option<String>,
    #[serde(rename = "canister_id")]
    pub canister_id: Option<String>,
    #[serde(rename = "video_id", skip_serializing_if = "Option::is_none")]
    pub video_id: Option<String>,
    #[serde(rename = "post_id", deserialize_with = "string_or_number")]
    pub post_id: String,
    #[serde(rename = "percentage_watched")]
    pub percentage_watched: f64,
    #[serde(rename = "absolute_watched")]
    pub absolute_watched: f64,
    #[serde(rename = "video_duration")]
    pub video_duration: f64,
}

/// Wrapper for the VideoDurationWatched analytics event.
pub type VideoDurationWatched = VideoDurationWatchedPayload;

/// Wrapper for the LikeVideo analytics event.
pub type LikeVideo = LikeVideoPayloadV2;

// --------------------------------------------------
// VideoDurationWatched
// --------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VideoDurationWatchedPayload {
    #[serde(rename = "publisher_user_id")]
    pub publisher_user_id: Option<String>,
    #[serde(rename = "user_id")]
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_logged_in: Option<bool>,
    #[serde(rename = "display_name", skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(rename = "canister_id")]
    pub canister_id: String,
    #[serde(rename = "video_id", skip_serializing_if = "Option::is_none")]
    pub video_id: Option<String>,
    #[serde(rename = "video_category")]
    pub video_category: String,
    #[serde(rename = "creator_category")]
    pub creator_category: String,
    #[serde(rename = "hashtag_count", skip_serializing_if = "Option::is_none")]
    pub hashtag_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_nsfw: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_hot_or_not: Option<bool>,
    #[serde(rename = "feed_type")]
    pub feed_type: String,
    #[serde(rename = "view_count", skip_serializing_if = "Option::is_none")]
    pub view_count: Option<u64>,
    #[serde(rename = "like_count", skip_serializing_if = "Option::is_none")]
    pub like_count: Option<u64>,
    #[serde(rename = "share_count")]
    pub share_count: u64,
    #[serde(rename = "percentage_watched")]
    pub percentage_watched: f64,
    #[serde(rename = "absolute_watched")]
    pub absolute_watched: f64,
    #[serde(rename = "video_duration")]
    pub video_duration: f64,
    #[serde(deserialize_with = "string_or_number")]
    pub post_id: String,
    #[serde(
        rename = "publisher_canister_id",
        skip_serializing_if = "Option::is_none"
    )]
    pub publisher_canister_id: Option<String>,
    #[serde(rename = "nsfw_probability", skip_serializing_if = "Option::is_none")]
    pub nsfw_probability: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VideoDurationWatchedPayloadV2 {
    pub publisher_user_id: Option<String>,
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_logged_in: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_id: Option<String>,
    pub video_category: String,
    pub creator_category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hashtag_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_nsfw: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_hot_or_not: Option<bool>,
    pub feed_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub like_count: Option<u64>,
    pub share_count: u64,
    pub percentage_watched: f64,
    pub absolute_watched: f64,
    pub video_duration: f64,
    #[serde(deserialize_with = "string_or_number")]
    pub post_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nsfw_probability: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VideoStartedPayload {
    pub video_id: String,
    pub publisher_user_id: String,
    pub feature_name: String,
    pub like_count: u64,
    pub share_count: u64,
    pub view_count: u64,
    pub is_game_enabled: bool,
    pub game_type: String,
    pub is_nsfw: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

// Wrapper type for VideoStarted
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VideoStarted {
    pub payload: VideoStartedPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LikeVideoPayloadV2 {
    pub publisher_user_id: String,
    pub user_id: String,
    pub is_logged_in: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub video_id: String,
    pub video_category: String,
    pub creator_category: String,
    pub hashtag_count: usize,
    #[serde(default)]
    pub is_nsfw: bool,
    #[serde(default)]
    pub is_hot_or_not: bool,
    pub feed_type: String,
    pub view_count: u64,
    pub like_count: u64,
    pub share_count: u64,
    #[serde(deserialize_with = "string_or_number")]
    pub post_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nsfw_probability: Option<f64>,
}