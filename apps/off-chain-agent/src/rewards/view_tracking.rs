use crate::yral_auth::dragonfly::DragonflyPool;
use anyhow::{Context, Result};
use candid::Principal;
use redis::AsyncCommands;
use sha1::{Digest, Sha1};
use std::sync::Arc;

const LUA_ATOMIC_VIEW_SCRIPT: &str = r#"
    --!df flags=allow-undeclared-keys
    -- Atomic operation for view counting with config change handling
    -- This script only runs for logged-in users
    -- Using HSET to store both count and config_version in single hash
    local views_set = KEYS[1]  -- impressions:rewards:views:{video_id} (set of user IDs)
    local video_hash = KEYS[2]  -- impressions:rewards:video:{video_id} (hash with count & config_version)
    local config_version_key = KEYS[3]  -- impressions:rewards:config:version

    local user_id = ARGV[1]

    -- Always increment total_count_all (counts all views including duplicates)
    redis.call('HINCRBY', video_hash, 'total_count_all', 1)

    -- Get current global config version from Redis
    local current_global_version = redis.call('GET', config_version_key) or '1'

    -- Get video's stored config version from hash
    local video_config_version = redis.call('HGET', video_hash, 'config_version') or '0'

    -- If config changed for THIS video, reset its counter
    if video_config_version ~= current_global_version then
        -- Reset counter to 0 and update config version
        redis.call('HSET', video_hash, 'count', 0, 'config_version', current_global_version)
        -- Note: We do NOT delete the views_set, so users who already viewed cannot view again
    end

    -- Check if user already viewed (critical check for unique views)
    local added = redis.call('SADD', views_set, user_id)
    if added == 1 then
        -- New unique logged-in view: increment unique counters
        redis.call('HINCRBY', video_hash, 'count', 1)
        redis.call('HINCRBY', video_hash, 'total_count_loggedin', 1)
        return redis.call('HGET', video_hash, 'count')
    else
        return nil  -- Duplicate view (total_count_all already incremented above)
    end
"#;

