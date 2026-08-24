//! Posts feature — migrated from the IC `@user_post_service` canister
//! (`gxhc3-pqaaa-aaaas-qbh3q-cai`).
//!
//! Reimplements only the methods called by in-repo consumers (see migration
//! plan in `/memories/session/plan-v2.md`). Methods with zero callers were
//! dropped: `sync_post_from_individual_canister`,
//! `initialize_posts_by_creator_index_batch`, `update_post_increment_share_count`,
//! `update_post_toggle_like_status_by_caller`.
//!
//! ## Likes dropped
//! `liked_by_me` and `like_count` in `PostDetailsForFrontend` always return
//! `false` / `0` (display fields, no backing data). The `likes` column is
//! not present on the `Post` table.
//!
//! ## Client access patterns
//! - **Rust services** (off-chain-agent, yral-web): use generated
//!   `spacetimedb-sdk` bindings (typed reducer/procedure calls).
//! - **Mobile** (Kotlin): calls procedures via REST
//!   (`POST /v1/database/{db}/call/:name`, JSON array body → typed JSON
//!   `SpacetimeType` return). Reducers for writes, procedures for reads.
//!
//! ## Admin model
//! Admin identities are hardcoded as a `const ADMINS` array in
//! `constants.rs`. Admin-only reducers (the `upsert_*` family) compare
//! `ctx.sender()` against that list.
//!
//! `add_post`, `update_post_status` and `delete_post` are admin **or** the
//! post's own creator. That lets a backend register and publish a post while
//! acting as the user — by forwarding the user's yral-auth `id_token`, which
//! already carries `ext_spacetimedb_token` — rather than holding the shared
//! admin token and its far wider blast radius.
//!
//! To add/remove an admin, edit the `ADMINS` constant and republish.

use spacetimedb::{
    Identity, ProcedureContext, ReducerContext, SpacetimeType, Table, Timestamp,
};

// ─────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────

/// Post lifecycle status. Mirrors the IC canister's `PostStatus` enum.
#[derive(SpacetimeType, Clone, Debug, PartialEq, Eq, Default)]
pub enum PostStatus {
    #[default]
    Uploaded,
    Transcoding,
    CheckingExplicitness,
    BannedForExplicitness,
    ReadyToView,
    BannedDueToUserReporting,
    Deleted,
    Draft,
}

/// View event details sent by the off-chain-agent when ingesting analytics.
/// Mirrors the IC canister's `PostViewDetailsFromFrontend` enum.
///
/// SpacetimeType only supports unit-variant and newtype-variant enums (confirmed
/// via compiler error: "must be a unit variant or a newtype variant"). The IC
/// canister's `WatchedPartially { percentage_watched }` /
/// `WatchedMultipleTimes { watch_count, percentage_watched }` struct variants
/// are flattened into a struct. `watch_count == 0` means WatchedPartially;
/// `watch_count > 0` means WatchedMultipleTimes (no separate discriminator needed).
#[derive(SpacetimeType, Clone, Debug)]
pub struct PostViewDetailsFromFrontend {
    /// Percentage of the video watched (1-100). Always required.
    pub percentage_watched: u8,
    /// Number of complete watches (excluding the partial one).
    /// `0` = WatchedPartially; `> 0` = WatchedMultipleTimes.
    pub watch_count: u8,
}

// ─────────────────────────────────────────────────────────────────────────
// Tables
// ─────────────────────────────────────────────────────────────────────────

/// A post (video with metadata). Mirrors the IC canister's `Post` struct
/// minus `likes` (dropped — zero callers for toggle_like).
///
/// Primary key: `id` (String, the post's UUID from the IC canister).
/// Index: `by_creator` btree on `creator` for profile-pagination queries.
#[spacetimedb::table(accessor = posts, public)]
pub struct Post {
    #[primary_key]
    pub id: String,
    #[index(btree)]
    pub creator: Identity,
    pub video_uid: String,
    pub description: String,
    pub hashtags: Vec<String>,
    pub status: PostStatus,
    pub created_at: Timestamp,
    pub share_count: u64,
    pub view_total_count: u64,
    pub view_threshold_count: u64,
    pub view_average_watch_percentage: u8,
}

/// V2 post table — adds `creator_principal_text` (principal text from the
/// yral-auth JWT) alongside `creator` (Identity, one-way hash). Clients
/// (yral-mobile) need the original principal text for CDN URL construction,
/// propic URLs, username fallback, profile links, and enrichment calls.
///
/// This is a **new table** (not a column on `Post`) to avoid a manual
/// migration on the existing 730K-row `Post` table. Rust's `#[default("")]`
/// doesn't work for `String` columns (not const-constructible), so adding
/// a column to the existing table requires `--delete-data`. Instead, we
/// create a separate V2 table, backfill it from IC, validate, then swap
/// the procedure reads to use V2. The old `Post` table is dropped later.
#[spacetimedb::table(name = "posts_v2", accessor = posts_v2, public)]
pub struct PostV2 {
    #[primary_key]
    pub id: String,
    #[index(btree)]
    pub creator: Identity,
    /// Principal text from the yral-auth JWT (e.g. a Google account ID).
    pub creator_principal_text: String,
    pub video_uid: String,
    pub description: String,
    pub hashtags: Vec<String>,
    pub status: PostStatus,
    pub created_at: Timestamp,
    pub share_count: u64,
    pub view_total_count: u64,
    pub view_threshold_count: u64,
    pub view_average_watch_percentage: u8,
}

