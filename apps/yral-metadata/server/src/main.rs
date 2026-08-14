// rebuild trigger
mod api;
mod auth;
mod config;
mod consts;
mod dragonfly;
mod services;
mod session;
mod signup;
mod state;
mod utils;

#[cfg(test)]
mod test_utils;
use std::sync::Arc;

use axum::{
    routing::{delete, get, post},
    Router,
};
use config::AppConfig;
use state::AppState;
use tower_http::cors::CorsLayer;
use utils::error::*;

fn setup_tracing_subscriber() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,hyper=warn,reqwest=warn,tower_http=warn".into()),
        )
        .init();
}

async fn main_impl() -> Result<()> {
    let conf = AppConfig::load()?;

    let state = Arc::new(AppState::new(&conf).await?);

    // Build the application router with all routes defined here
    let app = Router::new()
        // API routes
        .route(
            "/metadata/{user_principal}",
            post(api::handlers::set_user_metadata),
        )
        .route(
            "/admin/metadata/{user_principal}",
            post(api::handlers::admin_set_user_metadata),
        )
        .route(
            "/metadata/{user_principal}",
            get(api::handlers::get_user_metadata),
        )
        .route(
            "/metadata/bulk",
            delete(api::handlers::delete_metadata_bulk),
        )
        .route(
            "/metadata-bulk",
            post(api::handlers::get_user_metadata_bulk),
        )
        .route(
            "/canister-to-principal/bulk",
            post(api::handlers::get_canister_to_principal_bulk),
        )
        // Session routes
        .route(
            "/v2/update_session_as_registered",
            post(session::update_session_as_registered_v2),
        )
        // Signup routes
        .route("/email/{user_principal}", post(signup::set_user_email))
        .route(
            "/signup/{user_principal}",
            post(signup::set_signup_datetime),
        )
        // OpenAPI/Swagger UI routes
        .route("/explorer/{*tail}", get(services::openapi::get_swagger))
        .route("/explorer/", get(services::openapi::get_swagger_root))
        .route("/healthz", get(api::handlers::healthz))
        .layer(CorsLayer::permissive())
        // Add shared state
        .with_state(state.clone());

    let listener = tokio::net::TcpListener::bind(conf.bind_address)
        .await
        .map_err(|e| Error::IO(e))?;

    log::info!("Server starting on {}", conf.bind_address);

    axum::serve(listener, app).await.map_err(|e| Error::IO(e))?;

    Ok(())
}

fn main() {
    setup_tracing_subscriber();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            main_impl().await.unwrap();
        });
}
