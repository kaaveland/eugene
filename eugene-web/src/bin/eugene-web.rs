use axum::extract::RawForm;
use axum::http::Request;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{http, Json, Router};
use eugene::output;
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tracing::Span;

use eugene::output::LintReport;
use eugene_web::lint_scripts;

struct WebAppError {
    inner: anyhow::Error,
}

impl IntoResponse for WebAppError {
    fn into_response(self) -> Response {
        error!("{}", self.inner);
        Response::builder()
            .status(500)
            .body("Internal Server Error".into())
            .unwrap()
    }
}

impl<E> From<E> for WebAppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self { inner: err.into() }
    }
}

#[derive(Deserialize, Serialize)]
struct ScriptInput {
    script: String,
}

async fn json_lint_handler(
    Json(input): Json<ScriptInput>,
) -> Result<Json<Vec<LintReport>>, WebAppError> {
    let reports = lint_scripts(input.script)?;
    Ok(Json(reports))
}

async fn raw_lint_handler(RawForm(body): RawForm) -> Result<String, WebAppError> {
    let bytes = body.to_vec();
    let script = String::from_utf8(bytes)?;
    let reports: Result<Vec<_>, _> = lint_scripts(script)?
        .into_iter()
        .map(|report| output::templates::lint_text(&report))
        .collect();
    let reports: Vec<_> = reports?
        .into_iter()
        .filter(|report| !report.trim().is_empty())
        .collect();
    Ok(reports.join("\n"))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let logger = tower_http::trace::TraceLayer::new_for_http()
        .on_request(|req: &Request<_>, _span: &Span| {
            let path = req.uri().path();
            let method = req.method().as_str();
            let user_agent = req
                .headers()
                .get("user-agent")
                .map(|v| v.to_str().unwrap_or("invalid"));
            let len = req
                .headers()
                .get("content-length")
                .map(|v| v.to_str().unwrap_or("invalid"));
            info!(
                "{} {} {} {}",
                method,
                path,
                user_agent.unwrap_or("-"),
                len.unwrap_or("0")
            );
        })
        .on_response(
            |res: &http::Response<_>, duration: Duration, _span: &Span| {
                let status = res.status().as_u16();
                let len = res
                    .headers()
                    .get("content-length")
                    .map(|v| v.to_str().unwrap_or("invalid"));
                info!(
                    "{} {} {}ms",
                    status,
                    len.unwrap_or("0"),
                    duration.as_millis()
                );
            },
        );

    let api = Router::new()
        .route("/lint.json", post(json_lint_handler))
        .route("/lint.raw", post(raw_lint_handler));

    let app = Router::new()
        .nest("/eugene/app", api)
        .layer(ServiceBuilder::new().layer(logger).into_inner())
        .layer(RequestBodyLimitLayer::new(1024 * 50))
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
