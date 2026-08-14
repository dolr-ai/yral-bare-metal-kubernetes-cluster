//! User info service — migrated from the IC `user_info_service` canister
//! (`ivkka-7qaaa-aaaas-qbg3q-cai`).
//!
//! ## Tables
//! - `user_profiles`: keyed by `principal_text`, stores profile/follow data
//! - `user_follows`: keyed by `(follower_text, followee_text)`, bidirectional
//!
//! ## Procedures (reads, return typed data)
//! - `get_profile_details_v4(principal_text) -> Option<UserProfileDetailsV4>`
//! - `get_user_profile_details_v7(principal_text) -> Option<UserProfileDetailsV7>`
//! - `get_users_profile_details(principal_texts) -> Vec<UserProfileDetailsV7>`
//! - `get_followers(principal_text, limit, cursor) -> FollowersPage`
//! - `get_following(principal_text, limit, cursor) -> FollowingPage`
//!
//! ## Reducers (writes)
//! - `register_new_user()` — idempotent, creates empty profile for `ctx.sender()`
//! - `follow_user(followee_text)` — bidirectional follow
//! - `unfollow_user(followee_text)` — symmetric
//! - `update_profile_details(bio, website_url, profile_pic_url)` — sender only
//! - `update_profile_details_v2(bio, website_url, profile_picture)` — sender only
//! - `update_profile_ai_influencer_status(principal_text, is_ai)` — admin only
//! - `accept_new_user_registration_v2(new_principal, authenticated, main_account)` — register/bot
//! - `delete_user_info(principal_to_delete)` — cascade delete
//! - `update_user_last_access_time()` — sender only
//! - `update_profile_picture_nsfw_info(principal_text, nsfw_info)` — admin only
//! - `change_subscription_plan(principal_text, plan)` — admin only
//! - `add_pro_plan_free_video_credits(principal_text, credits)` — admin only
//! - `remove_pro_plan_free_video_credits(principal_text, credits)` — admin only
//! - `upsert_user_profile_batch(profiles)` — admin only, backfill
//! - `upsert_user_follow_batch(follows)` — admin only, backfill

use spacetimedb::{ProcedureContext, ReducerContext, SpacetimeType, Table, Timestamp};

// ─────────────────────────────────────────────────────────────────────────
// Tables
// ─────────────────────────────────────────────────────────────────────────

/// A user profile. Mirrors the IC `UserInfo` struct.
///
/// Primary key: `principal_text` (the IC Principal as text, e.g.
/// `w4rip-qiaaa-aaaas-ab5...`). This is the same value stored in
/// `posts.creator_principal_text`.
#[spacetimedb::table(accessor = user_profiles, public)]
#[derive(Clone)]
pub struct UserProfile {
    #[primary_key]
    pub principal_text: String,
    pub bio: String,
    pub website_url: String,
    pub profile_picture_url: String,
    pub followers_count: u64,
    pub following_count: u64,
    pub subscription_plan: SubscriptionPlan,
    pub is_ai_influencer: bool,
    pub is_nsfw: bool,
    pub nsfw_ec: String,
    pub nsfw_gore: String,
    pub csam_detected: bool,
    pub last_access_time: Timestamp,
    pub account_type: UserAccountType,
    /// Display name from the yral-metadata service (Redis/Dragonfly).
    /// Backfilled from `UserMetadata.user_name`. `None` if not yet backfilled.
    #[default(None::<String>)]
    pub username: Option<String>,
    /// Email from the yral-metadata service (Redis/Dragonfly).
    /// Backfilled from `UserMetadata.email`. `None` if not set.
    #[default(None::<String>)]
    pub email: Option<String>,
}

/// A follow relationship. Primary key is a composite key
/// `{follower_text}::{followee_text}` for uniqueness. Both directions
/// are indexed for efficient "who follows X" and "who does X follow"
/// queries.
#[spacetimedb::table(accessor = user_follows, public)]
#[derive(Clone)]
pub struct UserFollow {
    #[primary_key]
    pub key: String, // "{follower}::{followee}"
    #[index(btree)]
    pub follower_text: String,
    #[index(btree)]
    pub followee_text: String,
}

// ─────────────────────────────────────────────────────────────────────────
// Types (SpacetimeType — serialized as typed JSON for REST clients)
// ─────────────────────────────────────────────────────────────────────────

/// Pro subscription plan with video credits. Mirrors the IC
/// `YralProSubscription` struct.
#[derive(SpacetimeType, Clone, Debug, PartialEq, Eq, Copy)]
pub struct YralProSubscription {
    pub free_video_credits_left: u32,
    pub total_video_credits_alloted: u32,
}

impl Default for YralProSubscription {
    fn default() -> Self {
        YralProSubscription {
            free_video_credits_left: 30,
            total_video_credits_alloted: 30,
        }
    }
}

