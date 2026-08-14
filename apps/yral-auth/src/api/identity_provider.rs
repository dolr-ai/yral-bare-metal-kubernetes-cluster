use crate::{
    error::AuthErrorKind,
    kv::{KVStore, KVStoreImpl},
    oauth::SupportedOAuthProviders,
};

/// KV key for OAuth provider → user ID mapping.
/// Pure function.
pub fn oauth_lookup_key(provider: SupportedOAuthProviders, sub_id: &str) -> String {
    format!("{provider}-login-{sub_id}")
}

/// KV key for user existence marker.
fn user_existence_key(user_id: &str) -> String {
    format!("user:{user_id}")
}

/// Look up an existing user ID by OAuth provider + sub.
/// Returns `None` if the user doesn't exist yet.
pub async fn try_extract_user_id_from_oauth_sub(
    provider: SupportedOAuthProviders,
    kv: &KVStoreImpl,
    sub_id: &str,
    email: Option<&str>,
) -> Result<Option<String>, AuthErrorKind> {
    let key = oauth_lookup_key(provider, sub_id);
    let Some(user_id) = kv.read(key).await.map_err(AuthErrorKind::unexpected)? else {
        log::debug!("No user found for {provider} : {email:?}");
        return Ok(None);
    };

    log::debug!("Found user {user_id} for {provider} : {email:?}");

    if kv
        .has_key(user_existence_key(&user_id))
        .await
        .map_err(AuthErrorKind::unexpected)?
    {
        log::debug!("User {user_id} is valid for {provider} : {email:?}");
        Ok(Some(user_id))
    } else if email
        .map(|e| e.ends_with("@gobazzinga.io"))
        .unwrap_or(false)
    {
        log::debug!("User {user_id} is banned, but email {email:?} is whitelisted");
        Ok(None)
    } else {
        log::debug!("User {user_id} is banned for {provider} : {email:?}");
        Ok(None)
    }
}

/// Get or create a user ID for an OAuth login.
/// The OAuth `sub` is used directly as the user ID.
/// Thin wrapper — stores the mapping in KV.
pub async fn user_id_from_oauth_or_create(
    provider: SupportedOAuthProviders,
    kv: &KVStoreImpl,
    sub_id: &str,
    email: Option<&str>,
) -> Result<String, AuthErrorKind> {
    // Check if user already exists
    if let Some(existing_user_id) =
        try_extract_user_id_from_oauth_sub(provider, kv, sub_id, email).await?
    {
        return Ok(existing_user_id);
    }

    // New user — use the OAuth sub as the user ID
    let user_id = sub_id.to_string();

    // Store: OAuth provider+sub → user_id
    kv.write(oauth_lookup_key(provider, sub_id), user_id.clone())
        .await
        .map_err(|_| AuthErrorKind::unexpected("failed to associate id with oauth"))?;

    // Store: existence marker
    kv.write(user_existence_key(&user_id), "1".to_string())
        .await
        .map_err(|_| AuthErrorKind::unexpected("failed to write existence marker"))?;

    Ok(user_id)
}
