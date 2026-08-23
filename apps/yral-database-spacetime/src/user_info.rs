//! User info service — migrated from the IC `user_info_service` canister
//! (`ivkka-7qaaa-aaaas-qbg3q-cai`).
//!
//! ## Incremental migration: `user_profiles` → `user_profiles_2`
//!
//! The old tables `user_profiles` (keyed by `principal_text`) and `user_follows`
//! (keyed by `follower_text`/`followee_text`) are being migrated to `user_profiles_2`
//! (keyed by `oauth_subject`) and `user_follows_2` (keyed by
//! `follower_subject`/`followee_subject`). The old tables are kept as-is for the
//! migration; all writes go to the new tables, reads use lazy migration (check
//! `user_profiles_2` first, fall back to `user_profiles` and migrate on the fly).
//! The admin reducer `migrate_user_profiles_to_2` backfills all rows.
//!
//! ## Tables
//! - `user_profiles_2`: keyed by `oauth_subject`, stores profile/follow data
//! - `user_follows_2`: keyed by `(follower_subject, followee_subject)`, bidirectional
//! - `user_profiles` (legacy, read-only during migration)
//! - `user_follows` (legacy, read-only during migration)
//!
//! ## Procedures (reads, return typed data)
//! - `get_profile_details_v4(oauth_subject) -> Option<UserProfileDetailsV4>`
//! - `get_user_profile_details_v7(oauth_subject) -> Option<UserProfileDetailsV7>`
//! - `get_users_profile_details(oauth_subjects) -> Vec<UserProfileDetailsV7>`
//! - `get_followers(oauth_subject, limit, cursor) -> FollowersPage`
//! - `get_following(oauth_subject, limit, cursor) -> FollowingPage`
//!
//! ## Reducers (writes)
//! - `follow_user(followee_subject)` — bidirectional follow
//! - `unfollow_user(followee_subject)` — symmetric
//! - `update_profile_details(bio, website_url, profile_pic_url)` — sender only
//! - `update_profile_details_v2(bio, website_url, profile_picture)` — sender only
//! - `update_profile_ai_influencer_status(oauth_subject, is_ai)` — admin only
//! - `accept_new_user_registration_v2(new_principal_text, authenticated, main_account_text)` — register/bot
//! - `delete_user_info(principal_to_delete_text)` — cascade delete
//! - `update_user_last_access_time()` — sender only
//! - `update_profile_picture_nsfw_info(oauth_subject, nsfw_info)` — admin only
//! - `change_subscription_plan(oauth_subject, plan)` — admin only
//! - `add_pro_plan_free_video_credits(oauth_subject, credits)` — admin only
//! - `remove_pro_plan_free_video_credits(oauth_subject, credits)` — admin only
//! - `upsert_user_profile_batch(profiles)` — admin only, backfill (writes to `user_profiles_2`)
//! - `upsert_user_follow_batch(follows)` — admin only, backfill (writes to `user_follows_2`)
//! - `migrate_user_profiles_to_2()` — admin only, one-time backfill from old tables

use spacetimedb::{ProcedureContext, ReducerContext, SpacetimeType, Table, Timestamp};

// ─────────────────────────────────────────────────────────────────────────
// Tables
// ─────────────────────────────────────────────────────────────────────────

/// Legacy user profile table (kept as-is for the incremental migration).
///
/// Primary key: `principal_text`. For users registered via
/// `accept_new_user_registration_v2`, this is the OAuth `sub` claim from the
/// yral-auth JWT (e.g. a Google account ID like `100014004491598860137`).
/// This is the same value stored in `posts_v2.creator_principal_text`.
///
/// **Do not write to this table.** All writes go to `user_profiles_2`.
/// Reads use lazy migration: check `user_profiles_2` first, then fall back
/// here and migrate the row.
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
    /// Display name from the yral-metadata service.
    /// Backfilled from `UserMetadata.user_name`. `None` if not yet backfilled.
    #[default(None::<String>)]
    pub username: Option<String>,
    /// Email from the yral-metadata service.
    /// Backfilled from `UserMetadata.email`. `None` if not set.
    #[default(None::<String>)]
    pub email: Option<String>,
    /// OAuth subject identifier (Google/Apple `sub` or UUID for phone auth).
    /// Links this profile to the yral-auth identity. `None` for legacy users
    /// not yet linked to an OAuth sub.
    #[default(None::<String>)]
    pub user_id: Option<String>,
}

/// New user profile table (target of the incremental migration).
///
/// Same schema as `UserProfile` but with `principal_text` renamed to
/// `oauth_subject` to accurately reflect that this field holds the OAuth
/// `sub` claim (not an Internet Computer "principal").
///
/// Primary key: `oauth_subject`. All writes go here; reads check here first
/// and lazy-migrate from `user_profiles` on miss.
#[spacetimedb::table(name = "user_profiles_2", accessor = user_profiles_2, public)]
#[derive(Clone)]
pub struct UserProfile2 {
    #[primary_key]
    pub oauth_subject: String,
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
    /// Display name from the yral-metadata service.
    /// Backfilled from `UserMetadata.user_name`. `None` if not yet backfilled.
    #[default(None::<String>)]
    pub username: Option<String>,
    /// Email from the yral-metadata service.
    /// Backfilled from `UserMetadata.email`. `None` if not set.
    #[default(None::<String>)]
    pub email: Option<String>,
    /// OAuth subject identifier (Google/Apple `sub` or UUID for phone auth).
    /// Links this profile to the yral-auth identity. `None` for legacy users
    /// not yet linked to an OAuth sub.
    #[default(None::<String>)]
    pub user_id: Option<String>,
}

/// Legacy follow relationship table (kept as-is for the incremental migration).
///
/// Primary key is a composite key `{follower_text}::{followee_text}` for
/// uniqueness. Both directions are indexed for efficient "who follows X" and
/// "who does X follow" queries.
///
/// **Do not write to this table.** All writes go to `user_follows_2`.
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