#[derive(SpacetimeType, Clone, Debug, PartialEq, Eq, Copy, Default)]
pub enum SubscriptionPlan {
    #[default]
    Free,
    Pro(YralProSubscription),
}

/// NSFW information for content moderation. Mirrors the IC `NSFWInfo` struct.
#[derive(SpacetimeType, Clone, Debug, PartialEq, Eq, Default)]
pub struct NSFWInfo {
    pub is_nsfw: bool,
    pub nsfw_ec: String,
    pub nsfw_gore: String,
    pub csam_detected: bool,
}

/// Profile picture data with NSFW info. Mirrors the IC `ProfilePictureData`.
#[derive(SpacetimeType, Clone, Debug, PartialEq, Eq, Default)]
pub struct ProfilePictureData {
    pub url: String,
    pub nsfw_info: NSFWInfo,
}

/// Account type. Mirrors the IC `UserAccountType` enum.
/// `MainAccount` can own bots; `BotAccount` has an owner principal.
#[derive(SpacetimeType, Clone, Debug, PartialEq, Eq)]
pub enum UserAccountType {
    MainAccount { bots: Vec<String> },
    BotAccount { owner: String },
}

impl Default for UserAccountType {
    fn default() -> Self {
        UserAccountType::MainAccount { bots: Vec::new() }
    }
}

/// Frontend-facing profile projection. Mirrors the IC canister's
/// `UserProfileDetailsForFrontendV4`.
#[derive(SpacetimeType, Clone, Debug)]
pub struct UserProfileDetailsV4 {
    pub principal_text: String,
    pub bio: String,
    pub website_url: String,
    pub profile_picture_url: String,
    pub followers_count: u64,
    pub following_count: u64,
    pub subscription_plan: SubscriptionPlan,
    pub is_ai_influencer: bool,
    pub caller_follows_user: bool,
    pub user_follows_caller: bool,
}

