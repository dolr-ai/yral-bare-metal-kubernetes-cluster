use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};
use std::time::Instant;

/// HTTP logging middleware — logs HTTP requests using tracing (for LGTM stack).
///
/// Security: Does NOT capture request/response bodies (auth service)
pub async fn http_logging_middleware(request: Request, next: Next) -> Response {
    let method = request.method().to_string();
    let path = request.uri().path().to_string();
    let start = Instant::now();

    let response = next.run(request).await;

    let duration_ms = start.elapsed().as_millis();
    let status = response.status();

    if status.as_u16() >= 400 {
        log::error!(
            "HTTP Error: {} {} -> {} ({}ms)",
            method,
            path,
            status.as_u16(),
            duration_ms
        );
    } else {
        log::info!(
            "{} {} -> {} ({}ms)",
            method,
            path,
            status.as_u16(),
            duration_ms
        );
    }

    response
}
