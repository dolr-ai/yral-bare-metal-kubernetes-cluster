//! Lightweight user-identity view model used by non-profile pages
//! (wallet, notifs, settings, menu, analytics) to display the signed-in
//! user's avatar / username / principal without depending on the legacy
//! `ProfileDetails` profile-feature type.

use candid::Principal;
use serde::{Deserialize, Serialize};
use yral_canisters_common::utils::profile::{propic_from_principal, ProfileDetails};
use yral_username_gen::random_username_from_principal;

/// Display-name length cap mirroring `ProfileDetails` semantics.
const USERNAME_MAX_LEN: usize = 29;

/// Minimal user-identity struct consumed by wallet / notifs / settings /
/// menu / analytics. Derived from `ProfileDetails` at the boundary so the
/// app is decoupled from the profile-feature type.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UserIdentity {
    pub username: Option<String>,
    pub profile_pic: Option<String>,
    pub display_name: Option<String>,
    pub principal: Principal,
}

impl From<ProfileDetails> for UserIdentity {
    fn from(details: ProfileDetails) -> Self {
        Self {
            username: details.username,
            profile_pic: details.profile_pic,
            display_name: details.display_name,
            principal: details.principal,
        }
    }
}

impl UserIdentity {
    /// Username, or the textual principal if no username is set.
    /// Use this for URLs.
    pub fn username_or_principal(&self) -> String {
        self.username
            .clone()
            .unwrap_or_else(|| self.principal())
    }

    /// Username, or a deterministic random fallback username.
    /// WARN: do not use for URLs; use `username_or_principal` instead.
    pub fn username_or_fallback(&self) -> String {
        self.username
            .clone()
            .unwrap_or_else(|| random_username_from_principal(self.principal, USERNAME_MAX_LEN))
    }

    pub fn principal(&self) -> String {
        self.principal.to_text()
    }

    pub fn display_name_or_fallback(&self) -> String {
        self.display_name
            .clone()
            .unwrap_or_else(|| self.username_or_fallback())
    }

    pub fn profile_pic_or_random(&self) -> String {
        let propic = self.profile_pic.clone().unwrap_or_default();
        if !propic.is_empty() {
            return propic;
        }
        propic_from_principal(self.principal)
    }
}