// TODO(migrate-to-delete-user-procedure): SpacetimeDB's standalone
// delete_user_info reducer was REMOVED (2026-09-10) — reducers cannot
// propagate deletion to external systems, which orphaned the bots'
// ai_influencers rows on the agent service (names stayed "taken":
// dolr-ai/yral-rishi-agent#512/#513). yral-auth's user-deletion flow
// (wherever it deletes a SpacetimeDB user — e.g. the admin/account
// paths in src/api/) must call the new delete_user PROCEDURE instead:
// POST /v1/database/{db}/call/delete_user with [subject, id_token] —
// the procedure runs the same cascade and then soft-deletes the bots'
// agent rows server-side. DeleteUserResult: [error, deleted_subjects,
// backend_deletions] (error "" on success; failed backend_deletions are
// retryable — the call is idempotent). See cascade_delete_user and
// delete_user in apps/yral-database-spacetime/src/user_info.rs.

// TODO(auth-page-all-providers): the web app's login surfaces support
// only a subset of the providers the mobile apps offer. Add:
//   - Sign in with Apple (oauth flow in src/oauth_provider.rs + the
//     Apple button on the auth page in src/page/auth.rs)
//   - WhatsApp OTP login (wire src/context/message_delivery_service/
//     to the auth page's phone flow once the WhatsApp API key works —
//     see the whatsapp-api-broken TODO there)

// TODO(ssr-to-client-spa): this app is a cargo-leptos SSR-rendered app
// (features ssr + hydrate; hydrate_body in lib.rs, the axum server in
// src/main.rs). Migrate to a fully client-rendered WASM SPA: drop the
// ssr feature, ship one wasm binary mounted via mount::mount_to_body,
// serve static assets from a plain file server (or the k8s nginx
// config in the infra repo), keep the API routes as a separate
// service. Rationale: the auth app is fully interactive post-load —
// SSR buys nothing but a Rust compile+server per deploy; a WASM SPA
// collapses the deploy to static hosting and matches the yral-web /
// my-website islands model already in this workspace.
#[cfg(feature = "ssr")]
pub mod api;
pub mod app;
pub mod components;
pub mod consts;
pub mod context;
pub mod error;
#[cfg(feature = "ssr")]
pub mod kv;
#[cfg(feature = "ssr")]
pub mod middleware;
pub mod oauth;
#[cfg(feature = "ssr")]
pub mod oauth_provider;
mod page;
#[cfg(feature = "ssr")]
pub mod spacetime;
pub mod utils;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::*;

    // initializes logging using the `log` crate
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    leptos::mount::hydrate_body(App);
}
