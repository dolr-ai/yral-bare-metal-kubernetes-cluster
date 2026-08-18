#![allow(dead_code)]

#[cfg(feature = "ssr")]
pub mod server_impl;

use leptos::prelude::*;
use leptos::{server, server_fn::codec::Json};
use serde::{Deserialize, Serialize};

/// Anonymous identity for non-logged-in users.
/// In the JWT era, this is just an empty placeholder — the client
/// gets an anonymous session from yral-auth without IC identity delegation.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AnonymousIdentity {
    pub refresh_token: String,
}

/// Generate an anonymous identity if refresh token is not set
#[server]
pub async fn generate_anonymous_identity_if_required(
) -> Result<Option<AnonymousIdentity>, ServerFnError> {
    server_impl::generate_anonymous_identity_if_required_impl().await
}

/// this server function is purely a side effect and only sets the refresh token cookie
#[server(endpoint = "set_anonymous_identity_cookie", input = Json, output = Json)]
pub async fn set_anonymous_identity_cookie(refresh_jwt: String) -> Result<(), ServerFnError> {
    server_impl::set_anonymous_identity_cookie_impl(refresh_jwt).await
}

/// Extract the user identity from the ID_TOKEN and REFRESH_TOKEN cookies.
/// Returns None if not logged in.
/// In the JWT era, this returns (user_id, id_token, refresh_token) instead
/// of the old DelegatedIdentityWire.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExtractedIdentity {
    pub user_id: String,
    pub id_token: String,
    pub refresh_token: String,
}

#[server(endpoint = "extract_identity", input = Json, output = Json)]
pub async fn extract_identity() -> Result<Option<ExtractedIdentity>, ServerFnError> {
    server_impl::extract_identity_impl().await
}

/// Get the current user's identifier (JWT sub claim) from the ID_TOKEN.
/// Returns None for anonymous users.
#[server(endpoint = "get_user_identifier", input = Json, output = Json)]
pub async fn get_user_identifier() -> Result<Option<String>, ServerFnError> {
    server_impl::get_user_identifier_impl().await
}

/// Get the current id_token for SpacetimeDB authentication.
/// Reads the ID_TOKEN cookie. If < 1h remaining, refreshes via REFRESH_TOKEN.
/// Returns None for anonymous users.
#[server(endpoint = "get_id_token", input = Json, output = Json)]
pub async fn get_id_token() -> Result<Option<String>, ServerFnError> {
    server_impl::get_id_token_impl().await
}

/// Force-refresh the id_token using the httpOnly refresh_token cookie.
/// Returns None if not logged in.
#[server(endpoint = "refresh_id_token", input = Json, output = Json)]
pub async fn refresh_id_token() -> Result<Option<String>, ServerFnError> {
    server_impl::refresh_id_token_impl().await
}

/// Logout — clears the identity. Returns Ok(()) on success.
#[server]
pub async fn logout_identity() -> Result<(), ServerFnError> {
    server_impl::logout_identity_impl().await
}