fn calculate_script_sha(script: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(script.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

#[derive(Clone)]
pub struct ViewTracker {
    redis_store_pool: Arc<DragonflyPool>,
    script_sha: Option<String>,
}

impl ViewTracker {
    pub fn new(redis_store_pool: Arc<DragonflyPool>) -> Self {
        Self {
            redis_store_pool,
            script_sha: None,
        }
    }

    pub async fn load_lua_scripts(&mut self) -> Result<String> {
        let sha: String = self
            .redis_store_pool
            .execute_with_retry(|mut conn| async move {
                redis::cmd("SCRIPT")
                    .arg("LOAD")
                    .arg(LUA_ATOMIC_VIEW_SCRIPT)
                    .query_async(&mut conn)
                    .await
            })
            .await
            .context("Failed to load Lua script")?;

        // Verify the SHA matches what we expect
        let expected_sha = calculate_script_sha(LUA_ATOMIC_VIEW_SCRIPT);
        if sha != expected_sha {
            log::warn!(
                "Loaded script SHA {} doesn't match calculated SHA {}",
                sha,
                expected_sha
            );
        }

        self.script_sha = Some(sha.clone());
        log::info!("Loaded view tracking Lua script with SHA: {}", sha);
        Ok(sha)
    }

    pub async fn track_view(
        &self,
        video_id: &str,
        user_id: &Principal,
        is_logged_in: bool,
    ) -> Result<Option<u64>> {
        // Fast path for non-logged-in: just increment total_count_all
        if !is_logged_in {
            let video_hash_key = format!("impressions:rewards:video:{}", video_id);
            self.redis_store_pool
                .execute_with_retry(|mut conn| {
                    let key = video_hash_key.clone();
                    async move { conn.hincr::<_, _, _, ()>(&key, "total_count_all", 1).await }
                })
                .await?;

            return Ok(None);
        }

        // Logged-in path: use Lua script for atomic duplicate checking
        let views_set_key = format!("impressions:rewards:views:{}", video_id);
        let video_hash_key = format!("impressions:rewards:video:{}", video_id);
        let config_version_key = "impressions:rewards:config:version".to_string();
        let user_id_str = user_id.to_string();
        let sha = self.script_sha.clone();

        // Try to use the loaded script SHA, fallback to EVAL if not loaded
        let result: Option<u64> = self
            .redis_store_pool
            .execute_with_retry(|mut conn| {
                let views_key = views_set_key.clone();
                let video_key = video_hash_key.clone();
                let config_key = config_version_key.clone();
                let user = user_id_str.clone();
                let script_sha = sha.clone();

                async move {
                    if let Some(sha_str) = &script_sha {
                        // Use EVALSHA for better performance
                        let evalsha_result = redis::cmd("EVALSHA")
                            .arg(sha_str)
                            .arg(3) // number of keys
                            .arg(&views_key)
                            .arg(&video_key)
                            .arg(&config_key)
                            .arg(&user)
                            .query_async(&mut conn)
                            .await;

                        match evalsha_result {
                            Ok(result) => Ok(result),
                            Err(e) => {
                                // If script not in cache, use EVAL
                                log::warn!("EVALSHA failed, falling back to EVAL: {}", e);
                                redis::cmd("EVAL")
                                    .arg(LUA_ATOMIC_VIEW_SCRIPT)
                                    .arg(3)
                                    .arg(&views_key)
                                    .arg(&video_key)
                                    .arg(&config_key)
                                    .arg(&user)
                                    .query_async(&mut conn)
                                    .await
                            }
                        }
                    } else {
                        // Script not loaded, use EVAL
                        redis::cmd("EVAL")
                            .arg(LUA_ATOMIC_VIEW_SCRIPT)
                            .arg(3)
                            .arg(&views_key)
                            .arg(&video_key)
                            .arg(&config_key)
                            .arg(&user)
                            .query_async(&mut conn)
                            .await
                    }
                }
            })
            .await
            .context("Failed to execute view tracking script")?;

        Ok(result)
    }

    pub async fn get_view_count(&self, video_id: &str) -> Result<u64> {
        let video_hash_key = format!("impressions:rewards:video:{}", video_id);
        let count: Option<String> = self
            .redis_store_pool
            .execute_with_retry(|mut conn| {
                let key = video_hash_key.clone();
                async move { conn.hget(&key, "count").await }
            })
            .await?;

        Ok(count.and_then(|s| s.parse().ok()).unwrap_or(0))
    }

    pub async fn get_last_milestone(&self, video_id: &str) -> Result<u64> {
        let video_hash_key = format!("impressions:rewards:video:{}", video_id);
        let milestone: Option<String> = self
            .redis_store_pool
            .execute_with_retry(|mut conn| {
                let key = video_hash_key.clone();
                async move { conn.hget(&key, "last_milestone").await }
            })
            .await?;

        Ok(milestone.and_then(|s| s.parse().ok()).unwrap_or(0))
    }

    pub async fn set_last_milestone(&self, video_id: &str, milestone: u64) -> Result<()> {
        let video_hash_key = format!("impressions:rewards:video:{}", video_id);
        self.redis_store_pool
            .execute_with_retry(|mut conn| {
                let key = video_hash_key.clone();
                async move {
                    conn.hset::<_, _, _, ()>(&key, "last_milestone", milestone)
                        .await
                }
            })
            .await?;

        Ok(())
    }

    pub async fn get_total_count_loggedin(&self, video_id: &str) -> Result<u64> {
        let video_hash_key = format!("impressions:rewards:video:{}", video_id);
        let count: Option<String> = self
            .redis_store_pool
            .execute_with_retry(|mut conn| {
                let key = video_hash_key.clone();
                async move { conn.hget(&key, "total_count_loggedin").await }
            })
            .await?;

        Ok(count.and_then(|s| s.parse().ok()).unwrap_or(0))
    }

    pub async fn get_total_count_all(&self, video_id: &str) -> Result<u64> {
        let video_hash_key = format!("impressions:rewards:video:{}", video_id);
        let count: Option<String> = self
            .redis_store_pool
            .execute_with_retry(|mut conn| {
                let key = video_hash_key.clone();
                async move { conn.hget(&key, "total_count_all").await }
            })
            .await?;

        Ok(count.and_then(|s| s.parse().ok()).unwrap_or(0))
    }

    /// Get all video stats in a single Redis call
    pub async fn get_all_video_stats(&self, video_id: &str) -> Result<(u64, u64, u64, u64)> {
        let video_hash_key = format!("impressions:rewards:video:{}", video_id);

        // Get all fields in one call
        let data: std::collections::HashMap<String, String> = self
            .redis_store_pool
            .execute_with_retry(|mut conn| {
                let key = video_hash_key.clone();
                async move { conn.hgetall(&key).await }
            })
            .await?;

        let count = data.get("count").and_then(|s| s.parse().ok()).unwrap_or(0);
        let total_count_loggedin = data
            .get("total_count_loggedin")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let total_count_all = data
            .get("total_count_all")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let last_milestone = data
            .get("last_milestone")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        Ok((count, total_count_loggedin, total_count_all, last_milestone))
    }

    /// Get stats for multiple videos using Redis pipelining (batched)
    pub async fn get_bulk_video_stats(
        &self,
        video_ids: &[String],
    ) -> Result<std::collections::HashMap<String, (u64, u64, u64, u64)>> {
        if video_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        // Build pipeline with HGETALL for each video
        let mut pipe = redis::pipe();
        for video_id in video_ids {
            let video_hash_key = format!("impressions:rewards:video:{}", video_id);
            pipe.hgetall(&video_hash_key);
        }

        // Execute pipeline - all commands in single round-trip
        let results: Vec<std::collections::HashMap<String, String>> = self
            .redis_store_pool
            .execute_with_retry(|mut conn| {
                let p = pipe.clone();
                async move { p.query_async(&mut conn).await }
            })
            .await?;

        // Parse results
        let mut response = std::collections::HashMap::new();
        for (i, video_id) in video_ids.iter().enumerate() {
            if let Some(data) = results.get(i) {
                let count = data.get("count").and_then(|s| s.parse().ok()).unwrap_or(0);
                let total_count_loggedin = data
                    .get("total_count_loggedin")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                let total_count_all = data
                    .get("total_count_all")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                let last_milestone = data
                    .get("last_milestone")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);

                response.insert(
                    video_id.clone(),
                    (count, total_count_loggedin, total_count_all, last_milestone),
                );
            }
        }

        Ok(response)
    }
}
