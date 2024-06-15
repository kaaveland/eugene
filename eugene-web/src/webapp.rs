use axum::routing::{get, post};
use axum::Router;
use eugene::hint_data::ALL;
use eugene::lints::rules;

pub mod error;
pub mod index;
pub mod lint_html;
pub mod lint_json;
pub mod lint_raw;
pub mod requestlog;
pub mod templates;

async fn random_sql() -> Result<impl axum::response::IntoResponse, error::WebAppError> {
    loop {
        let n: usize = rand::random();
        let choice = n % ALL.len();
        let id = ALL[choice].id;
        if rules::all_rules().any(|r| r.id() == id) {
            return Ok(ALL[choice].bad_example);
        }
    }
}

pub fn routes() -> Router {
    Router::new()
        .route("/", get(index::render_index))
        .route("/lint.html", post(lint_html::lint_html))
        .route("/lint.json", post(lint_json::json_lint_handler))
        .route("/lint.raw", post(lint_raw::raw_lint_handler))
        .route("/random.sql", get(random_sql))
}
