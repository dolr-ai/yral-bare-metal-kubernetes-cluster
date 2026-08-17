use crate::{api::ai_accounts::server_fn::codec::Json, kv::KVStore};
#[cfg(feature = "ssr")]
use crate::{context::server::ServerCtx, utils::user_id::generate_user_id};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

pub const MAX_AI_ACCOUNTS: u8 = 100;

/// KV key for an AI account slot: `{owner_user_id}-ai-account-{slot}`.
#[cfg(feature = "ssr")]
fn ai_account_slot_key(owner_user_id: &str, slot: u8) -> String {
    format!("{owner_user_id}-ai-account-{slot}")
}

/// KV key for reverse lookup: `ai-account:{ai_account_id}` → owner user ID.
#[cfg(feature = "ssr")]
fn ai_account_reverse_lookup_key(ai_account_id: &str) -> String {
    format!("ai-account:{ai_account_id}")
}

/// Response containing the AI account's user ID.
/// The mobile app uses this ID to authenticate as the AI account
/// (yral-auth mints a JWT with `sub = ai_account_id`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIAccountResponse {
    pub ai_account_id: String,
}

#[server(endpoint = "create_ai_account", input=Json, output=Json)]
pub async fn create_ai_account(user_id: String) -> Result<AIAccountResponse, ServerFnError> {
    let ctx = expect_context::<std::sync::Arc<ServerCtx>>();

    // AI accounts cannot create other AI accounts
    let reverse_key = ai_account_reverse_lookup_key(&user_id);
    if ctx.kv_store.has_key(reverse_key).await.unwrap_or(false) {
        return Err(ServerFnError::new(
            "AI accounts cannot create other AI accounts",
        ));
    }

    // Find the next available slot
    let mut next_slot: Option<u8> = None;
    for slot in 1..=MAX_AI_ACCOUNTS {
        let key = ai_account_slot_key(&user_id, slot);
        match ctx.kv_store.has_key(key).await {
            Ok(exists) => {
                if !exists && next_slot.is_none() {
                    next_slot = Some(slot);
                }
            }
            Err(e) => {
                return Err(ServerFnError::new(format!("Storage error: {}", e)));
            }
        }
    }

    let slot = match next_slot {
        Some(s) => s,
        None => {
            return Err(ServerFnError::new(format!(
                "Maximum of {} AI accounts already created",
                MAX_AI_ACCOUNTS
            )));
        }
    };

    // Generate a new AI account ID (UUID)
    let ai_account_id = generate_user_id();

    // Store: slot key → ai_account_id
    let slot_key = ai_account_slot_key(&user_id, slot);
    if let Err(e) = ctx.kv_store.write(slot_key, ai_account_id.clone()).await {
        return Err(ServerFnError::new(format!("Storage error: {}", e)));
    }

    // Store: reverse lookup → owner user_id
    let reverse_key = ai_account_reverse_lookup_key(&ai_account_id);
    if let Err(e) = ctx.kv_store.write(reverse_key, user_id).await {
        return Err(ServerFnError::new(format!("Storage error: {}", e)));
    }

    // Store: existence marker for the AI account (so token grant works)
    let existence_key = format!("user:{ai_account_id}");
    if let Err(e) = ctx.kv_store.write(existence_key, "1".to_string()).await {
        return Err(ServerFnError::new(format!("Storage error: {}", e)));
    }

    Ok(AIAccountResponse { ai_account_id })
}

/// Get all AI account IDs for a given user.
/// Called from `server_impl.rs` during token grant to populate `ext_ai_account_ids`.
#[cfg(feature = "ssr")]
pub async fn get_ai_account_ids_for_user(
    ctx: &ServerCtx,
    user_id: &str,
) -> Result<Vec<String>, String> {
    let mut ai_account_ids = Vec::new();

    for slot in 1..=MAX_AI_ACCOUNTS {
        let key = ai_account_slot_key(user_id, slot);
        match ctx.kv_store.read(key).await {
            Ok(Some(ai_account_id)) => {
                ai_account_ids.push(ai_account_id);
            }
            Ok(None) => continue,
            Err(e) => return Err(e.to_string()),
        }
    }

    Ok(ai_account_ids)
}
