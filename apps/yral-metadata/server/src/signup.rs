use crate::{
    services::error_wrappers::{ErrorWrapper, OkWrapper},
    state::{AppState, SpacetimeClient},
    utils::error::{Error, Result},
};
use axum::{
    extract::{Path, State},
    Json,
};
use candid::Principal;
use email_address::EmailAddress;
use std::sync::Arc;
use types::{ApiResult, SetUserEmailMetadataReq, SetUserSignedInMetadataReq, UserMetadata};

#[utoipa::path(
    post,
    path = "/email/{user_principal}",
    params(
        ("user_principal" = String, Path, description = "User principal ID")
    ),
    request_body = SetUserEmailMetadataReq,
    responses(
        (status = 200, description = "Set user metadata successfully", body = OkWrapper<UserMetadata>),
        (status = 400, description = "Invalid request", body = ErrorWrapper<Error>),
        (status = 401, description = "Unauthorized", body = ErrorWrapper<Error>),
        (status = 500, description = "Internal server error", body = ErrorWrapper<Error>)
    )
)]
pub async fn set_user_email(
    State(state): State<Arc<AppState>>,
    Path(user_principal): Path<Principal>,
    Json(req): Json<SetUserEmailMetadataReq>,
) -> Result<Json<ApiResult<UserMetadata>>> {
    let result = set_user_email_impl(&state.spacetime, user_principal, req.payload.email).await?;
    Ok(Json(Ok(result)))
}

#[utoipa::path(
    post,
    path = "/signup/{user_principal}",
    params(
        ("user_principal" = String, Path, description = "User principal ID")
    ),
    request_body = SetUserSignedInMetadataReq,
    responses(
        (status = 200, description = "Set user metadata successfully", body = OkWrapper<UserMetadata>),
        (status = 401, description = "Unauthorized", body = ErrorWrapper<Error>),
        (status = 500, description = "Internal server error", body = ErrorWrapper<Error>)
    )
)]
pub async fn set_signup_datetime(
    State(state): State<Arc<AppState>>,
    Path(user_principal): Path<Principal>,
    Json(_req): Json<SetUserSignedInMetadataReq>,
) -> Result<Json<ApiResult<UserMetadata>>> {
    // Signup datetime is now handled by SpacetimeDB's accept_new_user_registration
    // reducer. This endpoint is kept for backward compatibility — it just returns
    // the current user metadata.
    let result = get_user_metadata_from_spacetime(&state.spacetime, user_principal).await?;
    Ok(Json(Ok(result)))
}

/// Set user email via SpacetimeDB `set_email` reducer.
pub async fn set_user_email_impl(
    spacetime: &SpacetimeClient,
    user_principal: Principal,
    email: String,
) -> Result<UserMetadata> {
    if !is_valid_email(&email) {
        return Err(Error::InvalidEmail(email));
    }

    spacetime
        .call_reducer(
            "set_email",
            serde_json::json!([user_principal.to_text(), email.clone()]),
        )
        .await?;

    get_user_metadata_from_spacetime(spacetime, user_principal).await
}

/// Fetch user metadata from SpacetimeDB.
async fn get_user_metadata_from_spacetime(
    spacetime: &SpacetimeClient,
    user_principal: Principal,
) -> Result<UserMetadata> {
    let result: serde_json::Value = spacetime
        .call_procedure(
            "get_user_profile_details",
            serde_json::json!([user_principal.to_text()]),
        )
        .await?;

    // Parse Option from REST API: [0, {...}] = Some, [1, []] = None
    let profile = result
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_u64())
        .filter(|&tag| tag == 0)
        .and_then(|_| result.as_array()?.get(1));

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

        Ok(UserMetadata {
            user_canister_id: Principal::anonymous(),
            user_name: username.unwrap_or_default(),
            notification_key: None,
            email,
            signup_at: None,
            is_migrated: true,
        })
    } else {
        Err(Error::Unknown(format!(
            "User `{}` not found",
            user_principal.to_text()
        )))
    }
}

fn is_valid_email(email: &str) -> bool {
    EmailAddress::is_valid(email)
}
