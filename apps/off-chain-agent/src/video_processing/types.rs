use serde::{Deserialize, Serialize};

use crate::events::types::string_or_number;

/// Publisher data for video deduplication
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoPublisherDataV2 {
    pub publisher_principal: String,
    #[serde(deserialize_with = "string_or_number")]
    pub post_id: String,
}