/// New follow relationship table (target of the incremental migration).
///
/// Same schema as `UserFollow` but with `follower_text` → `follower_subject`
/// and `followee_text` → `followee_subject` to accurately reflect that these
/// fields hold OAuth `sub` claims.
///
/// Primary key is a composite key `{follower_subject}::{followee_subject}`
/// for uniqueness. Both directions are indexed.
#[spacetimedb::table(name = "user_follows_2", accessor = user_follows_2, public)]
#[derive(Clone)]
pub struct UserFollow2 {
    #[primary_key]
    pub key: String, // "{follower}::{followee}"
    #[index(btree)]
    pub follower_subject: String,
    #[index(btree)]
    pub followee_subject: String,
}

/// FCM push notification device token. A user can have multiple devices.
/// Replaces the `notification_key` field from the yral-metadata store.
#[spacetimedb::table(accessor = user_notification_tokens, public)]
#[derive(Clone)]
pub struct UserNotificationToken {
    #[primary_key]
    pub key: String, // "{user_id}::{token}"
    #[index(btree)]
    pub user_id: String,
    /// FCM device registration token.
    pub token: String,
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
/// `MainAccount` can own bots; `BotAccount` has an owner subject.
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
    pub oauth_subject: String,
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
    pub oauth_subject: String,
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
    pub oauth_subject: String,
    pub caller_follows: bool,
    pub profile_picture_url: String,
}

/// Frontend-facing profile projection V7. Mirrors the IC canister's
/// `UserProfileDetailsForFrontendV7`. This is the primary profile read
/// used by the mobile app. Includes `account_type` and `profile_picture`
/// with NSFW info.
#[derive(SpacetimeType, Clone, Debug)]
pub struct UserProfileDetailsV7 {
    pub oauth_subject: String,
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
    pub oauth_subject: String,
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
    /// Display name from the yral-metadata service.
    pub username: Option<String>,
    /// Email from the yral-metadata service.
    pub email: Option<String>,
}

