use futures::Future;
use serde::{Deserialize, Serialize};

/// Trait for accessing authenticated user info.
/// Implemented by `AuthSession` (state crate). This lets utils functions
/// accept the auth type without a circular dependency on `state`.
pub trait UserAuthInfo {
    fn user_id(&self) -> String;
    fn user_canister(&self) -> String;
    fn user_identity(&self) -> crate::user_identity::UserIdentity;
}

pub mod ab_testing;
pub mod client_ip;
pub mod health;
pub mod host;
pub mod icon;
pub mod local_storage;
pub mod ml_feed;
pub mod notifications;
pub mod posts;
pub mod route;
pub mod time;
pub mod types;
pub mod user_identity;
pub mod username_generator;
pub mod web;

/// Navigation category for bottom nav tracking and cookie logic.
/// Moved here from mixpanel_events.rs (which has been removed).
#[derive(serde::Serialize, serde::Deserialize, Debug, Default, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BottomNavigationCategory {
    #[default]
    Menu,
    Wallet,
}

impl std::convert::TryFrom<String> for BottomNavigationCategory {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.contains("/wallet/") {
            return Ok(BottomNavigationCategory::Wallet);
        }

        match value.as_str() {
            "/wallet" => Ok(BottomNavigationCategory::Wallet),
            "/menu" => Ok(BottomNavigationCategory::Menu),
            _ => Err(()),
        }
    }
}

/// Login provider kind, used for login flow processing state.
/// Moved here from event_streaming/events.rs (which has been removed).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderKind {
    #[cfg(any(feature = "oauth-ssr", feature = "oauth-hydrate"))]
    YralAuth,
}
/// Wrapper for PartialEq that always returns false
/// this is currently only used for resources
/// this does not provide a sane implementation of PartialEq
#[derive(Clone, Serialize, Deserialize)]
pub struct MockPartialEq<T>(pub T);

impl<T> PartialEq for MockPartialEq<T> {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}

use std::{
    convert::Infallible,
    fmt::{self, Display, Formatter},
    str::FromStr,
};

use consts::CF_STREAM_BASE;

pub fn bg_url(uid: impl Display) -> String {
    format!("{CF_STREAM_BASE}/{uid}/thumbnails/thumbnail.jpg")
}

pub fn stream_url(uid: impl Display) -> String {
    format!("{CF_STREAM_BASE}/{uid}/manifest/video.m3u8")
}

pub fn mp4_url(uid: impl Display) -> String {
    format!("{CF_STREAM_BASE}/{uid}/downloads/default.mp4")
}

#[cfg(not(feature = "hydrate"))]
pub fn send_wrap<Fut: Future + Send>(
    t: Fut,
) -> impl Future<Output = <Fut as Future>::Output> + Send {
    t
}

/// Wraps a specific future that is not `Send` when `hydrate` feature is enabled
/// the future must be `Send` when `ssr` is enabled
/// use only when necessary (usually inside resources)
/// if you get a Send related error inside an Action, it probably makes more
/// sense to use `Action::new_local` or `Action::new_unsync`
#[cfg(feature = "hydrate")]
pub fn send_wrap<Fut: Future>(t: Fut) -> impl Future<Output = <Fut as Future>::Output> + Send {
    send_wrapper::SendWrapper::new(t)
}

/// A user identifier — either a username or a raw user_id string.
/// In the IC era, this was `UsernameOrPrincipal` (username vs IC Principal).
/// Now that all user IDs are strings (JWT sub or UUID), this is just a
/// string alias — but kept as a type for backwards compatibility in routes.
pub type UsernameOrPrincipal = String;
