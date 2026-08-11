pub mod app_state;
pub mod app_type;
pub mod audio_state;
pub mod canisters;
#[cfg(feature = "ssr")]
pub mod spacetime;

#[cfg(feature = "ssr")]
pub mod server {

    use auth::server_impl::store::KVStoreImpl;
    use axum::extract::FromRef;
    use axum_extra::extract::cookie::Key;
    use leptos::prelude::*;
    use leptos_axum::AxumRouteListing;

    #[derive(FromRef, Clone)]
    pub struct AppState {
        pub leptos_options: LeptosOptions,
        #[cfg(feature = "cloudflare")]
        pub cloudflare: gob_cloudflare::CloudflareAuth,
        pub kv: KVStoreImpl,
        pub routes: Vec<AxumRouteListing>,
        pub cookie_key: Key,
        #[cfg(feature = "oauth-ssr")]
        pub yral_oauth_client: auth::server_impl::yral::YralOAuthClient,
        #[cfg(feature = "oauth-ssr")]
        pub yral_auth_migration_key: jsonwebtoken::EncodingKey,
        #[cfg(feature = "ssr")]
        pub spacetime_conn: std::sync::Arc<yral_database_spacetime_bindings::DbConnection>,
    }
}