/// Batch upsert entry for follow relationships (backfill).
#[derive(SpacetimeType, Clone, Debug)]
pub struct UserFollowBatchEntry {
    pub follower_subject: String,
    pub followee_subject: String,
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
fn profile_picture_data(profile: &UserProfile2) -> Option<ProfilePictureData> {
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
///
/// Uses `user_follows_2` (the new table). The caller and target subjects are
/// OAuth `sub` claims.
fn follow_relationships(
    tx: &spacetimedb::TxContext,
    caller_subject: &str,
    oauth_subject: &str,
) -> (Option<bool>, Option<bool>) {
    if caller_subject == oauth_subject {
        return (None, None);
    }
    let caller_follows_user = tx
        .db
        .user_follows_2()
        .iter()
        .any(|f| f.key == format!("{caller_subject}::{oauth_subject}"));
    let user_follows_caller = tx
        .db
        .user_follows_2()
        .iter()
        .any(|f| f.key == format!("{oauth_subject}::{caller_subject}"));
    (Some(caller_follows_user), Some(user_follows_caller))
}

/// Lazy-migration helper: look up a user profile by `oauth_subject` in
/// `user_profiles_2`. If not found, check the legacy `user_profiles` table
/// (by `principal_text`), migrate the row to `user_profiles_2`, and return it.
/// Returns `None` if the user exists in neither table.
///
/// Must be called inside a `with_tx` closure.
fn get_or_migrate_profile(
    tx: &spacetimedb::TxContext,
    oauth_subject: &str,
) -> Option<UserProfile2> {
    // Check the new table first
    if let Some(profile) = tx
        .db
        .user_profiles_2()
        .iter()
        .find(|p| p.oauth_subject == oauth_subject)
    {
        return Some(profile);
    }
    // Fall back to the legacy table and migrate
    let legacy = tx
        .db
        .user_profiles()
        .iter()
        .find(|p| p.principal_text == oauth_subject)?;
    let migrated = migrate_profile_row(&legacy);
    tx.db.user_profiles_2().insert(migrated.clone());
    Some(migrated)
}

/// Convert a legacy `UserProfile` row into a `UserProfile2` row (field-for-field
/// copy with `principal_text` → `oauth_subject`). Pure function — no I/O.
fn migrate_profile_row(legacy: &UserProfile) -> UserProfile2 {
    UserProfile2 {
        oauth_subject: legacy.principal_text.clone(),
        bio: legacy.bio.clone(),
        website_url: legacy.website_url.clone(),
        profile_picture_url: legacy.profile_picture_url.clone(),
        followers_count: legacy.followers_count,
        following_count: legacy.following_count,
        subscription_plan: legacy.subscription_plan,
        is_ai_influencer: legacy.is_ai_influencer,
        is_nsfw: legacy.is_nsfw,
        nsfw_ec: legacy.nsfw_ec.clone(),
        nsfw_gore: legacy.nsfw_gore.clone(),
        csam_detected: legacy.csam_detected,
        last_access_time: legacy.last_access_time,
        account_type: legacy.account_type.clone(),
        username: legacy.username.clone(),
        email: legacy.email.clone(),
        user_id: legacy.user_id.clone(),
    }
}

/// Convert a legacy `UserFollow` row into a `UserFollow2` row (field-for-field
/// copy with `follower_text` → `follower_subject`, `followee_text` →
/// `followee_subject`). Pure function — no I/O.
fn migrate_follow_row(legacy: &UserFollow) -> UserFollow2 {
    UserFollow2 {
        key: legacy.key.clone(),
        follower_subject: legacy.follower_text.clone(),
        followee_subject: legacy.followee_text.clone(),
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Reducers (writes)
// ─────────────────────────────────────────────────────────────────────────

/// Follow another user. Bidirectional — inserts into both follower's
/// following set and followee's followers set.
#[spacetimedb::reducer]
pub fn follow_user(ctx: &ReducerContext, followee_subject: String) -> Result<(), String> {
    let follower_subject = ctx.sender_auth().jwt().expect("JWT required").subject().to_string();
    let key = format!("{follower_subject}::{followee_subject}");

    if follower_subject == followee_subject {
        return Err("Cannot follow yourself".to_string());
    }

    if ctx.db.user_follows_2().iter().any(|f| f.key == key) {
        return Err("Already following".to_string());
    }

    ctx.db.user_follows_2().insert(UserFollow2 {
        key,
        follower_subject: follower_subject.clone(),
        followee_subject: followee_subject.clone(),
    });

    // Update follower's following count
    if let Some(mut p) = ctx
        .db
        .user_profiles_2()
        .iter()
        .find(|p| p.oauth_subject == follower_subject)
    {
        p.following_count += 1;
        ctx.db.user_profiles_2().oauth_subject().update(p);
    }

    // Update followee's followers count
    if let Some(mut p) = ctx
        .db
        .user_profiles_2()
        .iter()
        .find(|p| p.oauth_subject == followee_subject)
    {
        p.followers_count += 1;
        ctx.db.user_profiles_2().oauth_subject().update(p);
    }

    Ok(())
}

/// Unfollow another user. Symmetric to follow.
#[spacetimedb::reducer]
pub fn unfollow_user(ctx: &ReducerContext, followee_subject: String) -> Result<(), String> {
    let follower_subject = ctx.sender_auth().jwt().expect("JWT required").subject().to_string();
    let key = format!("{follower_subject}::{followee_subject}");

    let Some(follow) = ctx.db.user_follows_2().iter().find(|f| f.key == key) else {
        return Err("Not following".to_string());
    };

    ctx.db.user_follows_2().delete(follow);

    // Update follower's following count
    if let Some(mut p) = ctx
        .db
        .user_profiles_2()
        .iter()
        .find(|p| p.oauth_subject == follower_subject)
    {
        p.following_count = p.following_count.saturating_sub(1);
        ctx.db.user_profiles_2().oauth_subject().update(p);
    }

    // Update followee's followers count
    if let Some(mut p) = ctx
        .db
        .user_profiles_2()
        .iter()
        .find(|p| p.oauth_subject == followee_subject)
    {
        p.followers_count = p.followers_count.saturating_sub(1);
        ctx.db.user_profiles_2().oauth_subject().update(p);
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
    let oauth_subject = ctx.sender_auth().jwt().expect("JWT required").subject().to_string();

    let mut profile = match ctx
        .db
        .user_profiles_2()
        .iter()
        .find(|p| p.oauth_subject == oauth_subject)
    {
        Some(p) => p,
        None => return Err("User not found".to_string()),
    };

    profile.bio = bio;
    profile.website_url = website_url;
    profile.profile_picture_url = profile_pic_url;
    ctx.db.user_profiles_2().oauth_subject().update(profile);

    Ok(())
}

/// Admin-only: update the AI influencer status for a user.
#[spacetimedb::reducer]
pub fn update_profile_ai_influencer_status(
    ctx: &ReducerContext,
    oauth_subject: String,
    is_ai_influencer: bool,
) -> Result<(), String> {
    if !crate::constants::ADMINS.contains(&ctx.sender()) {
        return Err("Unauthorized".to_string());
    }

    let mut profile = match ctx
        .db
        .user_profiles_2()
        .iter()
        .find(|p| p.oauth_subject == oauth_subject)
    {
        Some(p) => p,
        None => return Err("User not found".to_string()),
    };

    profile.is_ai_influencer = is_ai_influencer;
    ctx.db.user_profiles_2().oauth_subject().update(profile);

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
    let oauth_subject = ctx.sender_auth().jwt().expect("JWT required").subject().to_string();

    let mut profile = match ctx
        .db
        .user_profiles_2()
        .iter()
        .find(|p| p.oauth_subject == oauth_subject)
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
    ctx.db.user_profiles_2().oauth_subject().update(profile);

    Ok(())
}

/// Admin-only: update the NSFW info on a user's profile picture.
/// Mirrors the IC canister's `update_profile_picture_nsfw_info`.
#[spacetimedb::reducer]
pub fn update_profile_picture_nsfw_info(
    ctx: &ReducerContext,
    oauth_subject: String,
    nsfw_info: NSFWInfo,
) -> Result<(), String> {
    if !crate::constants::ADMINS.contains(&ctx.sender()) {
        return Err("Unauthorized".to_string());
    }

    let mut profile = match ctx
        .db
        .user_profiles_2()
        .iter()
        .find(|p| p.oauth_subject == oauth_subject)
    {
        Some(p) => p,
        None => return Err("User not found".to_string()),
    };

    profile.is_nsfw = nsfw_info.is_nsfw;
    profile.nsfw_ec = nsfw_info.nsfw_ec;
    profile.nsfw_gore = nsfw_info.nsfw_gore;
    profile.csam_detected = nsfw_info.csam_detected;
    ctx.db.user_profiles_2().oauth_subject().update(profile);

    Ok(())
}

/// Register a new user or create a bot account. Mirrors the IC canister's
/// `accept_new_user_registration_v2`.
///
/// - When `main_account_text` is `Some(owner)`: creates a bot account owned
///   by `owner` (owner must exist and be a `MainAccount`). The bot is added
///   to the owner's `bots` list.
/// - When `main_account_text` is `None`: creates a normal account for
///   `new_principal_text` (stored as `oauth_subject` in `user_profiles_2`).
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
        .user_profiles_2()
        .iter()
        .any(|p| p.oauth_subject == new_principal_text)
    {
        return Ok(());
    }

    match main_account_text {
        Some(owner_text) => {
            // Bot account creation — owner must exist and be a MainAccount
            let mut owner_profile = match ctx
                .db
                .user_profiles_2()
                .iter()
                .find(|p| p.oauth_subject == owner_text)
            {
                Some(p) => p,
                None => return Err("Owner not found".to_string()),
            };
            match &owner_profile.account_type {
                UserAccountType::MainAccount { .. } => {
                    owner_profile.account_type = validate_owner_for_bot_creation(
                        &owner_profile.account_type,
                        &new_principal_text,
                    )
                    .expect("validated above");
                    ctx.db.user_profiles_2().oauth_subject().update(owner_profile);
                }
                UserAccountType::BotAccount { .. } => {
                    return Err("Bots cannot own other bots".to_string());
                }
            }

            // Create the bot account
            ctx.db.user_profiles_2().insert(build_bot_profile(
                new_principal_text,
                owner_text,
                ctx.timestamp,
            ));
        }
        None => {
            // Normal account registration
            ctx.db.user_profiles_2().insert(build_main_account_profile(
                new_principal_text,
                ctx.timestamp,
            ));
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
    let caller_text = ctx.sender_auth().jwt().expect("JWT required").subject().to_string();
    let admin = crate::constants::ADMINS.contains(&ctx.sender());

    let profile = match ctx
        .db
        .user_profiles_2()
        .iter()
        .find(|p| p.oauth_subject == principal_to_delete_text)
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
                    .user_profiles_2()
                    .iter()
                    .find(|p| &p.oauth_subject == bot_text)
                {
                    ctx.db.user_profiles_2().delete(bot_profile);
                }
            }
            // Delete the main account
            ctx.db.user_profiles_2().delete(profile);
        }
        UserAccountType::BotAccount { owner } => {
            // Admin or owner can delete a BotAccount
            if !admin && owner != &caller_text {
                return Err("Unauthorized".to_string());
            }
            // Remove this bot from the owner's bots list
            if let Some(mut owner_profile) = ctx
                .db
                .user_profiles_2()
                .iter()
                .find(|p| p.oauth_subject == *owner)
            {
                if let UserAccountType::MainAccount { bots } = &owner_profile.account_type {
                    let new_bots: Vec<String> = bots
                        .iter()
                        .filter(|b| *b != &principal_to_delete_text)
                        .cloned()
                        .collect();
                    owner_profile.account_type = UserAccountType::MainAccount { bots: new_bots };
                    ctx.db.user_profiles_2().oauth_subject().update(owner_profile);
                }
            }
            // Delete the bot
            ctx.db.user_profiles_2().delete(profile);
        }
    }

    // Also delete any follow relationships involving this user
    let follows_to_delete: Vec<UserFollow2> = ctx
        .db
        .user_follows_2()
        .iter()
        .filter(|f| {
            f.follower_subject == principal_to_delete_text
                || f.followee_subject == principal_to_delete_text
        })
        .collect();
    for f in follows_to_delete {
        ctx.db.user_follows_2().delete(f);
    }

    Ok(())
}

/// Update the caller's last access time to the current timestamp.
#[spacetimedb::reducer]
pub fn update_user_last_access_time(ctx: &ReducerContext) -> Result<(), String> {
    let oauth_subject = ctx.sender_auth().jwt().expect("JWT required").subject().to_string();

    let mut profile = match ctx
        .db
        .user_profiles_2()
        .iter()
        .find(|p| p.oauth_subject == oauth_subject)
    {
        Some(p) => p,
        None => return Err("User not found".to_string()),
    };

    profile.last_access_time = ctx.timestamp;
    ctx.db.user_profiles_2().oauth_subject().update(profile);

    Ok(())
}

/// Admin-only: change a user's subscription plan.
#[spacetimedb::reducer]
pub fn change_subscription_plan(
    ctx: &ReducerContext,
    oauth_subject: String,
    plan: SubscriptionPlan,
) -> Result<(), String> {
    if !crate::constants::ADMINS.contains(&ctx.sender()) {
        return Err("Unauthorized".to_string());
    }

    let mut profile = match ctx
        .db
        .user_profiles_2()
        .iter()
        .find(|p| p.oauth_subject == oauth_subject)
    {
        Some(p) => p,
        None => return Err("User not found".to_string()),
    };

    profile.subscription_plan = plan;
    ctx.db.user_profiles_2().oauth_subject().update(profile);

    Ok(())
}

/// Admin-only: add Pro plan free video credits to a user.
#[spacetimedb::reducer]
pub fn add_pro_plan_free_video_credits(
    ctx: &ReducerContext,
    oauth_subject: String,
    credits: u32,
) -> Result<(), String> {
    if !crate::constants::ADMINS.contains(&ctx.sender()) {
        return Err("Unauthorized".to_string());
    }

    let mut profile = match ctx
        .db
        .user_profiles_2()
        .iter()
        .find(|p| p.oauth_subject == oauth_subject)
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
    ctx.db.user_profiles_2().oauth_subject().update(profile);

    Ok(())
}

/// Admin-only: remove Pro plan free video credits from a user.
#[spacetimedb::reducer]
pub fn remove_pro_plan_free_video_credits(
    ctx: &ReducerContext,
    oauth_subject: String,
    credits: u32,
) -> Result<(), String> {
    if !crate::constants::ADMINS.contains(&ctx.sender()) {
        return Err("Unauthorized".to_string());
    }

    let mut profile = match ctx
        .db
        .user_profiles_2()
        .iter()
        .find(|p| p.oauth_subject == oauth_subject)
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
    ctx.db.user_profiles_2().oauth_subject().update(profile);

    Ok(())
}

/// Admin-only: batch upsert user profiles (for IC → SpacetimeDB backfill).
/// Writes to `user_profiles_2`. Idempotent — delete-then-insert by primary key.
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
            .user_profiles_2()
            .iter()
            .find(|p| p.oauth_subject == entry.oauth_subject)
        {
            ctx.db.user_profiles_2().delete(existing);
        }

        ctx.db.user_profiles_2().insert(UserProfile2 {
            oauth_subject: entry.oauth_subject,
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
            user_id: None,
        });
    }

    Ok(())
}

/// Admin-only: batch upsert follow relationships (for IC → SpacetimeDB backfill).
/// Writes to `user_follows_2`. Idempotent — delete-then-insert by primary key.
#[spacetimedb::reducer]
pub fn upsert_user_follow_batch(
    ctx: &ReducerContext,
    follows: Vec<UserFollowBatchEntry>,
) -> Result<(), String> {
    if !crate::constants::ADMINS.contains(&ctx.sender()) {
        return Err("Unauthorized".to_string());
    }

    for entry in follows {
        let key = format!("{}::{}", entry.follower_subject, entry.followee_subject);
        // Delete existing if present
        if let Some(existing) = ctx.db.user_follows_2().iter().find(|f| f.key == key) {
            ctx.db.user_follows_2().delete(existing);
        }

        ctx.db.user_follows_2().insert(UserFollow2 {
            key,
            follower_subject: entry.follower_subject,
            followee_subject: entry.followee_subject,
        });
    }

    Ok(())
}

/// Admin-only: batch backfill reducer that copies rows from the legacy
/// `user_profiles` and `user_follows` tables into the new `user_profiles_2`
/// and `user_follows_2` tables. Idempotent — skips rows that already exist in
/// the new tables.
///
/// Processes at most `batch_limit` rows per call to stay within the module
/// energy budget. Call repeatedly until no more rows are migrated.
/// Reducers cannot return values, so check progress by querying
/// `SELECT count(*) FROM user_profiles_2` between calls.
#[spacetimedb::reducer]
pub fn migrate_user_profiles_to_2(
    ctx: &ReducerContext,
    batch_limit: u32,
) -> Result<(), String> {
    if !crate::constants::ADMINS.contains(&ctx.sender()) {
        return Err("Unauthorized".to_string());
    }

    let mut migrated_count: u32 = 0;

    // Migrate user_profiles → user_profiles_2 (skip rows already present)
    for legacy_profile in ctx.db.user_profiles().iter() {
        if migrated_count >= batch_limit {
            return Ok(());
        }
        // Use primary key lookup instead of full scan — O(1) not O(n)
        if ctx
            .db
            .user_profiles_2()
            .oauth_subject()
            .find(legacy_profile.principal_text.clone())
            .is_none()
        {
            ctx.db
                .user_profiles_2()
                .insert(migrate_profile_row(&legacy_profile));
            migrated_count += 1;
        }
    }

    // Migrate user_follows → user_follows_2 (skip rows already present)
    for legacy_follow in ctx.db.user_follows().iter() {
        if migrated_count >= batch_limit {
            return Ok(());
        }
        // Use primary key lookup instead of full scan
        if ctx
            .db
            .user_follows_2()
            .key()
            .find(legacy_follow.key.clone())
            .is_none()
        {
            ctx.db
                .user_follows_2()
                .insert(migrate_follow_row(&legacy_follow));
            migrated_count += 1;
        }
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Notification token reducers
// ─────────────────────────────────────────────────────────────────────────

/// Register a device notification token for the caller.
/// Idempotent — if the token already exists, it's a no-op.
#[spacetimedb::reducer]
pub fn register_notification_token(ctx: &ReducerContext, token: String) -> Result<(), String> {
    let user_id = ctx.sender_auth().jwt().expect("JWT required").subject().to_string();
    let key = format!("{user_id}::{token}");

    // Check if already exists
    if ctx
        .db
        .user_notification_tokens()
        .iter()
        .any(|t| t.key == key)
    {
        return Ok(());
    }

    ctx.db
        .user_notification_tokens()
        .insert(UserNotificationToken {
            key,
            user_id,
            token,
        });

    Ok(())
}

/// Unregister a device notification token for the caller.
#[spacetimedb::reducer]
pub fn unregister_notification_token(ctx: &ReducerContext, token: String) -> Result<(), String> {
    let user_id = ctx.sender_auth().jwt().expect("JWT required").subject().to_string();
    let key = format!("{user_id}::{token}");

    if let Some(existing) = ctx
        .db
        .user_notification_tokens()
        .iter()
        .find(|t| t.key == key)
    {
        ctx.db.user_notification_tokens().delete(existing);
    }

    Ok(())
}

/// Link a user profile to an OAuth user_id. Called by yral-auth after
/// OAuth login to associate the SpacetimeDB identity with the user's
/// OAuth sub. Admin-only.
#[spacetimedb::reducer]
pub fn link_user_id(
    ctx: &ReducerContext,
    oauth_subject: String,
    user_id: String,
) -> Result<(), String> {
    if !crate::constants::ADMINS.contains(&ctx.sender()) {
        return Err("Unauthorized".to_string());
    }

    if let Some(mut profile) = ctx
        .db
        .user_profiles_2()
        .iter()
        .find(|p| p.oauth_subject == oauth_subject)
    {
        profile.user_id = Some(user_id);
        let profile_clone = profile.clone();
        ctx.db.user_profiles_2().delete(profile);
        ctx.db.user_profiles_2().insert(profile_clone);
    }

    Ok(())
}

/// Set username for a user profile. Admin-only (called by yral-metadata).
/// Validates username format: 3-15 alphanumeric characters.
#[spacetimedb::reducer]
pub fn set_username(
    ctx: &ReducerContext,
    oauth_subject: String,
    username: String,
) -> Result<(), String> {
    if !crate::constants::ADMINS.contains(&ctx.sender()) {
        return Err("Unauthorized".to_string());
    }

    if username.is_empty() {
        return Err("Username cannot be empty".to_string());
    }

    // Validate: 3-15 alphanumeric characters
    if !username.chars().all(|c| c.is_ascii_alphanumeric())
        || username.len() < 3
        || username.len() > 15
    {
        return Err("Invalid username: must be 3-15 alphanumeric characters".to_string());
    }

    // Check for duplicate username
    if ctx
        .db
        .user_profiles_2()
        .iter()
        .any(|p| p.username.as_deref() == Some(&username))
    {
        return Err("DuplicateUsername".to_string());
    }

    if let Some(mut profile) = ctx
        .db
        .user_profiles_2()
        .iter()
        .find(|p| p.oauth_subject == oauth_subject)
    {
        // Remove old username association if any
        profile.username = Some(username);
        let profile_clone = profile.clone();
        ctx.db.user_profiles_2().delete(profile);
        ctx.db.user_profiles_2().insert(profile_clone);
    } else {
        return Err("User not found".to_string());
    }

    Ok(())
}

/// Set email for a user profile. Admin-only (called by yral-metadata).
#[spacetimedb::reducer]
pub fn set_email(
    ctx: &ReducerContext,
    oauth_subject: String,
    email: String,
) -> Result<(), String> {
    if !crate::constants::ADMINS.contains(&ctx.sender()) {
        return Err("Unauthorized".to_string());
    }

    if let Some(mut profile) = ctx
        .db
        .user_profiles_2()
        .iter()
        .find(|p| p.oauth_subject == oauth_subject)
    {
        profile.email = Some(email);
        let profile_clone = profile.clone();
        ctx.db.user_profiles_2().delete(profile);
        ctx.db.user_profiles_2().insert(profile_clone);
    } else {
        return Err("User not found".to_string());
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Procedures (reads — return typed data)
// ─────────────────────────────────────────────────────────────────────────

/// Get profile details for a user. Returns `None` if the user doesn't exist.
/// The `caller_follows_user` and `user_follows_caller` fields are computed
/// from the `user_follows_2` table using `ctx.sender()`.
#[spacetimedb::procedure]
pub fn get_profile_details_v4(
    ctx: &mut ProcedureContext,
    oauth_subject: String,
) -> Option<UserProfileDetailsV4> {
    ctx.with_tx(|tx| {
        let profile = get_or_migrate_profile(tx, &oauth_subject)?;
        let caller_subject = tx.sender_auth().jwt().expect("JWT required").subject().to_string();

        let caller_follows_user = tx
            .db
            .user_follows_2()
            .iter()
            .any(|f| f.key == format!("{caller_subject}::{oauth_subject}"));

        let user_follows_caller = tx
            .db
            .user_follows_2()
            .iter()
            .any(|f| f.key == format!("{oauth_subject}::{caller_subject}"));

        Some(UserProfileDetailsV4 {
            oauth_subject: profile.oauth_subject,
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
    oauth_subject: String,
) -> Option<UserProfileDetailsV7> {
    ctx.with_tx(|tx| {
        let profile = get_or_migrate_profile(tx, &oauth_subject)?;
        let caller_subject = tx.sender_auth().jwt().expect("JWT required").subject().to_string();

        let (caller_follows_user, user_follows_caller) =
            follow_relationships(tx, &caller_subject, &oauth_subject);

        let oauth_subject = profile.oauth_subject.clone();
        let profile_picture = profile_picture_data(&profile);

        Some(UserProfileDetailsV7 {
            oauth_subject,
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

/// Batch profile lookup. Returns V7 profile details for each subject.
/// Users that are not found are silently skipped (matches IC canister
/// behavior). Follow relationships are computed using `ctx.sender()`.
#[spacetimedb::procedure]
pub fn get_users_profile_details(
    ctx: &mut ProcedureContext,
    oauth_subjects: Vec<String>,
) -> Vec<UserProfileDetailsV7> {
    ctx.with_tx(|tx| {
        let caller_subject = tx.sender_auth().jwt().expect("JWT required").subject().to_string();

        oauth_subjects
            .iter()
            .filter_map(|oauth_subject| {
                let profile = get_or_migrate_profile(tx, oauth_subject)?;

                let (caller_follows_user, user_follows_caller) =
                    follow_relationships(tx, &caller_subject, oauth_subject);

                let oauth_subject = profile.oauth_subject.clone();
                let profile_picture = profile_picture_data(&profile);

                Some(UserProfileDetailsV7 {
                    oauth_subject,
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
    oauth_subject: String,
    limit: u64,
    cursor: Option<String>,
) -> FollowersPage {
    ctx.with_tx(|tx| {
        let limit = limit.min(MAX_PAGE_SIZE) as usize;
        let caller_subject =
            tx.sender_auth().jwt().expect("JWT required").subject().to_string();

        // Collect all followers of this user (followee = oauth_subject)
        let mut followers: Vec<UserFollow2> = tx
            .db
            .user_follows_2()
            .iter()
            .filter(|f| f.followee_subject == oauth_subject)
            .collect();

        // Sort by follower_subject for stable pagination
        followers.sort_by(|a, b| a.follower_subject.cmp(&b.follower_subject));

        // Find cursor position
        let start = match &cursor {
            Some(cursor_id) => followers
                .iter()
                .position(|f| f.follower_subject.as_str() > cursor_id.as_str())
                .unwrap_or(0),
            None => 0,
        };

        let page: Vec<UserFollow2> =
            followers.iter().skip(start).take(limit).cloned().collect();
        let next_cursor = if start + limit < followers.len() {
            page.last().map(|f| f.follower_subject.clone())
        } else {
            None
        };

        let total_count = followers.len() as u64;

        // Build follower items with profile pics and follow status
        let items: Vec<FollowerItem> = page
            .iter()
            .map(|f| {
                let profile = get_or_migrate_profile(tx, &f.follower_subject);
                let pic =
                    profile.map(|p| p.profile_picture_url).unwrap_or_default();
                let caller_follows = tx
                    .db
                    .user_follows_2()
                    .iter()
                    .any(|uf| uf.key == format!("{caller_subject}::{}", f.follower_subject));
                FollowerItem {
                    oauth_subject: f.follower_subject.clone(),
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
    oauth_subject: String,
    limit: u64,
    cursor: Option<String>,
) -> FollowingPage {
    ctx.with_tx(|tx| {
        let limit = limit.min(MAX_PAGE_SIZE) as usize;
        let caller_subject =
            tx.sender_auth().jwt().expect("JWT required").subject().to_string();

        // Collect all users this user is following (follower = oauth_subject)
        let mut following: Vec<UserFollow2> = tx
            .db
            .user_follows_2()
            .iter()
            .filter(|f| f.follower_subject == oauth_subject)
            .collect();

        // Sort by followee_subject for stable pagination
        following.sort_by(|a, b| a.followee_subject.cmp(&b.followee_subject));

        // Find cursor position
        let start = match &cursor {
            Some(cursor_id) => following
                .iter()
                .position(|f| f.followee_subject.as_str() > cursor_id.as_str())
                .unwrap_or(0),
            None => 0,
        };

        let page: Vec<UserFollow2> =
            following.iter().skip(start).take(limit).cloned().collect();
        let next_cursor = if start + limit < following.len() {
            page.last().map(|f| f.followee_subject.clone())
        } else {
            None
        };

        let total_count = following.len() as u64;

        // Build following items with profile pics and follow status
        let items: Vec<FollowingItem> = page
            .iter()
            .map(|f| {
                let profile = get_or_migrate_profile(tx, &f.followee_subject);
                let pic =
                    profile.map(|p| p.profile_picture_url).unwrap_or_default();
                let caller_follows = tx
                    .db
                    .user_follows_2()
                    .iter()
                    .any(|uf| uf.key == format!("{caller_subject}::{}", f.followee_subject));
                FollowingItem {
                    oauth_subject: f.followee_subject.clone(),
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

/// Get all notification tokens for a user (by user_id).
/// Used by the push notification service to send FCM notifications.
#[spacetimedb::procedure]
pub fn get_notification_tokens(ctx: &mut ProcedureContext, user_id: String) -> Vec<String> {
    ctx.with_tx(|tx| {
        tx.db
            .user_notification_tokens()
            .iter()
            .filter(|t| t.user_id == user_id)
            .map(|t| t.token)
            .collect()
    })
}

/// Look up a user profile by OAuth user_id (sub).
/// Returns the profile details if found, `None` otherwise.
/// Used by yral-auth and services that have the OAuth sub but need
/// the full profile.
#[spacetimedb::procedure]
pub fn get_user_profile_by_user_id(
    ctx: &mut ProcedureContext,
    user_id: String,
) -> Option<UserProfileDetailsV7> {
    ctx.with_tx(|tx| {
        let profile = tx
            .db
            .user_profiles_2()
            .iter()
            .find(|p| p.user_id.as_ref() == Some(&user_id))?;

        let caller_subject =
            tx.sender_auth().jwt().expect("JWT required").subject().to_string();
        let (caller_follows_user, user_follows_caller) =
            follow_relationships(tx, &caller_subject, &profile.oauth_subject);

        let oauth_subject = profile.oauth_subject.clone();
        let profile_picture = profile_picture_data(&profile);

        Some(UserProfileDetailsV7 {
            oauth_subject,
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

/// Look up a user profile by username.
/// Returns the profile details if found, `None` otherwise.
#[spacetimedb::procedure]
pub fn get_user_profile_by_username(
    ctx: &mut ProcedureContext,
    username: String,
) -> Option<UserProfileDetailsV7> {
    ctx.with_tx(|tx| {
        let profile = tx
            .db
            .user_profiles_2()
            .iter()
            .find(|p| p.username.as_deref() == Some(&username))?;

        let caller_subject =
            tx.sender_auth().jwt().expect("JWT required").subject().to_string();
        let (caller_follows_user, user_follows_caller) =
            follow_relationships(tx, &caller_subject, &profile.oauth_subject);

        let oauth_subject = profile.oauth_subject.clone();
        let profile_picture = profile_picture_data(&profile);

        Some(UserProfileDetailsV7 {
            oauth_subject,
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

// ─────────────────────────────────────────────────────────────────────────
// Pure helpers for bot registration logic (extracted from the reducer so
// they can be unit-tested without a ReducerContext).
// ─────────────────────────────────────────────────────────────────────────

/// Validate the owner for a bot-account creation. Returns the updated
/// `UserAccountType` with the new bot appended to the `bots` list, or
/// an error if the owner is a `BotAccount` (bots cannot own bots).
fn validate_owner_for_bot_creation(
    owner_account_type: &UserAccountType,
    new_bot_subject: &str,
) -> Result<UserAccountType, String> {
    match owner_account_type {
        UserAccountType::MainAccount { bots } => {
            let mut updated_bots = bots.clone();
            updated_bots.push(new_bot_subject.to_string());
            Ok(UserAccountType::MainAccount { bots: updated_bots })
        }
        UserAccountType::BotAccount { .. } => Err("Bots cannot own other bots".to_string()),
    }
}

/// Build a new bot `UserProfile2` with default values, owned by `owner_subject`.
fn build_bot_profile(
    bot_subject: String,
    owner_subject: String,
    timestamp: Timestamp,
) -> UserProfile2 {
    UserProfile2 {
        oauth_subject: bot_subject,
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
        last_access_time: timestamp,
        account_type: UserAccountType::BotAccount {
            owner: owner_subject,
        },
        username: None,
        email: None,
        user_id: None,
    }
}

/// Build a new main-account `UserProfile2` with default values.
fn build_main_account_profile(oauth_subject: String, timestamp: Timestamp) -> UserProfile2 {
    UserProfile2 {
        oauth_subject,
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
        last_access_time: timestamp,
        account_type: UserAccountType::MainAccount { bots: Vec::new() },
        username: None,
        email: None,
        user_id: None,
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_TIMESTAMP: Timestamp = Timestamp::UNIX_EPOCH;

    // ── validate_owner_for_bot_creation ──

    #[test]
    fn test_validate_owner_main_account_with_no_bots() {
        let owner_type = UserAccountType::MainAccount { bots: Vec::new() };
        let result = validate_owner_for_bot_creation(&owner_type, "bot-1");
        assert!(result.is_ok());
        let updated = result.unwrap();
        match updated {
            UserAccountType::MainAccount { bots } => {
                assert_eq!(bots, vec!["bot-1"]);
            }
            UserAccountType::BotAccount { .. } => panic!("Expected MainAccount"),
        }
    }

    #[test]
    fn test_validate_owner_main_account_with_existing_bots() {
        let owner_type = UserAccountType::MainAccount {
            bots: vec!["existing-bot-1".to_string(), "existing-bot-2".to_string()],
        };
        let result = validate_owner_for_bot_creation(&owner_type, "new-bot");
        assert!(result.is_ok());
        let updated = result.unwrap();
        match updated {
            UserAccountType::MainAccount { bots } => {
                assert_eq!(bots, vec!["existing-bot-1", "existing-bot-2", "new-bot"]);
            }
            UserAccountType::BotAccount { .. } => panic!("Expected MainAccount"),
        }
    }

    #[test]
    fn test_validate_owner_bot_account_rejected() {
        let owner_type = UserAccountType::BotAccount {
            owner: "someone-else".to_string(),
        };
        let result = validate_owner_for_bot_creation(&owner_type, "new-bot");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Bots cannot own other bots");
    }

    #[test]
    fn test_validate_owner_does_not_mutate_original() {
        let owner_type = UserAccountType::MainAccount {
            bots: vec!["bot-a".to_string()],
        };
        let _ = validate_owner_for_bot_creation(&owner_type, "bot-b");
        match owner_type {
            UserAccountType::MainAccount { bots } => {
                assert_eq!(bots, vec!["bot-a"]);
            }
            _ => panic!("Expected MainAccount"),
        }
    }

    // ── build_bot_profile ──

    #[test]
    fn test_build_bot_profile_defaults() {
        let profile = build_bot_profile(
            "bot-subject".to_string(),
            "owner-subject".to_string(),
            TEST_TIMESTAMP,
        );
        assert_eq!(profile.oauth_subject, "bot-subject");
        assert_eq!(profile.bio, "");
        assert_eq!(profile.website_url, "");
        assert_eq!(profile.profile_picture_url, "");
        assert_eq!(profile.followers_count, 0);
        assert_eq!(profile.following_count, 0);
        assert_eq!(profile.subscription_plan, SubscriptionPlan::Free);
        assert!(!profile.is_ai_influencer);
        assert!(!profile.is_nsfw);
        assert_eq!(profile.nsfw_ec, "");
        assert_eq!(profile.nsfw_gore, "");
        assert!(!profile.csam_detected);
        assert_eq!(profile.last_access_time, TEST_TIMESTAMP);
        assert_eq!(profile.username, None);
        assert_eq!(profile.email, None);
        assert_eq!(profile.user_id, None);
    }

    #[test]
    fn test_build_bot_profile_account_type() {
        let profile = build_bot_profile(
            "bot-123".to_string(),
            "owner-456".to_string(),
            TEST_TIMESTAMP,
        );
        match &profile.account_type {
            UserAccountType::BotAccount { owner } => {
                assert_eq!(owner, "owner-456");
            }
            UserAccountType::MainAccount { .. } => {
                panic!("Expected BotAccount, got MainAccount")
            }
        }
    }

    // ── build_main_account_profile ──

    #[test]
    fn test_build_main_account_profile_defaults() {
        let profile = build_main_account_profile("main-subject".to_string(), TEST_TIMESTAMP);
        assert_eq!(profile.oauth_subject, "main-subject");
        assert_eq!(profile.followers_count, 0);
        assert_eq!(profile.following_count, 0);
        assert_eq!(profile.subscription_plan, SubscriptionPlan::Free);
        assert!(!profile.is_ai_influencer);
        assert_eq!(profile.username, None);
        assert_eq!(profile.email, None);
        assert_eq!(profile.user_id, None);
    }

    #[test]
    fn test_build_main_account_profile_account_type() {
        let profile = build_main_account_profile("main-user".to_string(), TEST_TIMESTAMP);
        match &profile.account_type {
            UserAccountType::MainAccount { bots } => {
                assert!(bots.is_empty());
            }
            UserAccountType::BotAccount { .. } => {
                panic!("Expected MainAccount, got BotAccount")
            }
        }
    }

    // ── profile_picture_data ──

    #[test]
    fn test_profile_picture_data_empty_url() {
        let profile = build_main_account_profile("test".to_string(), TEST_TIMESTAMP);
        assert!(profile_picture_data(&profile).is_none());
    }

    #[test]
    fn test_profile_picture_data_with_url() {
        let profile = UserProfile2 {
            oauth_subject: "test".to_string(),
            bio: String::new(),
            website_url: String::new(),
            profile_picture_url: "https://example.com/pic.jpg".to_string(),
            followers_count: 0,
            following_count: 0,
            subscription_plan: SubscriptionPlan::Free,
            is_ai_influencer: false,
            is_nsfw: true,
            nsfw_ec: "ec-value".to_string(),
            nsfw_gore: "gore-value".to_string(),
            csam_detected: false,
            last_access_time: TEST_TIMESTAMP,
            account_type: UserAccountType::MainAccount { bots: Vec::new() },
            username: None,
            email: None,
            user_id: None,
        };
        let pic_data = profile_picture_data(&profile).unwrap();
        assert_eq!(pic_data.url, "https://example.com/pic.jpg");
        assert!(pic_data.nsfw_info.is_nsfw);
        assert_eq!(pic_data.nsfw_info.nsfw_ec, "ec-value");
        assert_eq!(pic_data.nsfw_info.nsfw_gore, "gore-value");
        assert!(!pic_data.nsfw_info.csam_detected);
    }

    // ── UserAccountType Default ──

    #[test]
    fn test_user_account_type_default_is_main_account() {
        let default = UserAccountType::default();
        match default {
            UserAccountType::MainAccount { bots } => {
                assert!(bots.is_empty());
            }
            UserAccountType::BotAccount { .. } => {
                panic!("Default should be MainAccount")
            }
        }
    }
}
