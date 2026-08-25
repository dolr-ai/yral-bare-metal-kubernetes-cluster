#![recursion_limit = "256"]
use axum::{Router, routing::get};
use axum::{
    body::Body as AxumBody,
    extract::State,
    http::Request,
    response::{IntoResponse, Response},
};
use state::server::AppState;
use tracing::instrument;
use yral_web::fallback::file_and_error_handler;

use http::{HeaderName, Method, header};
use leptos::prelude::*;
use leptos_axum::handle_server_fns_with_context;
use leptos_axum::{LeptosRoutes, generate_route_list};
use tower_http::cors::{AllowOrigin, CorsLayer};
use yral_web::app::shell;
use yral_web::{app::App, init::AppStateBuilder};

#[instrument(skip(app_state))]
pub async fn server_fn_handler(
    State(app_state): State<AppState>,
    request: Request<AxumBody>,
) -> impl IntoResponse {
    handle_server_fns_with_context(
        move || {
            provide_context(app_state.cookie_key.clone());

            #[cfg(feature = "oauth-ssr")]
            {
                provide_context(app_state.yral_oauth_client.clone());
            }


            #[cfg(feature = "ssr")]
            provide_context(app_state.spacetime_conn.clone());
        },
        request,
    )
    .await
}

#[instrument(skip(state))]
pub async fn leptos_routes_handler(state: State<AppState>, req: Request<AxumBody>) -> Response {
    let State(app_state) = state.clone();
    let handler = leptos_axum::render_route_with_context(
        app_state.routes.clone(),
        move || {
            provide_context(app_state.cookie_key.clone());
            #[cfg(feature = "oauth-ssr")]
            provide_context(app_state.yral_oauth_client.clone());

            #[cfg(feature = "ssr")]
            provide_context(app_state.spacetime_conn.clone());
        },
        move || shell(app_state.leptos_options.clone()),
    );
    handler(state, req).await.into_response()
}

async fn main_impl() -> Result<(), Box<dyn std::error::Error>> {
    // Setting get_configuration(None) means we'll be using cargo-leptos's env values
    // For deployment these variables are:
    // <https://github.com/leptos-rs/start-axum#executing-a-server-on-a-remote-machine-without-the-toolchain>
    // Alternately a file can be specified such as Some("Cargo.toml")
    // The file would need to be included with the executable when moved to deployment
    let conf = get_configuration(None).unwrap();
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    let res = AppStateBuilder::new(leptos_options, routes.clone())
        .build()
        .await;
    let terminate = {
        use tokio::signal;

        let ctrl_c = async {
            signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            use tokio::signal;
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        async {
            tokio::select! {
                _ = ctrl_c => {},
                _ = terminate => {},
            }
            log::info!("stopping...");
        }
    };

    // build our application with a route
    let app = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route(
            "/api/{*fn_name}",
            get(server_fn_handler).post(server_fn_handler),
        )
        .layer(
            CorsLayer::new()
                .allow_credentials(true)
                .allow_headers([
                    header::AUTHORIZATION,
                    header::CONTENT_TYPE,
                    header::ACCEPT,
                    HeaderName::from_static("baggage"),
                ])
                .allow_methods([Method::POST, Method::GET, Method::PUT, Method::OPTIONS])
                .allow_origin(AllowOrigin::predicate(|origin, _| {
                    if let Ok(host) = origin.to_str() {
                        host == "legacy.yral.com"
                    } else {
                        false
                    }
                })),
        )
        .leptos_routes_with_handler(routes, get(leptos_routes_handler))
        .fallback(file_and_error_handler)
        .with_state(res.app_state);

    // run our app with hyper
    // `axum::Server` is a re-export of `hyper::Server`
    println!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(terminate)
    .await
    .unwrap();

    Ok(())
}

fn setup_tracing_subscriber() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                // axum logs rejections from built-in extractors with the `axum::rejection`
                // target, at `TRACE` level. `axum::rejection=trace` enables showing those events
                format!(
                    "{}=debug,tower_http=debug,axum::rejection=trace",
                    env!("CARGO_CRATE_NAME")
                )
                .into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

fn main() {
    setup_tracing_subscriber();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            if let Err(e) = main_impl().await {
                eprintln!("Server error: {e}");
                std::process::exit(1);
            }
        });
}
