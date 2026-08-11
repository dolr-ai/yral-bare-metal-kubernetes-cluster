use std::env;

use auth::server_impl::store::KVStoreImpl;
use axum_extra::extract::cookie::Key;
use leptos::prelude::*;
use leptos_axum::AxumRouteListing;
use state::server::AppState;

#[cfg(feature = "cloudflare")]
fn init_cf() -> gob_cloudflare::CloudflareAuth {
    use gob_cloudflare::{CloudflareAuth, Credentials};
    let creds = Credentials {
        token: env::var("CF_TOKEN").expect("`CF_TOKEN` is required!"),
        account_id: env::var("CF_ACCOUNT_ID").expect("`CF_ACCOUNT_ID` is required!"),
    };
    CloudflareAuth::new(creds)
}

fn init_cookie_key() -> Key {
    let cookie_key_str = env::var("COOKIE_KEY").expect("`COOKIE_KEY` is required!");
    let cookie_key_raw =
        hex::decode(cookie_key_str).expect("Invalid `COOKIE_KEY` (must be length 128 hex)");
    Key::from(&cookie_key_raw)
}

#[cfg(feature = "oauth-ssr")]
fn init_yral_oauth() -> auth::server_impl::yral::YralOAuthClient {
    use auth::server_impl::yral::YralOAuthClient;
    use consts::yral_auth::{
        YRAL_AUTH_AUTHORIZATION_URL, YRAL_AUTH_CLIENT_ID_ENV, YRAL_AUTH_ISSUER_URL,
        YRAL_AUTH_TOKEN_URL,
    };
    use openidconnect::{AuthType, AuthUrl, TokenUrl};
    use openidconnect::{ClientId, ClientSecret, IssuerUrl, RedirectUrl};

    let client_id = env::var(YRAL_AUTH_CLIENT_ID_ENV)
        .unwrap_or_else(|_| panic!("`{YRAL_AUTH_CLIENT_ID_ENV}` is required!"));
    let client_secret =
        env::var("YRAL_AUTH_CLIENT_SECRET").expect("`YRAL_AUTH_CLIENT_SECRET` is required!");
    let redirect_uri =
        env::var("YRAL_AUTH_REDIRECT_URL").expect("`YRAL_AUTH_REDIRECT_URL` is required!");

    YralOAuthClient::new(
        ClientId::new(client_id),
        Some(ClientSecret::new(client_secret)),
        IssuerUrl::new(YRAL_AUTH_ISSUER_URL.to_string()).unwrap(),
        AuthUrl::new(YRAL_AUTH_AUTHORIZATION_URL.to_string()).unwrap(),
        Some(TokenUrl::new(YRAL_AUTH_TOKEN_URL.to_string()).unwrap()),
        None,
        Default::default(),
    )
    .set_redirect_uri(RedirectUrl::new(redirect_uri).unwrap())
    .set_auth_type(AuthType::RequestBody)
}

#[cfg(feature = "oauth-ssr")]
fn init_yral_auth_migration_key() -> jsonwebtoken::EncodingKey {
    let raw_pem = env::var("YRAL_AUTH_MIGRATION_ES256_PEM")
        .expect("`YRAL_AUTH_MIGRATION_ES256_PEM` is required!");
    let enc_key = jsonwebtoken::EncodingKey::from_ec_pem(raw_pem.as_bytes())
        .expect("Invalid `YRAL_AUTH_MIGRATION_ES256_PEM`");

    enc_key
}

pub struct AppStateRes {
    pub app_state: AppState,
}

pub struct AppStateBuilder {
    leptos_options: LeptosOptions,
    routes: Vec<AxumRouteListing>,
}

impl AppStateBuilder {
    pub fn new(leptos_options: LeptosOptions, routes: Vec<AxumRouteListing>) -> Self {
        Self {
            leptos_options,
            routes,
        }
    }

    async fn init_redis_kv(&mut self) -> KVStoreImpl {
        use auth::server_impl::store::dragonfly_kv::DragonflyKV;

        log::info!("initializing dragonfly redis instance");
        KVStoreImpl::DragonflyKV(
            DragonflyKV::new()
                .await
                .expect("failed to initialize dragonfly redis"),
        )
    }

    pub async fn build(mut self) -> AppStateRes {
        let kv = self.init_redis_kv().await;

        let app_state = AppState {
            leptos_options: self.leptos_options,
            routes: self.routes,
            #[cfg(feature = "cloudflare")]
            cloudflare: init_cf(),
            kv,
            cookie_key: init_cookie_key(),
            #[cfg(feature = "oauth-ssr")]
            yral_oauth_client: init_yral_oauth(),
            #[cfg(feature = "oauth-ssr")]
            yral_auth_migration_key: init_yral_auth_migration_key(),
            #[cfg(feature = "ssr")]
            spacetime_conn: state::spacetime::init_spacetime()
                .expect("Failed to connect to SpacetimeDB"),
        };

        AppStateRes { app_state }
    }
}
