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

        // Try SpacetimeDB first; fall back to IC if not connected.
        if let Some(ref conn) = app_state.spacetime_conn {
            if let Err(e) = spacetime::send_view_details(conn, post_id.clone(), percentage_watched, watch_count) {
                error!("Failed to send view details to SpacetimeDB for post {post_id}: {e:?}");
            }
        } else {
            // Fallback: IC canister (deprecated — will be removed once SpacetimeDB is confirmed stable)
            self.update_view_count_canister_ic(app_state, post_id, percentage_watched, watch_count);
        }
    }

    /// IC canister fallback for view-count updates (deprecated).
    fn update_view_count_canister_ic(
        &self,
        app_state: &AppState,
        post_id: String,
        percentage_watched: u8,
        watch_count: u8,
    ) {
        use crate::consts::USER_POST_SERVICE_CANISTER_ID;
        use canisters_client::user_post_service::{
            PostViewDetailsFromFrontend as UserPostViewDetails, UserPostService,
        };
        use std::cmp::Ordering;

        let app_state = app_state.clone();

        tokio::spawn(async move {
            let payload = match percentage_watched.cmp(&95) {
                Ordering::Less => UserPostViewDetails::WatchedPartially { percentage_watched },
                _ => UserPostViewDetails::WatchedMultipleTimes {
                    percentage_watched,
                    watch_count,
                },
            };

            let user_post_service = UserPostService(*USER_POST_SERVICE_CANISTER_ID, &app_state.agent);

            if let Err(e) = user_post_service
                .update_post_add_view_details(post_id.clone(), payload)
                .await
            {
                error!("Failed to update view details for post {post_id} in IC UserPostService canister (fallback): {e:?}");
            }
        });
    }

    pub async fn process_btc_rewards(&self, app_state: &AppState) {
        if self.event.event != "video_duration_watched" {
            return;
        }

        // Parse the event parameters
        let params: Result<VideoDurationWatchedPayloadV2, _> =
            serde_json::from_str(&self.event.params);

        let params = match params {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to parse video_duration_watched params for rewards: {e:?}");
                return;
            }
        };

        // Initialize reward engine
        let reward_engine = app_state.rewards_module.reward_engine.clone();

        // Process the view for rewards
        let app_state_arc = std::sync::Arc::new(app_state.clone());
        if let Err(e) = reward_engine
            .process_video_view(params, &app_state_arc)
            .await
        {
            log::error!("Failed to process BTC rewards: {e:?}");
        }
    }

    pub async fn process_video_started_event(&self, app_state: &AppState) {
        if self.event.event != "video_started" {
            return;
        }

        // Parse the event parameters
        let params: Result<VideoStartedPayload, _> = serde_json::from_str(&self.event.params);

        let params = match params {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to parse video_started params: {e:?}");
                return;
            }
        };

        // Initialize reward engine
        let reward_engine = app_state.rewards_module.reward_engine.clone();

        // Process the video started event
        if let Err(e) = reward_engine.process_video_started(params).await {
            log::error!("Failed to process video_started event: {e:?}");
        }
    }
}
