use axum::{
    body::Body as AxumBody,
    extract::State,
    http::Request,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use leptos::{config::get_configuration, logging::log, prelude::provide_context};
use leptos_axum::{generate_route_list, LeptosRoutes};
use my_website::{app::{shell, app}, content::ContentProvider};

#[derive(Clone, axum::extract::FromRef)]
pub struct ServerState {
    pub leptos_options: leptos::config::LeptosOptions,
    pub routes: Vec<leptos_axum::AxumRouteListing>,
}

async fn leptos_routes_handler(
    State(app_state): State<ServerState>,
    req: Request<AxumBody>,
) -> Response {
    let State(app_state) = State(app_state);
    let leptos_options = app_state.leptos_options.clone();
    let routes = app_state.routes.clone();
    let handler = leptos_axum::render_route_with_context(
        routes,
        move || {
            provide_context(ContentProvider::new());
        },
        move || shell(leptos_options.clone()),
    );
    handler(State(app_state), req).await.into_response()
}

fn setup_tracing_subscriber() {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();
}

#[tokio::main]
async fn main() {
    setup_tracing_subscriber();

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;
    let routes = generate_route_list(app);

    let app_state = ServerState {
        leptos_options,
        routes: routes.clone(),
    };

    let app = Router::new()
        .leptos_routes_with_handler(routes, get(leptos_routes_handler))
        .fallback(leptos_axum::file_and_error_handler::<ServerState, _>(shell))
        .with_state(app_state);

    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}