/// A page of followers (cursor-paginated).
#[derive(SpacetimeType, Clone, Debug)]
pub struct FollowersPage {
    pub followers: Vec<FollowerItem>,
    pub total_count: u64,
    pub next_cursor: Option<String>,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct FollowerItem {
    pub principal_text: String,
    pub caller_follows: bool,
    pub profile_picture_url: String,
}

/// A page of following (cursor-paginated).
#[derive(SpacetimeType, Clone, Debug)]
pub struct FollowingPage {
    pub following: Vec<FollowingItem>,
    pub total_count: u64,
    pub next_cursor: Option<String>,
}

#[derive(SpacetimeType, Clone, Debug)]
pub struct FollowingItem {
    pub principal_text: String,
    pub caller_follows: bool,
    pub profile_picture_url: String,
}

/// Frontend-facing profile projection V7. Mirrors the IC canister's
/// `UserProfileDetailsForFrontendV7`. This is the primary profile read
/// used by the mobile app. Includes `account_type` and `profile_picture`
/// with NSFW info.
#[derive(SpacetimeType, Clone, Debug)]
pub struct UserProfileDetailsV7 {
    pub principal_text: String,
    pub profile_picture: Option<ProfilePictureData>,
    pub bio: String,
    pub website_url: String,
    pub followers_count: u64,
    pub following_count: u64,
    /// `None` when caller == user (self), `Some(bool)` otherwise.
    pub caller_follows_user: Option<bool>,
    /// `None` when caller == user (self), `Some(bool)` otherwise.
    pub user_follows_caller: Option<bool>,
    pub subscription_plan: SubscriptionPlan,
    pub is_ai_influencer: bool,
    pub account_type: UserAccountType,
}

/// Batch upsert entry for backfill. Mirrors `UserProfile` but without
/// `account_type` (backfill sets it to `MainAccount` default).
#[derive(SpacetimeType, Clone, Debug)]
pub struct UserProfileBatchEntry {
    pub principal_text: String,
    pub bio: String,
    pub website_url: String,
    pub profile_picture_url: String,
    pub followers_count: u64,
    pub following_count: u64,
    pub subscription_plan: SubscriptionPlan,
    pub is_ai_influencer: bool,
    pub is_nsfw: bool,
    pub nsfw_ec: String,
    pub nsfw_gore: String,
    pub csam_detected: bool,
    pub last_access_time: Timestamp,
    /// Display name from the yral-metadata service (Redis/Dragonfly).
    pub username: Option<String>,
    /// Email from the yral-metadata service (Redis/Dragonfly).
    pub email: Option<String>,
}

/// Batch upsert entry for follow relationships (backfill).
#[derive(SpacetimeType, Clone, Debug)]
pub struct UserFollowBatchEntry {
    pub follower_text: String,
    pub followee_text: String,
}

// ─────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────

/// Maximum number of items per page (matches IC canister's max).
const MAX_PAGE_SIZE: u64 = 100;

// ─────────────────────────────────────────────────────────────────────────
// Helpers (non-trivial logic shared by multiple call sites)
// ─────────────────────────────────────────────────────────────────────────

/// Build a `ProfilePictureData` from the profile's URL and NSFW fields.
/// Returns `None` when the profile has no picture URL set.
fn profile_picture_data(profile: &UserProfile) -> Option<ProfilePictureData> {
    if profile.profile_picture_url.is_empty() {
        return None;
    }
    Some(ProfilePictureData {
        url: profile.profile_picture_url.clone(),
        nsfw_info: NSFWInfo {
            is_nsfw: profile.is_nsfw,
            nsfw_ec: profile.nsfw_ec.clone(),
            nsfw_gore: profile.nsfw_gore.clone(),
            csam_detected: profile.csam_detected,
        },
    })
}

/// Compute the follow relationship between caller and target.
/// Returns `(caller_follows_user, user_follows_caller)`, both `None` when
/// caller == user (self). Must be called inside a `with_tx` closure where
/// `tx` is a `TxContext` (has `db` + `sender()`).
fn follow_relationships(
    tx: &spacetimedb::TxContext,
    caller_text: &str,
    principal_text: &str,
) -> (Option<bool>, Option<bool>) {
    if caller_text == principal_text {
        return (None, None);
    }
    let caller_follows_user = tx
        .db
        .user_follows()
        .iter()
        .any(|f| f.key == format!("{caller_text}::{principal_text}"));
    let user_follows_caller = tx
        .db
        .user_follows()
        .iter()
        .any(|f| f.key == format!("{principal_text}::{caller_text}"));
    (Some(caller_follows_user), Some(user_follows_caller))
}

// ─────────────────────────────────────────────────────────────────────────
// Reducers (writes)
// ─────────────────────────────────────────────────────────────────────────

/// Register a new user. Idempotent — if the user already exists, does nothing.
/// Called by the mobile app after authentication.
#[spacetimedb::reducer]
pub fn register_new_user(ctx: &ReducerContext) -> Result<(), String> {
    let principal_text = ctx.sender().to_hex().to_string();

    if ctx
        .db
        .user_profiles()
        .iter()
        .any(|p| p.principal_text == principal_text)
    {
        return Ok(()); // Already registered
    }

    ctx.db.user_profiles().insert(UserProfile {
        principal_text,
        bio: String::new(),
        website_url: String::new(),
        profile_picture_url: String::new(),
        followers_count: 0,
        following_count: 0,
        subscription_plan: SubscriptionPlan::Free,
        is_ai_influencer: false,
        is_nsfw: false,
        nsfw_ec: String::new(),
        nsfw_gore: String::new(),
        csam_detected: false,
        last_access_time: ctx.timestamp,
        account_type: UserAccountType::MainAccount { bots: Vec::new() },
        username: None,
        email: None,
    });

    Ok(())
}

/// Follow another user. Bidirectional — inserts into both follower's
/// following set and followee's followers set.
#[spacetimedb::reducer]
pub fn follow_user(ctx: &ReducerContext, followee_text: String) -> Result<(), String> {
    let follower_text = ctx.sender().to_hex().to_string();
    let key = format!("{follower_text}::{followee_text}");

    if follower_text == followee_text {
        return Err("Cannot follow yourself".to_string());
    }

    if ctx.db.user_follows().iter().any(|f| f.key == key) {
        return Err("Already following".to_string());
    }

    ctx.db.user_follows().insert(UserFollow {
        key,
        follower_text: follower_text.clone(),
        followee_text: followee_text.clone(),
    });

    // Update follower's following count
    if let Some(mut p) = ctx
        .db
        .user_profiles()
        .iter()
        .find(|p| p.principal_text == follower_text)
    {
        p.following_count += 1;
        ctx.db.user_profiles().delete(p.clone());
        ctx.db.user_profiles().insert(p);
    }

    // Update followee's followers count
    if let Some(mut p) = ctx
        .db
        .user_profiles()
        .iter()
        .find(|p| p.principal_text == followee_text)
    {
        p.followers_count += 1;
        ctx.db.user_profiles().delete(p.clone());
        ctx.db.user_profiles().insert(p);
    }

    Ok(())
}

/// Unfollow another user. Symmetric to follow.
#[spacetimedb::reducer]
pub fn unfollow_user(ctx: &ReducerContext, followee_text: String) -> Result<(), String> {
    let follower_text = ctx.sender().to_hex().to_string();
    let key = format!("{follower_text}::{followee_text}");

    let Some(follow) = ctx.db.user_follows().iter().find(|f| f.key == key) else {
        return Err("Not following".to_string());
    };

    ctx.db.user_follows().delete(follow);

    // Update follower's following count
    if let Some(mut p) = ctx
        .db
        .user_profiles()
        .iter()
        .find(|p| p.principal_text == follower_text)
    {
        p.following_count = p.following_count.saturating_sub(1);
        ctx.db.user_profiles().delete(p.clone());
        ctx.db.user_profiles().insert(p);
    }

    // Update followee's followers count
    if let Some(mut p) = ctx
        .db
        .user_profiles()
        .iter()
        .find(|p| p.principal_text == followee_text)
    {
        p.followers_count = p.followers_count.saturating_sub(1);
        ctx.db.user_profiles().delete(p.clone());
        ctx.db.user_profiles().insert(p);
    }

    Ok(())
}

/// Update profile details. Only the authenticated user can update their own
/// profile.
#[spacetimedb::reducer]
pub fn update_profile_details(
    ctx: &ReducerContext,
    bio: String,
    website_url: String,
    profile_pic_url: String,
) -> Result<(), String> {
    let principal_text = ctx.sender().to_hex().to_string();

    let mut profile = match ctx
        .db
        .user_profiles()
        .iter()
        .find(|p| p.principal_text == principal_text)
    {
        Some(p) => p,
        None => return Err("User not found".to_string()),
    };

    profile.bio = bio;
    profile.website_url = website_url;
    profile.profile_picture_url = profile_pic_url;
    ctx.db.user_profiles().delete(profile.clone());
    ctx.db.user_profiles().insert(profile);

    Ok(())
}

/// Admin-only: update the AI influencer status for a user.
#[spacetimedb::reducer]
pub fn update_profile_ai_influencer_status(
    ctx: &ReducerContext,
    principal_text: String,
    is_ai_influencer: bool,
) -> Result<(), String> {
    if !crate::constants::ADMINS.contains(&ctx.sender()) {
        return Err("Unauthorized".to_string());
    }

    let mut profile = match ctx
        .db
        .user_profiles()
        .iter()
        .find(|p| p.principal_text == principal_text)
    {
        Some(p) => p,
        None => return Err("User not found".to_string()),
    };

    profile.is_ai_influencer = is_ai_influencer;
    ctx.db.user_profiles().delete(profile.clone());
    ctx.db.user_profiles().insert(profile);

    Ok(())
}

/// Update profile details (V2 — with NSFW-aware profile picture).
/// Mirrors the IC canister's `update_profile_details_v2`. Only the
/// authenticated user can update their own profile.
#[spacetimedb::reducer]
pub fn update_profile_details_v2(
    ctx: &ReducerContext,
    bio: Option<String>,
    website_url: Option<String>,
    profile_picture: Option<ProfilePictureData>,
) -> Result<(), String> {
    let principal_text = ctx.sender().to_hex().to_string();

    let mut profile = match ctx
        .db
        .user_profiles()
        .iter()
        .find(|p| p.principal_text == principal_text)
    {
        Some(p) => p,
        None => return Err("User not found".to_string()),
    };

    if let Some(bio) = bio {
        profile.bio = bio;
    }
    if let Some(website_url) = website_url {
        profile.website_url = website_url;
    }
    if let Some(pic) = profile_picture {
        profile.profile_picture_url = pic.url;
        profile.is_nsfw = pic.nsfw_info.is_nsfw;
        profile.nsfw_ec = pic.nsfw_info.nsfw_ec;
        profile.nsfw_gore = pic.nsfw_info.nsfw_gore;
        profile.csam_detected = pic.nsfw_info.csam_detected;
    }
    ctx.db.user_profiles().delete(profile.clone());
    ctx.db.user_profiles().insert(profile);

    Ok(())
}

/// Admin-only: update the NSFW info on a user's profile picture.
/// Mirrors the IC canister's `update_profile_picture_nsfw_info`.
#[spacetimedb::reducer]
pub fn update_profile_picture_nsfw_info(
    ctx: &ReducerContext,
    principal_text: String,
    nsfw_info: NSFWInfo,
) -> Result<(), String> {
    if !crate::constants::ADMINS.contains(&ctx.sender()) {
        return Err("Unauthorized".to_string());
    }

    let mut profile = match ctx
        .db
        .user_profiles()
        .iter()
        .find(|p| p.principal_text == principal_text)
    {
        Some(p) => p,
        None => return Err("User not found".to_string()),
    };

    profile.is_nsfw = nsfw_info.is_nsfw;
    profile.nsfw_ec = nsfw_info.nsfw_ec;
    profile.nsfw_gore = nsfw_info.nsfw_gore;
    profile.csam_detected = nsfw_info.csam_detected;
    ctx.db.user_profiles().delete(profile.clone());
    ctx.db.user_profiles().insert(profile);

    Ok(())
}

/// Register a new user or create a bot account. Mirrors the IC canister's
/// `accept_new_user_registration_v2`.
///
/// - When `main_account_text` is `Some(owner)`: creates a bot account owned
///   by `owner` (owner must exist and be a `MainAccount`). The bot is added
///   to the owner's `bots` list.
/// - When `main_account_text` is `None`: creates a normal account for
///   `new_principal_text`.
///
/// The `authenticated` parameter is accepted for API compatibility but does
/// not set a session type (session types were dropped from the migration).
#[spacetimedb::reducer]
pub fn accept_new_user_registration_v2(
    ctx: &ReducerContext,
    new_principal_text: String,
    _authenticated: bool,
    main_account_text: Option<String>,
) -> Result<(), String> {
    // Idempotent: if user already exists, do nothing
    if ctx
        .db
        .user_profiles()
        .iter()
        .any(|p| p.principal_text == new_principal_text)
    {
        return Ok(());
    }

    match main_account_text {
        Some(owner_text) => {
            // Bot account creation — owner must exist and be a MainAccount
            let mut owner_profile = match ctx
                .db
                .user_profiles()
                .iter()
                .find(|p| p.principal_text == owner_text)
            {
                Some(p) => p,
                None => return Err("Owner not found".to_string()),
            };
            match &owner_profile.account_type {
                UserAccountType::MainAccount { bots } => {
                    let mut bots = bots.clone();
                    bots.push(new_principal_text.clone());
                    owner_profile.account_type = UserAccountType::MainAccount { bots };
                    ctx.db.user_profiles().delete(owner_profile.clone());
                    ctx.db.user_profiles().insert(owner_profile);
                }
                UserAccountType::BotAccount { .. } => {
                    return Err("Bots cannot own other bots".to_string());
                }
            }

            // Create the bot account
            ctx.db.user_profiles().insert(UserProfile {
                principal_text: new_principal_text,
                bio: String::new(),
                website_url: String::new(),
                profile_picture_url: String::new(),
                followers_count: 0,
                following_count: 0,
                subscription_plan: SubscriptionPlan::Free,
                is_ai_influencer: false,
                is_nsfw: false,
                nsfw_ec: String::new(),
                nsfw_gore: String::new(),
                csam_detected: false,
                last_access_time: ctx.timestamp,
                account_type: UserAccountType::BotAccount { owner: owner_text },
                username: None,
                email: None,
            });
        }
        None => {
            // Normal account registration
            ctx.db.user_profiles().insert(UserProfile {
                principal_text: new_principal_text,
                bio: String::new(),
                website_url: String::new(),
                profile_picture_url: String::new(),
                followers_count: 0,
                following_count: 0,
                subscription_plan: SubscriptionPlan::Free,
                is_ai_influencer: false,
                is_nsfw: false,
                nsfw_ec: String::new(),
                nsfw_gore: String::new(),
                csam_detected: false,
                last_access_time: ctx.timestamp,
                account_type: UserAccountType::MainAccount { bots: Vec::new() },
                username: None,
                email: None,
            });
        }
    }

    Ok(())
}

/// Delete a user and cascade-delete their bots. Mirrors the IC canister's
/// `delete_user_info`.
///
/// - `MainAccount`: admin or self can delete. Cascade-deletes all bots.
/// - `BotAccount`: admin or owner can delete. Removes the bot from the
///   owner's `bots` list.
#[spacetimedb::reducer]
pub fn delete_user_info(
    ctx: &ReducerContext,
    principal_to_delete_text: String,
) -> Result<(), String> {
    let caller_text = ctx.sender().to_hex().to_string();
    let admin = crate::constants::ADMINS.contains(&ctx.sender());

    let profile = match ctx
        .db
        .user_profiles()
        .iter()
        .find(|p| p.principal_text == principal_to_delete_text)
    {
        Some(p) => p,
        None => return Err("User not found".to_string()),
    };

    match &profile.account_type {
        UserAccountType::MainAccount { bots } => {
            // Admin or self can delete a MainAccount
            if !admin && principal_to_delete_text != caller_text {
                return Err("Unauthorized".to_string());
            }
            // Cascade: delete all bots
            for bot_text in bots {
                if let Some(bot_profile) = ctx
                    .db
                    .user_profiles()
                    .iter()
                    .find(|p| &p.principal_text == bot_text)
                {
                    ctx.db.user_profiles().delete(bot_profile);
                }
            }
            // Delete the main account
            ctx.db.user_profiles().delete(profile);
        }
        UserAccountType::BotAccount { owner } => {
            // Admin or owner can delete a BotAccount
            if !admin && owner != &caller_text {
                return Err("Unauthorized".to_string());
            }
            // Remove this bot from the owner's bots list
            if let Some(mut owner_profile) = ctx
                .db
                .user_profiles()
                .iter()
                .find(|p| p.principal_text == *owner)
            {
                if let UserAccountType::MainAccount { bots } = &owner_profile.account_type {
                    let new_bots: Vec<String> = bots
                        .iter()
                        .filter(|b| *b != &principal_to_delete_text)
                        .cloned()
                        .collect();
                    owner_profile.account_type = UserAccountType::MainAccount { bots: new_bots };
                    ctx.db.user_profiles().delete(owner_profile.clone());
                    ctx.db.user_profiles().insert(owner_profile);
                }
            }
            // Delete the bot
            ctx.db.user_profiles().delete(profile);
        }
    }

    // Also delete any follow relationships involving this user
    let follows_to_delete: Vec<UserFollow> = ctx
        .db
        .user_follows()
        .iter()
        .filter(|f| f.follower_text == principal_to_delete_text || f.followee_text == principal_to_delete_text)
        .collect();
    for f in follows_to_delete {
        ctx.db.user_follows().delete(f);
    }

    Ok(())
}

/// Update the caller's last access time to the current timestamp.
#[spacetimedb::reducer]
pub fn update_user_last_access_time(ctx: &ReducerContext) -> Result<(), String> {
    let principal_text = ctx.sender().to_hex().to_string();

    let mut profile = match ctx
        .db
        .user_profiles()
        .iter()
        .find(|p| p.principal_text == principal_text)
    {
        Some(p) => p,
        None => return Err("User not found".to_string()),
    };

    profile.last_access_time = ctx.timestamp;
    ctx.db.user_profiles().delete(profile.clone());
    ctx.db.user_profiles().insert(profile);

    Ok(())
}

/// Admin-only: change a user's subscription plan.
#[spacetimedb::reducer]
pub fn change_subscription_plan(
    ctx: &ReducerContext,
    principal_text: String,
    plan: SubscriptionPlan,
) -> Result<(), String> {
    if !crate::constants::ADMINS.contains(&ctx.sender()) {
        return Err("Unauthorized".to_string());
    }

    let mut profile = match ctx
        .db
        .user_profiles()
        .iter()
        .find(|p| p.principal_text == principal_text)
    {
        Some(p) => p,
        None => return Err("User not found".to_string()),
    };

    profile.subscription_plan = plan;
    ctx.db.user_profiles().delete(profile.clone());
    ctx.db.user_profiles().insert(profile);

    Ok(())
}

/// Admin-only: add Pro plan free video credits to a user.
#[spacetimedb::reducer]
pub fn add_pro_plan_free_video_credits(
    ctx: &ReducerContext,
    principal_text: String,
    credits: u32,
) -> Result<(), String> {
    if !crate::constants::ADMINS.contains(&ctx.sender()) {
        return Err("Unauthorized".to_string());
    }

    let mut profile = match ctx
        .db
        .user_profiles()
        .iter()
        .find(|p| p.principal_text == principal_text)
    {
        Some(p) => p,
        None => return Err("User not found".to_string()),
    };

    match profile.subscription_plan {
        SubscriptionPlan::Pro(ref mut sub) => {
            sub.free_video_credits_left = sub.free_video_credits_left.saturating_add(credits);
        }
        SubscriptionPlan::Free => {
            return Err("User is on Free plan".to_string());
        }
    }
    ctx.db.user_profiles().delete(profile.clone());
    ctx.db.user_profiles().insert(profile);

    Ok(())
}

/// Admin-only: remove Pro plan free video credits from a user.
#[spacetimedb::reducer]
pub fn remove_pro_plan_free_video_credits(
    ctx: &ReducerContext,
    principal_text: String,
    credits: u32,
) -> Result<(), String> {
    if !crate::constants::ADMINS.contains(&ctx.sender()) {
        return Err("Unauthorized".to_string());
    }

    let mut profile = match ctx
        .db
        .user_profiles()
        .iter()
        .find(|p| p.principal_text == principal_text)
    {
        Some(p) => p,
        None => return Err("User not found".to_string()),
    };

    match profile.subscription_plan {
        SubscriptionPlan::Pro(ref mut sub) => {
            sub.free_video_credits_left = sub.free_video_credits_left.saturating_sub(credits);
        }
        SubscriptionPlan::Free => {
            return Err("User is on Free plan".to_string());
        }
    }
    ctx.db.user_profiles().delete(profile.clone());
    ctx.db.user_profiles().insert(profile);

    Ok(())
}

/// Admin-only: batch upsert user profiles (for IC → SpacetimeDB backfill).
/// Idempotent — delete-then-insert by primary key.
#[spacetimedb::reducer]
pub fn upsert_user_profile_batch(
    ctx: &ReducerContext,
    profiles: Vec<UserProfileBatchEntry>,
) -> Result<(), String> {
    if !crate::constants::ADMINS.contains(&ctx.sender()) {
        return Err("Unauthorized".to_string());
    }

    for entry in profiles {
        // Delete existing if present
        if let Some(existing) = ctx
            .db
            .user_profiles()
            .iter()
            .find(|p| p.principal_text == entry.principal_text)
        {
            ctx.db.user_profiles().delete(existing);
        }

        ctx.db.user_profiles().insert(UserProfile {
            principal_text: entry.principal_text,
            bio: entry.bio,
            website_url: entry.website_url,
            profile_picture_url: entry.profile_picture_url,
            followers_count: entry.followers_count,
            following_count: entry.following_count,
            subscription_plan: entry.subscription_plan,
            is_ai_influencer: entry.is_ai_influencer,
            is_nsfw: entry.is_nsfw,
            nsfw_ec: entry.nsfw_ec,
            nsfw_gore: entry.nsfw_gore,
            csam_detected: entry.csam_detected,
            last_access_time: entry.last_access_time,
            account_type: UserAccountType::MainAccount { bots: Vec::new() },
            username: entry.username,
            email: entry.email,
        });
    }

    Ok(())
}

/// Admin-only: batch upsert follow relationships (for IC → SpacetimeDB backfill).
/// Idempotent — delete-then-insert by primary key.
#[spacetimedb::reducer]
pub fn upsert_user_follow_batch(
    ctx: &ReducerContext,
    follows: Vec<UserFollowBatchEntry>,
) -> Result<(), String> {
    if !crate::constants::ADMINS.contains(&ctx.sender()) {
        return Err("Unauthorized".to_string());
    }

    for entry in follows {
        let key = format!("{}::{}", entry.follower_text, entry.followee_text);
        // Delete existing if present
        if let Some(existing) = ctx.db.user_follows().iter().find(|f| f.key == key) {
            ctx.db.user_follows().delete(existing);
        }

        ctx.db.user_follows().insert(UserFollow {
            key,
            follower_text: entry.follower_text,
            followee_text: entry.followee_text,
        });
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Procedures (reads — return typed data)
// ─────────────────────────────────────────────────────────────────────────

/// Get profile details for a user. Returns `None` if the user doesn't exist.
/// The `caller_follows_user` and `user_follows_caller` fields are computed
/// from the `user_follows` table using `ctx.sender()`.
#[spacetimedb::procedure]
pub fn get_profile_details_v4(
    ctx: &mut ProcedureContext,
    principal_text: String,
) -> Option<UserProfileDetailsV4> {
    ctx.with_tx(|tx| {
        let profile = tx
            .db
            .user_profiles()
            .iter()
            .find(|p| p.principal_text == principal_text)?;
        let caller_text = tx.sender().to_hex().to_string();

        let caller_follows_user = tx
            .db
            .user_follows()
            .iter()
            .any(|f| f.key == format!("{caller_text}::{principal_text}"));

        let user_follows_caller = tx
            .db
            .user_follows()
            .iter()
            .any(|f| f.key == format!("{principal_text}::{caller_text}"));

        Some(UserProfileDetailsV4 {
            principal_text: profile.principal_text,
            bio: profile.bio,
            website_url: profile.website_url,
            profile_picture_url: profile.profile_picture_url,
            followers_count: profile.followers_count,
            following_count: profile.following_count,
            subscription_plan: profile.subscription_plan,
            is_ai_influencer: profile.is_ai_influencer,
            caller_follows_user,
            user_follows_caller,
        })
    })
}

/// Get profile details V7 for a user. Returns `None` if the user doesn't
/// exist. This is the primary profile read used by the mobile app.
/// Includes `account_type`, `profile_picture` with NSFW info, and
/// `subscription_plan` with Pro credits.
///
/// Follow relationship fields are `None` when caller == user (self).
#[spacetimedb::procedure]
pub fn get_user_profile_details_v7(
    ctx: &mut ProcedureContext,
    principal_text: String,
) -> Option<UserProfileDetailsV7> {
    ctx.with_tx(|tx| {
        let profile = tx
            .db
            .user_profiles()
            .iter()
            .find(|p| p.principal_text == principal_text)?;
        let caller_text = tx.sender().to_hex().to_string();

        let (caller_follows_user, user_follows_caller) =
            follow_relationships(tx, &caller_text, &principal_text);

        let principal_text = profile.principal_text.clone();
        let profile_picture = profile_picture_data(&profile);

        Some(UserProfileDetailsV7 {
            principal_text,
            profile_picture,
            bio: profile.bio,
            website_url: profile.website_url,
            followers_count: profile.followers_count,
            following_count: profile.following_count,
            caller_follows_user,
            user_follows_caller,
            subscription_plan: profile.subscription_plan,
            is_ai_influencer: profile.is_ai_influencer,
            account_type: profile.account_type,
        })
    })
}

/// Batch profile lookup. Returns V7 profile details for each principal.
/// Users that are not found are silently skipped (matches IC canister
/// behavior). Follow relationships are computed using `ctx.sender()`.
#[spacetimedb::procedure]
pub fn get_users_profile_details(
    ctx: &mut ProcedureContext,
    principal_texts: Vec<String>,
) -> Vec<UserProfileDetailsV7> {
    ctx.with_tx(|tx| {
        let caller_text = tx.sender().to_hex().to_string();

        principal_texts
            .iter()
            .filter_map(|principal_text| {
                let profile = tx
                    .db
                    .user_profiles()
                    .iter()
                    .find(|p| &p.principal_text == principal_text)?;

                let (caller_follows_user, user_follows_caller) =
                    follow_relationships(tx, &caller_text, principal_text);

                let principal_text = profile.principal_text.clone();
                let profile_picture = profile_picture_data(&profile);

                Some(UserProfileDetailsV7 {
                    principal_text,
                    profile_picture,
                    bio: profile.bio,
                    website_url: profile.website_url,
                    followers_count: profile.followers_count,
                    following_count: profile.following_count,
                    caller_follows_user,
                    user_follows_caller,
                    subscription_plan: profile.subscription_plan,
                    is_ai_influencer: profile.is_ai_influencer,
                    account_type: profile.account_type,
                })
            })
            .collect()
    })
}

/// Get a page of followers for a user (cursor-paginated).
#[spacetimedb::procedure]
pub fn get_followers(
    ctx: &mut ProcedureContext,
    principal_text: String,
    limit: u64,
    cursor: Option<String>,
) -> FollowersPage {
    ctx.with_tx(|tx| {
        let limit = limit.min(MAX_PAGE_SIZE) as usize;
        let caller_text = tx.sender().to_hex().to_string();

        // Collect all followers of this user (followee = principal_text)
        let mut followers: Vec<UserFollow> = tx
            .db
            .user_follows()
            .iter()
            .filter(|f| f.followee_text == principal_text)
            .collect();

        // Sort by follower_text for stable pagination
        followers.sort_by(|a, b| a.follower_text.cmp(&b.follower_text));

        // Find cursor position
        let start = match &cursor {
            Some(cursor_id) => followers
                .iter()
                .position(|f| f.follower_text.as_str() > cursor_id.as_str())
                .unwrap_or(0),
            None => 0,
        };

        let page: Vec<UserFollow> = followers.iter().skip(start).take(limit).cloned().collect();
        let next_cursor = if start + limit < followers.len() {
            page.last().map(|f| f.follower_text.clone())
        } else {
            None
        };

        let total_count = followers.len() as u64;

        // Build follower items with profile pics and follow status
        let items: Vec<FollowerItem> = page
            .iter()
            .map(|f| {
                let profile = tx
                    .db
                    .user_profiles()
                    .iter()
                    .find(|p| p.principal_text == f.follower_text);
                let pic = profile
                    .map(|p| p.profile_picture_url)
                    .unwrap_or_default();
                let caller_follows = tx
                    .db
                    .user_follows()
                    .iter()
                    .any(|uf| uf.key == format!("{caller_text}::{}", f.follower_text));
                FollowerItem {
                    principal_text: f.follower_text.clone(),
                    caller_follows,
                    profile_picture_url: pic,
                }
            })
            .collect();

        FollowersPage {
            followers: items,
            total_count,
            next_cursor,
        }
    })
}

/// Get a page of users that a user is following (cursor-paginated).
#[spacetimedb::procedure]
pub fn get_following(
    ctx: &mut ProcedureContext,
    principal_text: String,
    limit: u64,
    cursor: Option<String>,
) -> FollowingPage {
    ctx.with_tx(|tx| {
        let limit = limit.min(MAX_PAGE_SIZE) as usize;
        let caller_text = tx.sender().to_hex().to_string();

        // Collect all users this user is following (follower = principal_text)
        let mut following: Vec<UserFollow> = tx
            .db
            .user_follows()
            .iter()
            .filter(|f| f.follower_text == principal_text)
            .collect();

        // Sort by followee_text for stable pagination
        following.sort_by(|a, b| a.followee_text.cmp(&b.followee_text));

        // Find cursor position
        let start = match &cursor {
            Some(cursor_id) => following
                .iter()
                .position(|f| f.followee_text.as_str() > cursor_id.as_str())
                .unwrap_or(0),
            None => 0,
        };

        let page: Vec<UserFollow> = following.iter().skip(start).take(limit).cloned().collect();
        let next_cursor = if start + limit < following.len() {
            page.last().map(|f| f.followee_text.clone())
        } else {
            None
        };

        let total_count = following.len() as u64;

        // Build following items with profile pics and follow status
        let items: Vec<FollowingItem> = page
            .iter()
            .map(|f| {
                let profile = tx
                    .db
                    .user_profiles()
                    .iter()
                    .find(|p| p.principal_text == f.followee_text);
                let pic = profile
                    .map(|p| p.profile_picture_url)
                    .unwrap_or_default();
                let caller_follows = tx
                    .db
                    .user_follows()
                    .iter()
                    .any(|uf| uf.key == format!("{caller_text}::{}", f.followee_text));
                FollowingItem {
                    principal_text: f.followee_text.clone(),
                    caller_follows,
                    profile_picture_url: pic,
                }
            })
            .collect();

        FollowingPage {
            following: items,
            total_count,
            next_cursor,
        }
    })
}