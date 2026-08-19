use crate::{api::ai_accounts::server_fn::codec::Json, kv::KVStore};
#[cfg(feature = "ssr")]
use crate::{context::server::ServerCtx, utils::user_id::generate_user_id};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// KV key for a user's AI account list: `{owner_user_id}-ai-accounts` → JSON array of AI account IDs.
#[cfg(feature = "ssr")]
fn ai_account_list_key(owner_user_id: &str) -> String {
    format!("{owner_user_id}-ai-accounts")
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

    // Read the existing AI account list (empty if first account)
    let list_key = ai_account_list_key(&user_id);
    let mut ai_account_ids: Vec<String> = match ctx.kv_store.read(list_key.clone()).await {
        Ok(Some(json)) => serde_json::from_str(&json)
            .map_err(|e| ServerFnError::new(format!("Storage error: {}", e)))?,
        Ok(None) => Vec::new(),
        Err(e) => return Err(ServerFnError::new(format!("Storage error: {}", e))),
    };

    // Generate a new AI account ID (UUID)
    let ai_account_id = generate_user_id();

    // Append the new AI account to the list and persist
    ai_account_ids.push(ai_account_id.clone());
    let updated_json = serde_json::to_string(&ai_account_ids)
        .map_err(|e| ServerFnError::new(format!("Storage error: {}", e)))?;
    if let Err(e) = ctx.kv_store.write(list_key, updated_json).await {
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
    let list_key = ai_account_list_key(user_id);
    match ctx.kv_store.read(list_key).await {
        Ok(Some(json)) => serde_json::from_str(&json).map_err(|e| e.to_string()),
        Ok(None) => Ok(Vec::new()),
        Err(e) => Err(e.to_string()),
    }
}
