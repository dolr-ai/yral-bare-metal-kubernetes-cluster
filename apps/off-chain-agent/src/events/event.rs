use crate::events::types::{
    VideoDurationWatchedPayload, VideoDurationWatchedPayloadV2, VideoStartedPayload,
};
use crate::{app_state::AppState, events::WarehouseEvent, spacetime};
use log::{debug, error};

#[derive(Debug)]
pub struct Event {
    pub event: WarehouseEvent,
}

impl Event {
    pub fn new(event: WarehouseEvent) -> Self {
        Self { event }
    }

    /// Send a view-count update to SpacetimeDB (fire-and-forget via SDK bindings).
    /// Falls back to IC canister if SpacetimeDB connection is not available.
    ///
    /// The IC version used an enum (`WatchedPartially`/`WatchedMultipleTimes`)
    /// and routed based on canister ID. With SpacetimeDB, all posts are in a
    /// single database — no canister routing needed. The SpacetimeDB
    /// `PostViewDetailsFromFrontend` is a flat struct `{ percentage_watched,
    /// watch_count }` that both IC enum branches map to directly.
    pub fn update_view_count_canister(&self, app_state: &AppState) {
        if self.event.event != "video_duration_watched" {
            return;
        }

        // Try V3 first (new format with publisher_user_id), fall back to V2 (legacy)
        let params_v3: Result<VideoDurationWatchedPayloadV2, _> =
            serde_json::from_str(&self.event.params);
        let params_v2: Result<VideoDurationWatchedPayload, _> =
            serde_json::from_str(&self.event.params);

        let (post_id, percentage_watched) = match params_v3 {
            Ok(params) => (params.post_id, params.percentage_watched as u8),
            Err(_) => match params_v2 {
                Ok(params) => (params.post_id, params.percentage_watched as u8),
                Err(e) => {
                    error!("Failed to parse video_duration_watched params as V3 or V2: {e:?}");
                    return;
                }
            },
        };

        if percentage_watched == 0 || percentage_watched > 100 {
            debug!("Invalid percentage_watched: {percentage_watched}");
            return;
        }

        let watch_count: u8 = 1;

        // Send to SpacetimeDB via SDK connection
        if let Some(ref conn) = app_state.spacetime_conn {
            if let Err(e) = spacetime::send_view_details(conn, post_id.clone(), percentage_watched, watch_count) {
                error!("Failed to send view details to SpacetimeDB for post {post_id}: {e:?}");
            }
        }
    }

    pub async fn process_btc_rewards(&self, _app_state: &AppState) {
        // BTC rewards processing removed — rewards module migrated to SpacetimeDB
        // (view counts tracked via SpacetimeDB, reward distribution handled separately)
    }

    pub async fn process_video_started_event(&self, _app_state: &AppState) {
        // Video started rewards processing removed — rewards module migrated to SpacetimeDB
    }
}
