//! User info service — migrated from the IC `user_info_service` canister
//! (`ivkka-7qaaa-aaaas-qbg3q-cai`).
//!
//! ## Tables
//! - `user_profiles`: keyed by `principal_text`, stores profile/follow data
//! - `user_follows`: keyed by `(follower_text, followee_text)`, bidirectional
//!
//! ## Procedures (reads, return typed data)
//! - `get_profile_details_v4(principal_text) -> Option<UserProfileDetailsV4>`
//! - `get_followers(principal_text, limit, cursor) -> FollowersPage`
//! - `get_following(principal_text, limit, cursor) -> FollowingPage`
//!
//! ## Reducers (writes)
//! - `register_new_user()` — idempotent, creates empty profile for `ctx.sender()`
//! - `follow_user(followee_text)` — bidirectional follow
//! - `unfollow_user(followee_text)` — symmetric
//! - `update_profile_details(bio, website_url, profile_pic_url)` — sender only
//! - `update_profile_ai_influencer_status(principal_text, is_ai)` — admin only

use spacetimedb::{Identity, ProcedureContext, ReducerContext, SpacetimeType, Table, Timestamp};

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

#[derive(SpacetimeType, Clone, Debug, PartialEq, Eq)]
pub enum SubscriptionPlan {
    Free,
    Pro,
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

// ─────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────

/// Maximum number of items per page (matches IC canister's max).
const MAX_PAGE_SIZE: u64 = 100;

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
        followee_text,
    });

    // Update follower's following count
    let mut profiles: Vec<UserProfile> = ctx
        .db
        .user_profiles()
        .iter()
        .filter(|p| p.principal_text == follower_text)
        .collect();
    if let Some(mut p) = profiles.pop() {
        p.following_count += 1;
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

    if ctx.db.user_follows().iter().all(|f| f.key != key) {
        return Err("Not following".to_string());
    }

    // Delete the follow relationship
    let existing: Vec<UserFollow> = ctx
        .db
        .user_follows()
        .iter()
        .filter(|f| f.key == key)
        .collect();
    for f in existing {
        ctx.db.user_follows().delete(f);
    }

    // Update follower's following count
    let mut profiles: Vec<UserProfile> = ctx
        .db
        .user_profiles()
        .iter()
        .filter(|p| p.principal_text == follower_text)
        .collect();
    if let Some(mut p) = profiles.pop() {
        p.following_count = p.following_count.saturating_sub(1);
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

    let mut profile: Vec<UserProfile> = ctx
        .db
        .user_profiles()
        .iter()
        .filter(|p| p.principal_text == principal_text)
        .collect();
    if profile.is_empty() {
        return Err("User not found".to_string());
    }
    let mut profile = profile.remove(0);

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

    let mut profile: Vec<UserProfile> = ctx
        .db
        .user_profiles()
        .iter()
        .filter(|p| p.principal_text == principal_text)
        .collect();
    if profile.is_empty() {
        return Err("User not found".to_string());
    }
    let mut profile = profile.remove(0);

    profile.is_ai_influencer = is_ai_influencer;
    ctx.db.user_profiles().delete(profile.clone());
    ctx.db.user_profiles().insert(profile);

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