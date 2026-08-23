#![recursion_limit = "256"]

use anyhow::Result;
use axum::extract::DefaultBodyLimit;
use axum::http::StatusCode;
use axum::{routing::get, Router};
use config::AppConfig;
use std::net::SocketAddr;
use std::sync::Arc;
use tower::make::Shared;
use tower_http::cors::CorsLayer;
use tracing::instrument;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

mod app_state;
mod auth;
mod config;
mod consts;
mod events;
mod middleware;
mod posts;
mod spacetime;
mod types;
pub mod user;
pub mod utils;

use app_state::AppState;

async fn main_impl() -> Result<()> {
    #[derive(OpenApi)]
    #[openapi(
        tags(
            (name = "OFF_CHAIN", description = "Off Chain Agent API"),
        )
    )]
    struct ApiDoc;

    let conf = AppConfig::load()?;

    let shared_state = Arc::new(AppState::new(conf.clone()).await);

    let router = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/api/v1/posts", posts::posts_router(shared_state.clone()))
        .nest(
            "/api/v1/events",
            events::events_router(shared_state.clone()),
        )
        .nest("/api/v1/user", user::user_router(shared_state.clone()))
        .nest(
            "/api/v2/events",
            events::events_router_v2(shared_state.clone()),
        )
        .nest(
            "/api/v2/posts",
            posts::posts_router_v2(shared_state.clone()),
        );

    let (router, api) = router.split_for_parts();

    let router =
        router.merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api.clone()));

    // build our application with a route
    let http = Router::new()
        .route("/healthz", get(health_handler))
        .fallback_service(router)
        .layer(DefaultBodyLimit::max(50 * 1024 * 1024)) // 50MB limit
        .layer(CorsLayer::permissive())
        .layer(axum::middleware::from_fn(
            crate::middleware::http_logging_middleware,
        ))
        .with_state(shared_state.clone());

    // run it
    let addr = SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 0], 50051));
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    log::info!("listening on {addr}");

    axum::serve(listener, Shared::new(http)).await.unwrap();

    Ok(())
}

fn main() {
    // Initialize the rustls crypto provider
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    setup_tracing_subscriber();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            main_impl().await.unwrap();
        });
}

fn setup_tracing_subscriber() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!(
                    "{}=info,tower_http=warn,axum::rejection=warn,hyper=warn,reqwest=warn",
                    env!("CARGO_CRATE_NAME")
                )
                .into()
            }),
        )
        .init();
}

#[instrument]
async fn health_handler() -> (StatusCode, &'static str) {
    (StatusCode::OK, "OK")
}
