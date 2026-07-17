use crate::consts::{USER_INFO_SERVICE_CANISTER_ID, USER_POST_SERVICE_CANISTER_ID};
use crate::events::types::{
    string_or_number, VideoDurationWatchedPayload, VideoDurationWatchedPayloadV2,
    VideoStartedPayload, VideoUploadSuccessfulPayload,
};
use crate::pipeline::Step;
use crate::setup_context;
use crate::{app_state::AppState, events::WarehouseEvent, AppError};
use axum::{extract::State, Json};
use log::{debug, error};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tracing::instrument;

pub mod storj;

/// Flat event for Mixpanel - event name + all params at same level
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlatEvent {
    pub event: String,
    #[serde(flatten)]
    pub params: Value,
}

#[derive(Debug)]
pub struct Event {
    pub event: WarehouseEvent,
}

impl Event {
    pub fn new(event: WarehouseEvent) -> Self {
        Self { event }
    }

    /// Convert to flat event (for Mixpanel)
    /// Returns None if no principal can be determined (analytics server requires it)
    #[allow(dead_code)]
    fn to_flat_event(&self) -> Option<FlatEvent> {
        let mut params: Value = serde_json::from_str(&self.event.params).ok()?;

        // Remove "event" from params if present (avoid duplication)
        if let Value::Object(ref mut map) = params {
            map.remove("event");

            // Analytics server requires "principal" field
            // Check multiple possible sources in priority order
            if !map.contains_key("principal") {
                let principal_value = map
                    .get("user_id")
                    .or_else(|| map.get("publisher_user_id"))
                    .or_else(|| map.get("creator_id"))
                    .or_else(|| map.get("viewer_id"))
                    .cloned();

                if let Some(value) = principal_value {
                    map.insert("principal".to_string(), value);
                } else {
                    // No principal found - skip this event for Mixpanel
                    log::debug!(
                        "Skipping event '{}' for Mixpanel - no principal field found",
                        self.event.event
                    );
                    return None;
                }
            }
        }

        Some(FlatEvent {
            event: self.event.event.clone(),
            params,
        })
    }

    /// Mixpanel format: {event: string, user_id: string, video_id: string, ...} (flat)
    #[allow(dead_code)]
    pub fn forward_to_mixpanel(&self, app_state: &AppState) {
        let mixpanel_client = app_state.mixpanel_client.clone();
        let flat_event = match self.to_flat_event() {
            Some(e) => e,
            None => {
                log::warn!(
                    "Skipping mixpanel forward - event: '{}', params: {}",
                    self.event.event,
                    self.event.params
                );
                return;
            }
        };

        tokio::spawn(async move {
            let token = mixpanel_client.token.clone();
            let event_name = flat_event.event.clone();

            let response = match mixpanel_client
                .client
                .post(&mixpanel_client.url)
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", token))
                .json(&flat_event)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    error!("Failed to send mixpanel request: {}", e);
                    return;
                }
            };

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_default();
                error!(
                    "Mixpanel proxy returned error {} for event '{}': {}",
                    status, event_name, error_text
                );
            } else {
                log::debug!("Successfully forwarded event '{}' to mixpanel", event_name);
            }
        });
    }

    pub async fn check_video_deduplication(
        &self,
        app_state: &AppState,
    ) -> Result<(), anyhow::Error> {
        if self.event.event == "video_upload_successful" {
            let params: Result<VideoUploadSuccessfulPayload, _> =
                serde_json::from_str(&self.event.params);

            let params = match params {
                Ok(params) => params,
                Err(e) => {
                    error!("Failed to parse video_upload_successful params: {e:?}");
                    return Err(anyhow::anyhow!(
                        "failed to parse video_upload_successful params: {e:?}"
                    ));
                }
            };

            #[cfg(feature = "local-bin")]
            {
                log::info!(
                    "Skipping durable video processing enqueue in local-bin for {}",
                    params.video_id
                );
            }

            #[cfg(not(feature = "local-bin"))]
            {
                let video_processing_pool = app_state.yral_redis_store_dragonfly.clone();
                let video_id = params.video_id;
                let post_id = params.post_id.clone();
                let publisher_user_id = params.publisher_user_id.to_text();
                let canister_id = Some(params.canister_id.to_text());

                let job = crate::video_processing::worker::new_upload_job(
                    video_id.clone(),
                    publisher_user_id,
                    post_id,
                    canister_id,
                );

                // Await the durable write so upload processing fails visibly instead of dropping NSFW handoff state.
                crate::video_processing::queue::enqueue_video_processing_job(
                    &video_processing_pool,
                    job,
                )
                .await?;
                log::info!("Durable video processing job queued for video_id: {video_id}");
            }
        }

        Ok(())
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
                        use yral_canisters_client::user_post_service::{
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
                                use yral_canisters_client::user_post_service::{
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
