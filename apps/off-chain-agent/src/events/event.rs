use crate::consts::{USER_INFO_SERVICE_CANISTER_ID, USER_POST_SERVICE_CANISTER_ID};
use crate::events::types::{
    VideoDurationWatchedPayload, VideoDurationWatchedPayloadV2, VideoStartedPayload,
};
use crate::{app_state::AppState, events::WarehouseEvent};
use log::{debug, error};

#[derive(Debug)]
pub struct Event {
    pub event: WarehouseEvent,
}

impl Event {
    pub fn new(event: WarehouseEvent) -> Self {
        Self { event }
    }

    // TODO: canister_id being used
    pub fn update_view_count_canister(&self, app_state: &AppState) {
        if self.event.event == "video_duration_watched" {
            // Try V3 first (new format with publisher_user_id)
            let params_v3: Result<VideoDurationWatchedPayloadV2, _> =
                serde_json::from_str(&self.event.params);

            let app_state = app_state.clone();

            match params_v3 {
                Ok(params) => {
                    // Handle V3 payload
                    tokio::spawn(async move {
                        use std::cmp::Ordering;
                        use canisters_client::user_post_service::{
                            PostViewDetailsFromFrontend as UserPostViewDetails, UserPostService,
                        };

                        let percentage_watched = params.percentage_watched as u8;
                        if percentage_watched == 0 || percentage_watched > 100 {
                            debug!("Invalid percentage_watched: {percentage_watched}");
                            return;
                        }
                        let post_id = params.post_id; // Already a String
                        let watch_count = 1u8;

                        // Get publisher user ID
                        let publisher_user_id = match params.publisher_user_id {
                            Some(id) => id,
                            None => {
                                error!("Missing publisher_user_id in V3 payload");
                                return;
                            }
                        };

                        // Get the publisher's canister
                        match app_state
                            .get_individual_canister_by_user_principal(publisher_user_id)
                            .await
                        {
                            Ok(publisher_canister_id) => {
                                // Check if it's the user post service canister
                                if publisher_canister_id == *USER_INFO_SERVICE_CANISTER_ID {
                                    // Use UserPostService
                                    let payload = match percentage_watched.cmp(&95) {
                                        Ordering::Less => UserPostViewDetails::WatchedPartially {
                                            percentage_watched,
                                        },
                                        _ => UserPostViewDetails::WatchedMultipleTimes {
                                            percentage_watched,
                                            watch_count,
                                        },
                                    };

                                    let user_post_service = UserPostService(
                                        *USER_POST_SERVICE_CANISTER_ID,
                                        &app_state.agent,
                                    );

                                    if let Err(e) = user_post_service
                                        .update_post_add_view_details(post_id.clone(), payload)
                                        .await
                                    {
                                        error!(
                                            "Failed to update view details for post {post_id} in UserPostService canister: {e:?}"
                                        );
                                    }
                                } else {
                                    // TODO: migrate to user_post_service/user_info_service
                                    // Individual user canisters have been decommissioned. Route view
                                    // detail updates through UserPostService for all users.
                                    let payload = match percentage_watched.cmp(&95) {
                                        Ordering::Less => UserPostViewDetails::WatchedPartially {
                                            percentage_watched,
                                        },
                                        _ => UserPostViewDetails::WatchedMultipleTimes {
                                            percentage_watched,
                                            watch_count,
                                        },
                                    };

                                    let user_post_service = UserPostService(
                                        *USER_POST_SERVICE_CANISTER_ID,
                                        &app_state.agent,
                                    );

                                    if let Err(e) = user_post_service
                                        .update_post_add_view_details(post_id.clone(), payload)
                                        .await
                                    {
                                        error!(
                                            "Failed to update view details for post {post_id} in UserPostService canister (legacy user): {e:?}"
                                        );
                                    }
                                }
                            }
                            Err(_) => {
                                let payload = match percentage_watched.cmp(&95) {
                                    Ordering::Less => {
                                        UserPostViewDetails::WatchedPartially { percentage_watched }
                                    }
                                    _ => UserPostViewDetails::WatchedMultipleTimes {
                                        percentage_watched,
                                        watch_count,
                                    },
                                };

                                let user_post_service = UserPostService(
                                    *USER_POST_SERVICE_CANISTER_ID,
                                    &app_state.agent,
                                );

                                let _ = user_post_service
                                    .update_post_add_view_details(post_id.clone(), payload)
                                    .await
                                    .map_err(|e| error!("Failed to update view details for post {post_id} in UserPostService canister (fallback): {e:?}"));
                            }
                        }
                    });
                }
                Err(_) => {
                    // Fall back to V2 (legacy format with publisher_canister_id)
                    let params_v2: Result<VideoDurationWatchedPayload, _> =
                        serde_json::from_str(&self.event.params);

                    match params_v2 {
                        Ok(params) => {
                            // Handle V2 payload (legacy)
                            // TODO: migrate to user_post_service/user_info_service
                            // Previously used IndividualUserTemplate to update view details.
                            // Individual user canisters have been decommissioned — route
                            // through UserPostService instead.
                            tokio::spawn(async move {
                                use std::cmp::Ordering;
                                use canisters_client::user_post_service::{
                                    PostViewDetailsFromFrontend as UserPostViewDetails,
                                    UserPostService,
                                };

                                let percentage_watched = params.percentage_watched as u8;
                                if percentage_watched == 0 || percentage_watched > 100 {
                                    debug!("Invalid percentage_watched: {percentage_watched}");
                                    return;
                                }
                                let post_id = params.post_id;
                                let watch_count = 1u8;

                                let payload = match percentage_watched.cmp(&95) {
                                    Ordering::Less => {
                                        UserPostViewDetails::WatchedPartially { percentage_watched }
                                    }
                                    _ => UserPostViewDetails::WatchedMultipleTimes {
                                        percentage_watched,
                                        watch_count,
                                    },
                                };

                                let user_post_service = UserPostService(
                                    *USER_POST_SERVICE_CANISTER_ID,
                                    &app_state.agent,
                                );

                                if let Err(e) = user_post_service
                                    .update_post_add_view_details(post_id.clone(), payload)
                                    .await
                                {
                                    error!(
                                        "Failed to update view details for post {post_id} in UserPostService canister (V2 legacy): {e:?}"
                                    );
                                }
                            });
                        }
                        Err(e) => {
                            error!(
                                "Failed to parse video_duration_watched params as V3 or V2: {e:?}"
                            );
                        }
                    }
                }
            }
        }
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
