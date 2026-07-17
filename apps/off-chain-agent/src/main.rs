#![recursion_limit = "256"]

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::extract::DefaultBodyLimit;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{routing::get, Router};
use canister::canister_health_handler;
use config::AppConfig;
use events::event::storj::enqueue_storj_backfill_item;
use http::header::CONTENT_TYPE;
use offchain_service::report_approved_handler;
use tonic::service::Routes;
use tower::make::Shared;
use tower::steer::Steer;
use tower_http::cors::CorsLayer;
use tracing::instrument;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

use crate::auth::check_auth_grpc;
use crate::events::warehouse_events::warehouse_events_server::WarehouseEventsServer;
use crate::events::{warehouse_events, WarehouseEventsService};
use crate::offchain_service::off_chain::off_chain_server::OffChainServer;
use crate::offchain_service::{off_chain, OffChainService};
use error::*;

mod ai_video_detector;
mod app_state;
mod auth;
pub mod canister;
mod config;
mod consts;
mod error;
mod events;
pub mod kvrocks;
pub mod leaderboard;
mod middleware;
mod offchain_service;
pub mod pipeline;
mod posts;
mod rewards;
pub mod scratchpad;
mod types;
pub mod user;
pub mod utils;
#[cfg(not(feature = "local-bin"))]
mod video_processing;
pub mod videogen;
pub mod yral_auth;

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
    #[cfg(not(feature = "local-bin"))]
    video_processing::worker::spawn_worker(shared_state.clone())?;

    let router = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/api/v1/posts", posts::posts_router(shared_state.clone()))
        .nest(
            "/api/v1/events",
            events::events_router(shared_state.clone()),
        )
        .nest("/api/v1/user", user::user_router(shared_state.clone()))
        .nest(
            "/api/v1/videogen",
            videogen::videogen_router(shared_state.clone()),
        )
        .nest(
            "/api/v1/leaderboard",
            leaderboard::leaderboard_router(shared_state.clone()),
        )
        .nest(
            "/api/v1/rewards",
            rewards::api::rewards_router(shared_state.clone()),
        )
        .nest(
            "/api/v2/videogen",
            videogen::videogen_router_v2(shared_state.clone()),
        )
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
    let replicate_webhook_routes = videogen::router::replicate_webhook_router(shared_state.clone());
    let comfyui_webhook_routes = videogen::router::comfyui_webhook_router(shared_state.clone());

    let http = Router::new()
        .route("/healthz", get(health_handler))
        .route("/canister-health", get(canister_health_handler))
        .route("/report-approved", post(report_approved_handler))
        .route(
            "/enqueue_storj_backfill_item",
            post(enqueue_storj_backfill_item),
        )
        .nest("/replicate", replicate_webhook_routes)
        .nest("/comfyui", comfyui_webhook_routes)
        .fallback_service(router)
        .layer(DefaultBodyLimit::max(50 * 1024 * 1024)) // 50MB limit
        .layer(CorsLayer::permissive())
        .layer(axum::middleware::from_fn(
            crate::middleware::http_logging_middleware,
        ))
        .with_state(shared_state.clone());

    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(warehouse_events::FILE_DESCRIPTOR_SET)
        .register_encoded_file_descriptor_set(off_chain::FILE_DESCRIPTOR_SET)
        .build_v1()
        .unwrap();

    let grpc_axum = Routes::builder()
        .routes()
        .add_service(WarehouseEventsServer::with_interceptor(
            WarehouseEventsService {
                shared_state: shared_state.clone(),
            },
            check_auth_grpc,
        ))
        .add_service(OffChainServer::with_interceptor(
            OffChainService {
                shared_state: shared_state.clone(),
            },
            check_auth_grpc,
        ))
        .add_service(reflection_service)
        .into_axum_router();

    let http_grpc = Steer::new(
        vec![http, grpc_axum],
        |req: &axum::extract::Request, _svcs: &[_]| {
            if req.headers().get(CONTENT_TYPE).map(|v| v.as_bytes()) != Some(b"application/grpc") {
                0
            } else {
                1
            }
        },
    );

    // run it
    let addr = SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 0], 50051));
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    log::info!("listening on {addr}");

    axum::serve(listener, Shared::new(http_grpc)).await.unwrap();

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
