use crate::yral_auth::dragonfly::DragonflyPool;
use anyhow::{Context, Result};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RewardTokenType {
    #[default]
    Btc,
    Dolr,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum RewardMode {
    InrAmount { amount_per_view_inr: f64 },
    DirectTokenE8s { amount_per_milestone_e8s: u64 },
}

impl Default for RewardMode {
    fn default() -> Self {
        RewardMode::InrAmount {
            amount_per_view_inr: 0.037,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RewardConfigV1 {
    pub reward_amount_inr: f64,
    pub view_milestone: u64,
    pub min_watch_duration: f64,
    pub fraud_threshold: usize,
    pub shadow_ban_duration: u64,
    pub config_version: u64,
    #[serde(default)]
    pub reward_token: RewardTokenType,
}

impl From<RewardConfigV1> for RewardConfig {
    fn from(v1: RewardConfigV1) -> Self {
        Self {
            reward_mode: RewardMode::InrAmount {
                amount_per_view_inr: v1.reward_amount_inr,
            },
            view_milestone: v1.view_milestone,
            min_watch_duration: v1.min_watch_duration,
            fraud_threshold: v1.fraud_threshold,
            shadow_ban_duration: v1.shadow_ban_duration,
            config_version: v1.config_version,
            reward_token: v1.reward_token,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RewardConfig {
    pub reward_mode: RewardMode,
    pub view_milestone: u64,
    pub min_watch_duration: f64,
    pub fraud_threshold: usize,
    pub shadow_ban_duration: u64,
    pub config_version: u64,
    pub reward_token: RewardTokenType,
}

impl Default for RewardConfig {
    fn default() -> Self {
        Self {
            reward_mode: RewardMode::default(),
            view_milestone: 100,
            min_watch_duration: 3.0,
            fraud_threshold: 5,
            shadow_ban_duration: 3600,
            config_version: 1,
            reward_token: RewardTokenType::default(),
        }
    }
}

/// Get the current reward configuration from Dragonfly
pub async fn get_config(dragonfly_redis_store_pool: &Arc<DragonflyPool>) -> Result<RewardConfig> {
    let config_key = "impressions:rewards:config".to_string();

    let config_str: Option<String> = dragonfly_redis_store_pool
        .execute_with_retry(|mut conn| {
            let key = config_key.clone();
            async move { conn.get(&key).await }
        })
        .await
        .context("Failed to get config from Dragonfly")?;

    match config_str {
        Some(s) => {
            let config = match serde_json::from_str::<RewardConfig>(&s) {
                Ok(v2_config) => v2_config,
                Err(_) => {
                    log::info!("Attempting to migrate V1 config to V2");
                    let v1_config: RewardConfigV1 = serde_json::from_str(&s)
                        .context("Failed to deserialize as both V2 and V1 config")?;

                    let v2_config: RewardConfig = v1_config.into();

                    log::info!("Persisting migrated V2 config back to Redis");
                    let v2_json = serde_json::to_string(&v2_config)?;

                    dragonfly_redis_store_pool
                        .execute_with_retry(|mut conn| {
                            let key = config_key.clone();
                            let json = v2_json.clone();
                            async move { conn.set::<_, _, ()>(&key, json).await }
                        })
                        .await
                        .context("Failed to persist migrated V2 config")?;

                    v2_config
                }
            };

            Ok(config)
        }
        None => {
            // If no config exists, initialize with default
            let default_config = RewardConfig::default();
            initialize_config(dragonfly_redis_store_pool, &default_config).await?;
            Ok(default_config)
        }
    }
}

/// Update the reward configuration in Dragonfly
pub async fn update_config(
    dragonfly_redis_store_pool: &Arc<DragonflyPool>,
    new_config: RewardConfig,
) -> Result<()> {
    let config_version_key = "impressions:rewards:config:version".to_string();
    let config_key = "impressions:rewards:config".to_string();

    // Atomically increment the global config version
    let version: u64 = dragonfly_redis_store_pool
        .execute_with_retry(|mut conn| {
            let key = config_version_key.clone();
            async move { conn.incr(&key, 1).await }
        })
        .await
        .context("Failed to increment config version")?;

    // Update config with new version
    let mut config = new_config;
    config.config_version = version;

    let config_json = serde_json::to_string(&config)?;

    dragonfly_redis_store_pool
        .execute_with_retry(|mut conn| {
            let key = config_key.clone();
            let json = config_json.clone();
            async move { conn.set::<_, _, ()>(&key, json).await }
        })
        .await
        .context("Failed to store config in Dragonfly")?;

    log::info!("Updated reward config to version {}: {:?}", version, config);
    Ok(())
}

/// Get the current config version from Dragonfly
#[cfg(test)]
pub async fn get_config_version(dragonfly_pool: &Arc<DragonflyPool>) -> Result<u64> {
    let config_version_key = "impressions:rewards:config:version".to_string();

    let version: Option<u64> = dragonfly_pool
        .execute_with_retry(|mut conn| {
            let key = config_version_key.clone();
            async move { conn.get(&key).await }
        })
        .await
        .context("Failed to get config version")?;

    Ok(version.unwrap_or(1))
}

/// Initialize config in Dragonfly if it doesn't exist
async fn initialize_config(
    dragonfly_redis_store_pool: &Arc<DragonflyPool>,
    config: &RewardConfig,
) -> Result<()> {
    let config_version_key = "impressions:rewards:config:version".to_string();
    let config_key = "impressions:rewards:config".to_string();
    let version = config.config_version;

    // Set initial version if not exists
    let _: bool = dragonfly_redis_store_pool
        .execute_with_retry(|mut conn| {
            let key = config_version_key.clone();
            async move { conn.set_nx(&key, version).await }
        })
        .await
        .context("Failed to initialize config version")?;

    // Set config
    let config_json = serde_json::to_string(config)?;
    let _: bool = dragonfly_redis_store_pool
        .execute_with_retry(|mut conn| {
            let key = config_key.clone();
            let json = config_json.clone();
            async move { conn.set_nx(&key, json).await }
        })
        .await
        .context("Failed to initialize config")?;

    log::info!("Initialized reward config with defaults: {:?}", config);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_default_config() {
        let config = RewardConfig::default();
        assert!(matches!(
            config.reward_mode,
            RewardMode::InrAmount {
                amount_per_view_inr
            } if (amount_per_view_inr - 0.037).abs() < 0.001
        ));
        assert_eq!(config.view_milestone, 100);
        assert_eq!(config.min_watch_duration, 3.0);
        assert_eq!(config.fraud_threshold, 5);
        assert_eq!(config.shadow_ban_duration, 3600);
        assert_eq!(config.config_version, 1);
    }

    /// Test serialization and deserialization of both reward modes
    #[tokio::test]
    async fn test_reward_mode_serialization() {
        // Test InrAmount mode
        let inr_mode = RewardMode::InrAmount {
            amount_per_view_inr: 12.5,
        };
        let json = serde_json::to_string(&inr_mode).unwrap();
        let deserialized: RewardMode = serde_json::from_str(&json).unwrap();
        assert_eq!(inr_mode, deserialized);

        // Test DirectTokenE8s mode
        let e8s_mode = RewardMode::DirectTokenE8s {
            amount_per_milestone_e8s: 10_000_000,
        };
        let json = serde_json::to_string(&e8s_mode).unwrap();
        let deserialized: RewardMode = serde_json::from_str(&json).unwrap();
        assert_eq!(e8s_mode, deserialized);
    }

    /// Test V1 to V2 conversion
    #[tokio::test]
    async fn test_v1_conversion() {
        let v1 = RewardConfigV1 {
            reward_amount_inr: 15.0,
            view_milestone: 100,
            min_watch_duration: 3.0,
            fraud_threshold: 5,
            shadow_ban_duration: 3600,
            config_version: 1,
            reward_token: RewardTokenType::Btc,
        };

        let v2: RewardConfig = v1.into();

        assert!(matches!(
            v2.reward_mode,
            RewardMode::InrAmount {
                amount_per_view_inr
            } if (amount_per_view_inr - 15.0).abs() < 0.001
        ));
        assert_eq!(v2.view_milestone, 100);
        assert_eq!(v2.reward_token, RewardTokenType::Btc);
    }
}
