use std::{env, net::SocketAddr};

use anyhow::{Context, Result};
use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tracing::info;

#[derive(Clone)]
struct AppState {
    pool: PgPool,
}

#[derive(Serialize)]
struct CounterResponse {
    value: i64,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let database_url = env::var("DATABASE_URL").context("DATABASE_URL is required")?;
    let bind_addr = parse_bind_addr();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .context("failed to connect to PostgreSQL")?;

    init_db(&pool).await.context("failed to initialize schema")?;

    let state = AppState { pool };
    let app = Router::new()
        .route("/", get(increment_counter))
        .route("/healthz", get(healthz))
        .with_state(state);

    info!(%bind_addr, "starting rust-counter server");
    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .context("failed to bind server socket")?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server exited with error")?;

    Ok(())
}

async fn increment_counter(State(state): State<AppState>) -> Result<Json<CounterResponse>, (axum::http::StatusCode, String)> {
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(internal_error)?;

    let next_value = sqlx::query_scalar::<_, i64>(
        "UPDATE counter_state SET value = value + 1 WHERE id = 1 RETURNING value",
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(internal_error)?;

    tx.commit().await.map_err(internal_error)?;

    Ok(Json(CounterResponse { value: next_value }))
}

async fn healthz() -> &'static str {
    "ok"
}

async fn init_db(pool: &PgPool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS counter_state (id SMALLINT PRIMARY KEY, value BIGINT NOT NULL)",
    )
    .execute(pool)
    .await?;

    sqlx::query("INSERT INTO counter_state (id, value) VALUES (1, 0) ON CONFLICT (id) DO NOTHING")
        .execute(pool)
        .await?;

    Ok(())
}

fn parse_bind_addr() -> SocketAddr {
    let port = env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(8080);

    SocketAddr::from(([0, 0, 0, 0], port))
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rust_counter=info,tower_http=info,axum=info".into()),
        )
        .without_time()
        .init();
}

fn internal_error(error: sqlx::Error) -> (axum::http::StatusCode, String) {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        format!("database error: {error}"),
    )
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    info!("received shutdown signal");
}
