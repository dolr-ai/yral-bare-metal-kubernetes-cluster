//! User profile and identity types for yral-legacy.
//!
//! `ProfileDetails` is the full profile struct (fetched from SpacetimeDB).
//! `UserIdentity` is a lightweight view model used by non-profile pages
//! (wallet, notifs, settings, menu, analytics).

use candid::Principal;
use serde::{Deserialize, Serialize};
use username_gen::random_username_from_principal;

/// Display-name length cap.
const USERNAME_MAX_LEN: usize = 29;

/// Total number of GobGob NFTs used for fallback profile pictures.
const GOBGOB_TOTAL_COUNT: u32 = 18557;
const GOBGOB_PROPIC_URL: &str = "https://imagedelivery.net/abXI9nS4DYYtyR1yFFtziA/gob.";

fn index_from_principal(principal: Principal) -> u32 {
    let hash_value = crc32fast::hash(principal.as_slice());
    (hash_value % GOBGOB_TOTAL_COUNT) + 1
}

/// Deterministic fallback profile picture URL derived from a principal.
pub fn propic_from_principal(principal: Principal) -> String {
    let index = index_from_principal(principal);
    format!("{GOBGOB_PROPIC_URL}{index}/public")
}

/// Full user profile, populated from SpacetimeDB.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProfileDetails {
    pub username: Option<String>,
    pub lifetime_earnings: u64,
    pub followers_cnt: u64,
    pub following_cnt: u64,
    pub profile_pic: Option<String>,
    pub display_name: Option<String>,
    pub principal: Principal,
    pub user_canister: Principal,
    pub hots: u64,
    pub nots: u64,
    pub bio: Option<String>,
    pub website_url: Option<String>,
    pub caller_follows_user: Option<bool>,
    pub user_follows_caller: Option<bool>,
}

impl ProfileDetails {
    pub fn username_or_principal(&self) -> String {
        self.username.clone().unwrap_or_else(|| self.principal())
    }

    /// Username, or a consistent random username.
    /// WARN: do not use this method for URLs
    /// use `username_or_principal` instead
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

/// Minimal user-identity struct consumed by wallet / notifs / settings /
/// menu / analytics. Derived from `ProfileDetails` at the boundary.
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