/// V3 post table — renames `creator_principal_text` → `creator_oauth_subject`
/// to use accurate current terminology (the value is the OAuth `sub` claim from
/// the yral-auth JWT, not an IC principal). Created via incremental migration
/// from `posts_v2`: dual-write, lazy-migrate on read, batch backfill, then swap
/// read paths. The old `posts_v2` table stays as-is until the migration is
/// confirmed complete.
#[spacetimedb::table(name = "posts_3", accessor = posts_3, public)]
#[derive(Clone)]
pub struct Post3 {
    #[primary_key]
    pub id: String,
    #[index(btree)]
    pub creator: Identity,
    /// OAuth subject (`sub` claim) from the yral-auth JWT.
    pub creator_oauth_subject: String,
    pub video_uid: String,
    pub description: String,
    pub hashtags: Vec<String>,
    pub status: PostStatus,
    pub created_at: Timestamp,
    pub share_count: u64,
    pub view_total_count: u64,
    pub view_threshold_count: u64,
    pub view_average_watch_percentage: u8,
}

// ─────────────────────────────────────────────────────────────────────────
// Return types (SpacetimeType — serialized as typed JSON for REST clients)
// ─────────────────────────────────────────────────────────────────────────

/// Frontend-facing post projection. Mirrors the IC canister's
/// `PostDetailsForFrontend`. `like_count` and `liked_by_me` are always `0` /
/// `false` (likes feature dropped).
///
/// `creator_oauth_subject` is the OAuth subject (`sub` claim) from the
/// yral-auth JWT — needed by clients for propic URLs, username fallback,
/// and profile enrichment calls.
///
/// `status` mirrors the IC `Post` struct's status field (the IC
/// `PostDetailsForFrontend` type omits it, but mobile's `from_post` path
/// includes it).
#[derive(SpacetimeType, Clone, Debug)]
pub struct PostDetailsForFrontend {
    pub id: String,
    pub description: String,
    pub hashtags: Vec<String>,
    pub video_uid: String,
    pub creator: Identity,
    pub creator_oauth_subject: String,
    pub created_at: Timestamp,
    pub total_view_count: u64,
    pub like_count: u64,
    pub liked_by_me: bool,
    pub status: PostStatus,
}

/// A page of posts returned by cursor-paginated queries.
/// `next_cursor` is the ID of the last post in this page — pass it as the
/// `cursor` argument to fetch the next page. `None` means no more posts.
#[derive(SpacetimeType, Clone, Debug)]
pub struct PostPage {
    pub posts: Vec<PostDetailsForFrontend>,
    pub next_cursor: Option<String>,
}

/// Offset-paginated post list. Matches the IC canister's offset-based
/// pagination contract used by mobile (startIndex, pageSize).
#[derive(SpacetimeType, Clone, Debug)]
pub struct PostListOffset {
    pub posts: Vec<PostDetailsForFrontend>,
}

