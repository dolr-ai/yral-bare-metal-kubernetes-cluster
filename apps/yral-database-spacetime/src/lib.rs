//! Yral database module for SpacetimeDB.
//!
//! This crate is a SpacetimeDB wasm module (`cdylib`) published to Maincloud.
//! Features are organized by product function — each feature gets its own
//! top-level module. Currently empty (the rate limit feature was removed —
//! login checks are handled by the Prakash video-storage-service backend).
//! Future features (yral-auth, yral-metadata migration) will go here.
//!
//! ## SpacetimeDB conventions (spacetimedb 2.6.1)
//! - **No raw SQL from application code** (hard rule, see AGENTS.md). Rust
//!   services use the generated `spacetimedb-sdk` bindings; mobile/non-SDK
//!   clients call procedures via REST (`POST /v1/database/{db}/call/:name`).
//! - **Reducers** for mutations (transactional, can't return data); **procedures**
//!   for per-user/typed-return reads (non-transactional, `ctx.with_tx`, return
//!   typed `SpacetimeType`); **HTTP handlers** for truly public/identity-agnostic
//!   endpoints (bypass auth, arbitrary `http::Response`).
//! - Procedures + HTTP handlers require `features = ["unstable"]` in this
//!   crate's `Cargo.toml` (unstable-gated in spacetimedb 2.6.1).
//! - Table index macro: `index(accessor = by_x_y, btree(columns = [x, y]))`
//!   — `accessor` takes a bare ident (NOT `name =`); `#[unique]` auto-creates
//!   a unique btree index (no separate `index(...)` needed).
//! - The table `accessor = foo` generates a trait `foo` on `spacetimedb::Local`
//!   — `spacetimedb::Table` must be imported for `.insert()/.iter()/.id().update()`;
//!   the per-table accessor traits are auto-in-scope in the same module.

mod auth_kv;
mod constants;
mod posts;
