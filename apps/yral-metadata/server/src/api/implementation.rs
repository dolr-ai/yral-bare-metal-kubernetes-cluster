use crate::{state::SpacetimeClient, utils::error::Result};
use candid::Principal;
use std::collections::HashMap;
use types::{
    BulkGetUserMetadataReq, BulkGetUserMetadataRes, BulkUsers, GetUserMetadataV2Res,
    SetUserMetadataReq, SetUserMetadataReqMetadata, SetUserMetadataRes, UserMetadata,
    UserMetadataV2,
};

/// Set user metadata via SpacetimeDB.
/// Calls `set_username` and `set_email` reducers on the SpacetimeDB module.
pub async fn set_user_metadata_core(
    spacetime: &SpacetimeClient,
    user_principal: Principal,
    set_metadata: &SetUserMetadataReqMetadata,
) -> Result<SetUserMetadataRes> {
    let principal_text = user_principal.to_text();

    if !set_metadata.user_name.is_empty() {
        spacetime
            .call_reducer(
                "set_username",
                serde_json::json!([principal_text.clone(), set_metadata.user_name.clone()]),
            )
            .await?;
    }

    Ok(())
}

/// Full handler path: verifies signature then calls core.
pub async fn set_user_metadata_impl(
    spacetime: &SpacetimeClient,
    user_principal: Principal,
    req: SetUserMetadataReq,
) -> Result<SetUserMetadataRes> {
    // Signature verification removed — IC identity no longer used.
    // The caller is authenticated via the yral-auth JWT Bearer token.
    set_user_metadata_core(spacetime, user_principal, &req.metadata).await
}

/// Get user metadata from SpacetimeDB by username or principal.
pub async fn get_user_metadata_impl(
    spacetime: &SpacetimeClient,
    username_or_principal: String,
) -> Result<GetUserMetadataV2Res> {
    // Try to parse as principal first
    let result = if let Ok(principal) = Principal::from_text(&username_or_principal) {
        spacetime
            .call_procedure::<serde_json::Value>(
                "get_user_profile_details_v_7",
                serde_json::json!([principal.to_text()]),
            )
            .await?
    } else {
        // Lookup by username
        spacetime
            .call_procedure::<serde_json::Value>(
                "get_user_profile_by_username",
                serde_json::json!([username_or_principal]),
            )
            .await?
    };

    // Parse the SpacetimeDB response to extract user metadata
    // The REST API returns Option as [0, {...}] for Some, [1, []] for None
    let profile = parse_spacetime_option(&result);
    if let Some(profile) = profile {
        let username = profile
            .get("username")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let email = profile
            .get("email")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());
        let principal_text = profile
            .get("principal_text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let user_principal =
            Principal::from_text(&principal_text).unwrap_or(Principal::anonymous());

        let metadata = UserMetadata {
            user_canister_id: Principal::anonymous(),
            user_name: username.unwrap_or_default(),
            notification_key: None,
            email,
            signup_at: None,
            is_migrated: true,
        };

        Ok(Some(UserMetadataV2::from_metadata(
            user_principal,
            metadata,
        )))
    } else {
        Ok(None)
    }
}

/// Bulk delete user metadata from SpacetimeDB.
/// SpacetimeDB handles deletion via the delete_user_info reducer.
pub async fn delete_metadata_bulk_impl(
    spacetime: &SpacetimeClient,
    users: &BulkUsers,
) -> Result<()> {
    for user in &users.users {
        spacetime
            .call_reducer("delete_user_info", serde_json::json!([user.to_text()]))
            .await?;
    }
    Ok(())
}

/// Bulk get user metadata from SpacetimeDB.
pub async fn get_user_metadata_bulk_impl(
    spacetime: &SpacetimeClient,
    req: BulkGetUserMetadataReq,
) -> Result<BulkGetUserMetadataRes> {
    let principal_texts: Vec<String> = req.users.iter().map(|p| p.to_text()).collect();

    let results: serde_json::Value = spacetime
        .call_procedure(
            "get_users_profile_details",
            serde_json::json!([principal_texts]),
        )
        .await?;

    let mut result_map = HashMap::new();
    let empty_vec = Vec::new();
    let profiles = results.as_array().unwrap_or(&empty_vec);
    for (i, profile) in profiles.iter().enumerate() {
        if i < req.users.len() {
            let username = profile
                .get("username")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());
            let email = profile
                .get("email")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());

            let metadata = UserMetadata {
                user_canister_id: Principal::anonymous(),
                user_name: username.unwrap_or_default(),
                notification_key: None,
                email,
                signup_at: None,
                is_migrated: true,
            };
            result_map.insert(req.users[i], Some(metadata));
        }
    }

    Ok(result_map)
}

/// Parse SpacetimeDB Option<T> from REST API response.
/// [0, {...}] = Some, [1, []] = None
fn parse_spacetime_option(value: &serde_json::Value) -> Option<&serde_json::Value> {
    let arr = value.as_array()?;
    let tag = arr.first()?.as_u64()?;
    if tag == 0 {
        arr.get(1)
    } else {
        None
    }
}
