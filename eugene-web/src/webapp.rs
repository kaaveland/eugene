use axum::routing::{get, post};
use axum::Router;

pub mod error;
pub mod index;
pub mod lint_html;
pub mod lint_json;
pub mod lint_raw;
pub mod requestlog;
pub mod templates;

pub fn routes() -> Router {
    Router::new()
        .route("/", get(index::render_index))
        .route("/lint.html", post(lint_html::lint_html))
        .route("/lint.json", post(lint_json::json_lint_handler))
        .route("/lint.raw", post(lint_raw::raw_lint_handler))
}