/// Cursor-paginated scan result returned by `fetch_posts`.
#[derive(SpacetimeType, Clone, Debug)]
pub struct FetchPostsResult {
    pub posts: Vec<PostDetailsForFrontend>,
    pub next_cursor: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────

/// Maximum number of items a single paginated query can return.
const MAX_PAGE_SIZE: u64 = 100;

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

/// Check if a post status should be visible in profile feeds
/// (excludes Deleted, BannedDueToUserReporting, Draft).
fn is_visible_post(status: &PostStatus) -> bool {
    !matches!(
        status,
        PostStatus::Deleted | PostStatus::BannedDueToUserReporting | PostStatus::Draft
    )
}

/// Project a `Post3` row into a `PostDetailsForFrontend` for a given viewer.
/// `like_count` and `liked_by_me` are always `0` / `false` (likes dropped).
fn post_3_to_details(post: &Post3) -> PostDetailsForFrontend {
    PostDetailsForFrontend {
        id: post.id.clone(),
        description: post.description.clone(),
        hashtags: post.hashtags.clone(),
        video_uid: post.video_uid.clone(),
        creator: post.creator,
        creator_oauth_subject: post.creator_oauth_subject.clone(),
        created_at: post.created_at,
        total_view_count: post.view_total_count,
        like_count: 0,
        liked_by_me: false,
        status: post.status.clone(),
    }
}

/// Project a `Post` (V1) row into a `PostDetailsForFrontend`.
/// Used while V1 and V3 tables coexist during migration. `creator_oauth_subject`
/// is empty (V1 doesn't have it).
fn post_to_details(post: &Post) -> PostDetailsForFrontend {
    PostDetailsForFrontend {
        id: post.id.clone(),
        description: post.description.clone(),
        hashtags: post.hashtags.clone(),
        video_uid: post.video_uid.clone(),
        creator: post.creator,
        creator_oauth_subject: String::new(),
        created_at: post.created_at,
        total_view_count: post.view_total_count,
        like_count: 0,
        liked_by_me: false,
        status: post.status.clone(),
    }
}

/// Pure migration helper: map a `PostV2` row to a `Post3` row.
/// Copies all fields, renaming `creator_principal_text` → `creator_oauth_subject`.
pub fn migrate_post_v2_to_3(legacy: &PostV2) -> Post3 {
    Post3 {
        id: legacy.id.clone(),
        creator: legacy.creator,
        creator_oauth_subject: legacy.creator_principal_text.clone(),
        video_uid: legacy.video_uid.clone(),
        description: legacy.description.clone(),
        hashtags: legacy.hashtags.clone(),
        status: legacy.status.clone(),
        created_at: legacy.created_at,
        share_count: legacy.share_count,
        view_total_count: legacy.view_total_count,
        view_threshold_count: legacy.view_threshold_count,
        view_average_watch_percentage: legacy.view_average_watch_percentage,
    }
}

/// Lazy-migration helper: look up a post by ID in `posts_3` first. If not
/// found, check `posts_v2`; if found there, migrate the row into `posts_3`
/// and return it. Returns `None` if the post doesn't exist in either table.
/// Used by read procedures to transparently migrate rows on access.
///
/// Must be called inside a `with_tx` closure.
pub fn lazy_get_post_3(tx: &spacetimedb::TxContext, post_id: &str) -> Option<Post3> {
    if let Some(p) = tx.db.posts_3().id().find(post_id.to_string()) {
        return Some(p);
    }
    if let Some(legacy) = tx.db.posts_v2().id().find(post_id.to_string()) {
        let migrated = migrate_post_v2_to_3(&legacy);
        tx.db.posts_3().insert(migrated.clone());
        return Some(migrated);
    }
    None
}

/// Lazy-migration helper for reducers (takes `&ReducerContext` instead of
/// `&TxContext`). Same logic as `lazy_get_post_3`.
pub fn lazy_get_post_3_reducer(ctx: &ReducerContext, post_id: &str) -> Option<Post3> {
    if let Some(p) = ctx.db.posts_3().id().find(post_id.to_string()) {
        return Some(p);
    }
    if let Some(legacy) = ctx.db.posts_v2().id().find(post_id.to_string()) {
        let migrated = migrate_post_v2_to_3(&legacy);
        ctx.db.posts_3().insert(migrated.clone());
        return Some(migrated);
    }
    None
}

/// Recalculate the weighted running average of watch percentage.
/// Ported verbatim from the IC canister's `Post::recalculate_average_watched`.
fn recalculate_average_watched(
    avg: u8,
    total: u64,
    percentage_watched: u8,
    full_view_count: u8,
) -> u8 {
    let earlier_sum_component = avg as u64 * total;
    let current_full_view_component = 100 * full_view_count as u64;
    let current_total_dividend =
        earlier_sum_component + current_full_view_component + percentage_watched as u64;
    let current_total_divisor = total + full_view_count as u64 + 1;
    (current_total_dividend / current_total_divisor) as u8
}

/// Apply a view event to a post's view statistics.
/// Ported verbatim from the IC canister's `Post::add_view_details`.
fn apply_view_details(post: &mut Post, details: &PostViewDetailsFromFrontend) {
    let percentage_watched = details.percentage_watched;
    assert!(percentage_watched <= 100 && percentage_watched > 0);
    let watch_count = details.watch_count;

    post.view_average_watch_percentage = recalculate_average_watched(
        post.view_average_watch_percentage,
        post.view_total_count,
        percentage_watched,
        watch_count,
    );
    post.view_total_count += (watch_count + 1) as u64;
    post.view_threshold_count += watch_count as u64;
    if percentage_watched > 20 {
        post.view_threshold_count += 1;
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Post reducers — admin or the post's creator (called by backend services)
// ─────────────────────────────────────────────────────────────────────────

/// Add a new post. Admin, or the creator adding their own post. Rejects
/// duplicate post IDs.
///
/// Allowing `creator == sender` lets a service register a post while acting as
/// the user — by forwarding the user's own yral-auth `id_token`, which already
/// carries `ext_spacetimedb_token` — instead of holding the shared admin token.
/// Same gate shape as `delete_post`.
#[spacetimedb::reducer]
pub fn add_post(
    ctx: &ReducerContext,
    id: String,
    description: String,
    hashtags: Vec<String>,
    video_uid: String,
    creator: Identity,
    status: PostStatus,
) -> Result<(), String> {
    if !crate::constants::ADMINS.contains(&ctx.sender()) && creator != ctx.sender() {
        return Err("Unauthorized".to_string());
    }
    if ctx.db.posts().id().find(id.clone()).is_some() {
        return Err("DuplicatePostId".to_string());
    }
    ctx.db.posts().insert(Post {
        id,
        creator,
        video_uid,
        description,
        hashtags,
        status,
        created_at: ctx.timestamp,
        share_count: 0,
        view_total_count: 0,
        view_threshold_count: 0,
        view_average_watch_percentage: 0,
    });
    Ok(())
}

/// Update a post's status. Admin, or the post's creator.
/// Mirrors the IC canister's `update_post_status(text, PostStatus)`.
/// Special case: `Draft → Uploaded` resets `created_at` to now (publishing
/// a draft timestamps it), matching the IC canister's behavior.
///
/// The post is looked up before the authorization check so ownership can be
/// compared, exactly as `delete_post` does. A non-admin caller therefore sees
/// `PostNotFound` rather than `Unauthorized` for a post that does not exist.
#[spacetimedb::reducer]
pub fn update_post_status(
    ctx: &ReducerContext,
    post_id: String,
    status: PostStatus,
) -> Result<(), String> {
    let mut post = match ctx.db.posts().id().find(post_id) {
        Some(p) => p,
        None => return Err("PostNotFound".to_string()),
    };
    if !crate::constants::ADMINS.contains(&ctx.sender()) && post.creator != ctx.sender() {
        return Err("Unauthorized".to_string());
    }
    // Draft → Uploaded resets created_at (publishing a draft).
    if status == PostStatus::Uploaded && post.status == PostStatus::Draft {
        post.created_at = ctx.timestamp;
    }
    post.status = status;
    ctx.db.posts().id().update(post);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Post reducers — user-facing
// ─────────────────────────────────────────────────────────────────────────

/// Delete a post (set status to `Deleted`). Admin or the post creator can
/// delete. The off-chain-agent calls this as admin (it verifies ownership via
/// HTTP middleware before calling).
#[spacetimedb::reducer]
pub fn delete_post(ctx: &ReducerContext, post_id: String) -> Result<(), String> {
    let mut post = match ctx.db.posts().id().find(post_id) {
        Some(p) => p,
        None => return Err("PostNotFound".to_string()),
    };
    if !crate::constants::ADMINS.contains(&ctx.sender()) && post.creator != ctx.sender() {
        return Err("Unauthorized".to_string());
    }
    if post.status == PostStatus::Deleted {
        return Err("PostNotFound".to_string());
    }
    post.status = PostStatus::Deleted;
    ctx.db.posts().id().update(post);
    Ok(())
}

/// Record a view event on a post. Updates view statistics.
/// Mirrors the IC canister's `update_post_add_view_details(text, PostViewDetailsFromFrontend)`.
/// Called by the off-chain-agent when ingesting analytics events.
#[spacetimedb::reducer]
pub fn add_view_details(
    ctx: &ReducerContext,
    post_id: String,
    details: PostViewDetailsFromFrontend,
) -> Result<(), String> {
    let mut post = match ctx.db.posts().id().find(post_id) {
        Some(p) => p,
        None => return Err("PostNotFound".to_string()),
    };
    apply_view_details(&mut post, &details);
    ctx.db.posts().id().update(post);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Backfill reducers (PostV2 table)
// ─────────────────────────────────────────────────────────────────────────

/// Idempotent upsert of a V2 post row. Admin-only.
/// Used by the IC→SpacetimeDB backfill binary. Safe to run multiple times:
/// re-running with the same post ID updates the row instead of duplicating.
#[spacetimedb::reducer]
pub fn upsert_post(ctx: &ReducerContext, post: PostV2) -> Result<(), String> {
    if !crate::constants::ADMINS.contains(&ctx.sender()) {
        return Err("Unauthorized".to_string());
    }
    ctx.db.posts_v2().id().delete(post.id.clone());
    ctx.db.posts_v2().insert(post);
    Ok(())
}

/// Bulk upsert — accepts a Vec of V2 posts and upserts each one.
/// Admin-only. Used by the IC→SpacetimeDB backfill to reduce REST API calls.
#[spacetimedb::reducer]
pub fn upsert_posts_batch(ctx: &ReducerContext, posts: Vec<PostV2>) -> Result<(), String> {
    if !crate::constants::ADMINS.contains(&ctx.sender()) {
        return Err("Unauthorized".to_string());
    }
    for post in posts {
        ctx.db.posts_v2().id().delete(post.id.clone());
        ctx.db.posts_v2().insert(post);
    }
    Ok(())
}

/// Admin-only batch backfill: copy rows from `posts_v2` → `posts_3`, mapping
/// `creator_principal_text` → `creator_oauth_subject`. Idempotent — uses
/// primary-key lookup to skip rows already migrated (O(1) per row, not O(n)).
/// Processes at most `batch_limit` rows per call. Call repeatedly until all
/// rows are migrated. Part of the incremental migration from V2 → V3.
#[spacetimedb::reducer]
pub fn migrate_posts_to_3(ctx: &ReducerContext, batch_limit: u32) -> Result<(), String> {
    if !crate::constants::ADMINS.contains(&ctx.sender()) {
        return Err("Unauthorized".to_string());
    }
    let mut migrated_count: u32 = 0;
    for legacy in ctx.db.posts_v2().iter() {
        if migrated_count >= batch_limit {
            break;
        }
        // Skip if already migrated (O(1) primary-key lookup).
        if ctx.db.posts_3().id().find(legacy.id.clone()).is_some() {
            continue;
        }
        let migrated = migrate_post_v2_to_3(&legacy);
        ctx.db.posts_3().insert(migrated);
        migrated_count += 1;
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// V2 post reducers — creator-callable (JWT-derived principal text)
// ─────────────────────────────────────────────────────────────────────────

/// Add a new post to the `posts_3` table. Admin, or the creator adding
/// their own post. Rejects duplicate post IDs.
///
/// Unlike the V1 `add_post`, no `creator` argument is taken — `creator` is
/// derived from `ctx.sender()` and `creator_oauth_subject` is extracted
/// from the caller's JWT (`ctx.sender_auth().jwt().subject()`). This means
/// non-admin callers always create posts as themselves; an admin caller
/// can create a post, but the OAuth subject still comes from their own JWT.
#[spacetimedb::reducer]
pub fn add_post_2(
    ctx: &ReducerContext,
    id: String,
    description: String,
    hashtags: Vec<String>,
    video_uid: String,
    status: PostStatus,
) -> Result<(), String> {
    let creator = ctx.sender();
    let creator_oauth_subject = ctx
        .sender_auth()
        .jwt()
        .expect("JWT required")
        .subject()
        .to_string();
    if ctx.db.posts_3().id().find(id.clone()).is_some() {
        return Err("DuplicatePostId".to_string());
    }
    ctx.db.posts_3().insert(Post3 {
        id,
        creator,
        creator_oauth_subject,
        video_uid,
        description,
        hashtags,
        status,
        created_at: ctx.timestamp,
        share_count: 0,
        view_total_count: 0,
        view_threshold_count: 0,
        view_average_watch_percentage: 0,
    });
    Ok(())
}

/// Update a post's status in the `posts_3` table. Admin, or the post's
/// creator. Mirrors the V1 `update_post_status` but operates on `posts_3`.
/// Special case: `Draft → Uploaded` resets `created_at` to now (publishing
/// a draft timestamps it), matching the IC canister's behavior.
///
/// The post is looked up before the authorization check so ownership can be
/// compared. A non-admin caller therefore sees `PostNotFound` rather than
/// `Unauthorized` for a post that does not exist.
#[spacetimedb::reducer]
pub fn update_post_status_2(
    ctx: &ReducerContext,
    post_id: String,
    status: PostStatus,
) -> Result<(), String> {
    let mut post = match lazy_get_post_3_reducer(ctx, &post_id) {
        Some(p) => p,
        None => return Err("PostNotFound".to_string()),
    };
    if !crate::constants::ADMINS.contains(&ctx.sender()) && post.creator != ctx.sender() {
        return Err("Unauthorized".to_string());
    }
    // Draft → Uploaded resets created_at (publishing a draft).
    if status == PostStatus::Uploaded && post.status == PostStatus::Draft {
        post.created_at = ctx.timestamp;
    }
    post.status = status;
    ctx.db.posts_3().id().update(post);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────
// Procedures — typed-return reads (called by Rust SDK + REST)
// ─────────────────────────────────────────────────────────────────────────

/// Get a single post by ID. Returns `None` if the post doesn't exist or is
/// Deleted. `like_count` and `liked_by_me` always return `0` / `false`
/// (likes feature dropped — no per-viewer projection needed).
#[spacetimedb::procedure]
pub fn get_post_by_id(
    ctx: &mut ProcedureContext,
    post_id: String,
) -> Option<PostDetailsForFrontend> {
    ctx.with_tx(|tx| {
        // Try posts_3 first (has creator_oauth_subject), with lazy migration
        // from posts_v2. Fall back to V1 posts.
        if let Some(p) = lazy_get_post_3(tx, &post_id) {
            if p.status != PostStatus::Deleted {
                return Some(post_3_to_details(&p));
            }
        }
        tx.db
            .posts()
            .id()
            .find(post_id.clone())
            .filter(|p| p.status != PostStatus::Deleted)
            .map(|p| post_to_details(&p))
    })
}

/// Get a page of a user's visible posts (excludes Deleted, Banned, Draft),
/// sorted newest-first by `created_at`, then by `id` as a tiebreaker.
///
/// Args: `(creator, limit, cursor)` — cursor is the ID of the last post from
/// the previous page (pass `None` to start from the beginning). The return
/// includes `next_cursor` for the next page call (`None` = no more posts).
#[spacetimedb::procedure]
pub fn get_posts_of_user(
    ctx: &mut ProcedureContext,
    creator: Identity,
    limit: u64,
    cursor: Option<String>,
) -> PostPage {
    ctx.with_tx(|tx| {
        let limit = limit.min(MAX_PAGE_SIZE) as usize;
        let mut posts: Vec<Post> = tx
            .db
            .posts()
            .iter()
            .filter(|p| p.creator == creator && is_visible_post(&p.status))
            .collect();
        // Sort newest-first by created_at, then by id as tiebreaker.
        posts.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(b.id.cmp(&a.id)));

        // Find cursor position: skip past the cursor post.
        let start = match &cursor {
            Some(cursor_id) => posts
                .iter()
                .position(|p| &p.id == cursor_id)
                .map(|pos| pos + 1)
                .unwrap_or(0),
            None => 0,
        };

        let page: Vec<PostDetailsForFrontend> = posts
            .iter()
            .skip(start)
            .take(limit)
            .map(post_to_details)
            .collect();

        let next_cursor = if start + limit < posts.len() {
            page.last().map(|p| p.id.clone())
        } else {
            None
        };

        PostPage {
            posts: page,
            next_cursor,
        }
    })
}

/// Get a page of the current caller's draft posts, sorted newest-first.
/// Uses `ctx.sender()` as the creator.
///
/// Args: `(limit, cursor)` — cursor is the ID of the last post from the
/// previous page (pass `None` to start from the beginning).
#[spacetimedb::procedure]
pub fn get_draft_posts_of_user(
    ctx: &mut ProcedureContext,
    limit: u64,
    cursor: Option<String>,
) -> PostPage {
    let sender = ctx.sender();
    ctx.with_tx(|tx| {
        let limit = limit.min(MAX_PAGE_SIZE) as usize;
        let mut posts: Vec<Post> = tx
            .db
            .posts()
            .iter()
            .filter(|p| p.creator == sender && p.status == PostStatus::Draft)
            .collect();
        // Sort newest-first by created_at, then by id as tiebreaker.
        posts.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(b.id.cmp(&a.id)));

        let start = match &cursor {
            Some(cursor_id) => posts
                .iter()
                .position(|p| &p.id == cursor_id)
                .map(|pos| pos + 1)
                .unwrap_or(0),
            None => 0,
        };

        let page: Vec<PostDetailsForFrontend> = posts
            .iter()
            .skip(start)
            .take(limit)
            .map(post_to_details)
            .collect();

        let next_cursor = if start + limit < posts.len() {
            page.last().map(|p| p.id.clone())
        } else {
            None
        };

        PostPage {
            posts: page,
            next_cursor,
        }
    })
}

/// Cursor-paginated scan over all posts by ID (primary-key order).
/// Used by the off-chain-agent's backfill binary and the IC→SpacetimeDB
/// migration backfill.
///
/// Args: `(limit, cursor)` — cursor is the last post ID from the previous
/// page (pass `None` to start from the beginning).
#[spacetimedb::procedure]
pub fn fetch_posts(
    ctx: &mut ProcedureContext,
    limit: u64,
    cursor: Option<String>,
) -> FetchPostsResult {
    ctx.with_tx(|tx| {
        let limit = limit.min(MAX_PAGE_SIZE) as usize;
        let mut page: Vec<PostDetailsForFrontend> = Vec::with_capacity(limit);
        let mut next_cursor: Option<String> = None;

        for post in tx.db.posts().iter() {
            // Skip posts with id <= cursor (already processed).
            if let Some(ref cursor_id) = cursor {
                if post.id.as_str() <= cursor_id.as_str() {
                    continue;
                }
            }
            // Only include non-deleted posts in the scan result.
            if post.status == PostStatus::Deleted {
                continue;
            }
            if page.len() >= limit {
                break;
            }
            next_cursor = Some(post.id.clone());
            page.push(post_to_details(&post));
        }

        FetchPostsResult {
            posts: page,
            next_cursor,
        }
    })
}

/// Get a single post by ID, returning full details including `status`.
/// Matches the IC canister's `get_individual_post_details_by_id` which
/// returns the full `Post` (with status). Mobile's `from_post` path
/// expects `status` to be present.
///
/// Returns `None` if the post doesn't exist or is `Deleted`.
#[spacetimedb::procedure]
pub fn get_individual_post_details_by_id(
    ctx: &mut ProcedureContext,
    post_id: String,
) -> Option<PostDetailsForFrontend> {
    ctx.with_tx(|tx| {
        // Try posts_3 first (has creator_oauth_subject), with lazy migration
        // from posts_v2. Fall back to V1 posts.
        if let Some(p) = lazy_get_post_3(tx, &post_id) {
            if p.status != PostStatus::Deleted {
                return Some(post_3_to_details(&p));
            }
        }
        tx.db
            .posts()
            .id()
            .find(post_id.clone())
            .filter(|p| p.status != PostStatus::Deleted)
            .map(|p| post_to_details(&p))
    })
}

/// Get a page of a user's visible posts by OAuth subject, using offset
/// pagination. Matches the IC canister's
/// `get_posts_of_this_user_profile_with_pagination(principal, offset, limit)`.
///
/// Mobile passes `(principalId: String, startIndex: ULong, pageSize: ULong)`
/// — offset-based, not cursor-based. This procedure accepts the same
/// contract: `creator_oauth_subject` to match against `posts_3`, `offset`
/// to skip, `limit` to take.
///
/// Excludes Deleted, BannedDueToUserReporting, and Draft posts.
#[spacetimedb::procedure]
pub fn get_posts_of_user_by_principal(
    ctx: &mut ProcedureContext,
    creator_oauth_subject: String,
    offset: u64,
    limit: u64,
) -> PostListOffset {
    ctx.with_tx(|tx| {
        let limit = limit.min(MAX_PAGE_SIZE) as usize;
        let offset = offset as usize;

        // Read from posts_3 (has creator_oauth_subject). Also lazily migrate
        // any matching rows from posts_v2 that haven't been moved yet.
        // First, migrate any unmigrated rows for this user from posts_v2.
        for legacy in tx.db.posts_v2().iter().filter(|p| {
            p.creator_principal_text == creator_oauth_subject
                && tx.db.posts_3().id().find(p.id.clone()).is_none()
        }) {
            let migrated = migrate_post_v2_to_3(&legacy);
            tx.db.posts_3().insert(migrated);
        }

        let mut posts: Vec<PostDetailsForFrontend> = tx
            .db
            .posts_3()
            .iter()
            .filter(|p| {
                p.creator_oauth_subject == creator_oauth_subject
                    && is_visible_post(&p.status)
            })
            .map(|p| post_3_to_details(&p))
            .collect();

        if posts.is_empty() {
            // Fallback: V1 table (no creator_oauth_subject — can only match
            // if the caller passes an empty string, which won't happen in
            // practice since V3 is fully backfilled).
            posts = tx
                .db
                .posts()
                .iter()
                .filter(|p| is_visible_post(&p.status))
                .map(|p| post_to_details(&p))
                .collect();
        }

        // Sort newest-first by created_at, then by id as tiebreaker.
        posts.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(b.id.cmp(&a.id)));

        let page: Vec<PostDetailsForFrontend> = posts
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect();

        PostListOffset { posts: page }
    })
}

/// Get a page of the current caller's draft posts by OAuth subject, using
/// offset pagination. Matches the IC canister's
/// `get_draft_posts_of_this_user_profile_with_pagination(offset, limit)`.
///
/// Mobile passes `(startIndex: ULong, pageSize: ULong)` with no principal
/// (uses session). This procedure uses `ctx.sender()` as the creator and
/// accepts `creator_oauth_subject` for the `posts_3` lookup.
#[spacetimedb::procedure]
pub fn get_draft_posts_of_user_by_principal(
    ctx: &mut ProcedureContext,
    creator_oauth_subject: String,
    offset: u64,
    limit: u64,
) -> PostListOffset {
    ctx.with_tx(|tx| {
        let limit = limit.min(MAX_PAGE_SIZE) as usize;
        let offset = offset as usize;

        // Lazily migrate any unmigrated draft rows for this user from posts_v2.
        for legacy in tx.db.posts_v2().iter().filter(|p| {
            p.creator_principal_text == creator_oauth_subject
                && p.status == PostStatus::Draft
                && tx.db.posts_3().id().find(p.id.clone()).is_none()
        }) {
            let migrated = migrate_post_v2_to_3(&legacy);
            tx.db.posts_3().insert(migrated);
        }

        let mut posts: Vec<PostDetailsForFrontend> = tx
            .db
            .posts_3()
            .iter()
            .filter(|p| {
                p.creator_oauth_subject == creator_oauth_subject
                    && p.status == PostStatus::Draft
            })
            .map(|p| post_3_to_details(&p))
            .collect();

        // Sort newest-first by created_at, then by id as tiebreaker.
        posts.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(b.id.cmp(&a.id)));

        let page: Vec<PostDetailsForFrontend> = posts
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect();

        PostListOffset { posts: page }
    })
}

// ─────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_post(id: &str) -> Post {
        Post {
            id: id.to_string(),
            creator: Identity::ZERO,
            video_uid: "vid".to_string(),
            description: String::new(),
            hashtags: vec![],
            status: PostStatus::Uploaded,
            created_at: Timestamp::UNIX_EPOCH,
            share_count: 0,
            view_total_count: 0,
            view_threshold_count: 0,
            view_average_watch_percentage: 0,
        }
    }

    #[test]
    fn test_recalculate_average_watched_first_view() {
        // First view: 50% watched, no prior views.
        // avg = (0*0 + 100*0 + 50) / (0 + 0 + 1) = 50
        let avg = recalculate_average_watched(0, 0, 50, 0);
        assert_eq!(avg, 50);
    }

    #[test]
    fn test_recalculate_average_watched_second_view() {
        // After first view: avg=50, total=1. Second view: 100% watched.
        // avg = (50*1 + 100*0 + 100) / (1 + 0 + 1) = 150/2 = 75
        let avg = recalculate_average_watched(50, 1, 100, 0);
        assert_eq!(avg, 75);
    }

    #[test]
    fn test_recalculate_average_watched_with_full_views() {
        // avg=75, total=2. WatchedMultipleTimes: watch_count=2, percentage=80.
        // avg = (75*2 + 100*2 + 80) / (2 + 2 + 1) = (150+200+80)/5 = 430/5 = 86
        let avg = recalculate_average_watched(75, 2, 80, 2);
        assert_eq!(avg, 86);
    }

    #[test]
    fn test_apply_view_details_watched_partially() {
        let mut post = make_test_post("test-1");

        apply_view_details(
            &mut post,
            &PostViewDetailsFromFrontend {
                percentage_watched: 50,
                watch_count: 0,
            },
        );

        assert_eq!(post.view_total_count, 1);
        assert_eq!(post.view_average_watch_percentage, 50);
        // 50 > 20, so threshold_view_count increments.
        assert_eq!(post.view_threshold_count, 1);
    }

    #[test]
    fn test_apply_view_details_watched_partially_low_percentage() {
        let mut post = make_test_post("test-2");

        apply_view_details(
            &mut post,
            &PostViewDetailsFromFrontend {
                percentage_watched: 10,
                watch_count: 0,
            },
        );

        assert_eq!(post.view_total_count, 1);
        // 10 <= 20, so threshold_view_count does NOT increment.
        assert_eq!(post.view_threshold_count, 0);
    }

    #[test]
    fn test_apply_view_details_watched_multiple_times() {
        let mut post = make_test_post("test-3");

        apply_view_details(
            &mut post,
            &PostViewDetailsFromFrontend {
                percentage_watched: 100,
                watch_count: 2,
            },
        );

        // total = watch_count + 1 = 3
        assert_eq!(post.view_total_count, 3);
        // threshold = watch_count (2) + 1 (100 > 20) = 3
        assert_eq!(post.view_threshold_count, 3);
        // avg = (0*0 + 100*2 + 100) / (0 + 2 + 1) = 300/3 = 100
        assert_eq!(post.view_average_watch_percentage, 100);
    }

    #[test]
    fn test_is_visible_post() {
        assert!(is_visible_post(&PostStatus::Uploaded));
        assert!(is_visible_post(&PostStatus::ReadyToView));
        assert!(is_visible_post(&PostStatus::Transcoding));
        assert!(is_visible_post(&PostStatus::CheckingExplicitness));
        assert!(is_visible_post(&PostStatus::BannedForExplicitness));
        assert!(!is_visible_post(&PostStatus::Deleted));
        assert!(!is_visible_post(&PostStatus::BannedDueToUserReporting));
        assert!(!is_visible_post(&PostStatus::Draft));
    }

    #[test]
    fn test_post_to_details_likes_dropped() {
        let post = Post {
            id: "test-likes".to_string(),
            creator: Identity::ZERO,
            video_uid: "vid".to_string(),
            description: "desc".to_string(),
            hashtags: vec!["tag".to_string()],
            status: PostStatus::Uploaded,
            created_at: Timestamp::UNIX_EPOCH,
            share_count: 5,
            view_total_count: 10,
            view_threshold_count: 3,
            view_average_watch_percentage: 75,
        };

        let details = post_to_details(&post);
        assert_eq!(details.like_count, 0);
        assert!(!details.liked_by_me);
        assert_eq!(details.total_view_count, 10);
        assert_eq!(details.id, "test-likes");
        // V1 posts have no OAuth subject — should be empty.
        assert_eq!(details.creator_oauth_subject, "");
    }

    #[test]
    fn test_migrate_post_v2_to_3() {
        let legacy = PostV2 {
            id: "test-migrate".to_string(),
            creator: Identity::ZERO,
            creator_principal_text: "google-oauth-sub-123".to_string(),
            video_uid: "vid".to_string(),
            description: "desc".to_string(),
            hashtags: vec!["tag".to_string()],
            status: PostStatus::Uploaded,
            created_at: Timestamp::UNIX_EPOCH,
            share_count: 5,
            view_total_count: 10,
            view_threshold_count: 3,
            view_average_watch_percentage: 75,
        };

        let migrated = migrate_post_v2_to_3(&legacy);
        assert_eq!(migrated.id, "test-migrate");
        assert_eq!(migrated.creator, Identity::ZERO);
        // creator_principal_text → creator_oauth_subject
        assert_eq!(migrated.creator_oauth_subject, "google-oauth-sub-123");
        assert_eq!(migrated.video_uid, "vid");
        assert_eq!(migrated.description, "desc");
        assert_eq!(migrated.hashtags, vec!["tag".to_string()]);
        assert_eq!(migrated.status, PostStatus::Uploaded);
        assert_eq!(migrated.share_count, 5);
        assert_eq!(migrated.view_total_count, 10);
        assert_eq!(migrated.view_threshold_count, 3);
        assert_eq!(migrated.view_average_watch_percentage, 75);
    }

    #[test]
    fn test_post_3_to_details() {
        let post = Post3 {
            id: "test-3".to_string(),
            creator: Identity::ZERO,
            creator_oauth_subject: "oauth-sub-456".to_string(),
            video_uid: "vid".to_string(),
            description: "desc".to_string(),
            hashtags: vec!["tag".to_string()],
            status: PostStatus::Uploaded,
            created_at: Timestamp::UNIX_EPOCH,
            share_count: 5,
            view_total_count: 10,
            view_threshold_count: 3,
            view_average_watch_percentage: 75,
        };

        let details = post_3_to_details(&post);
        assert_eq!(details.id, "test-3");
        assert_eq!(details.creator_oauth_subject, "oauth-sub-456");
        assert_eq!(details.like_count, 0);
        assert!(!details.liked_by_me);
        assert_eq!(details.total_view_count, 10);
    }
}
