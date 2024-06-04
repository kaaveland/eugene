use crate::webapp;
use serde::Serialize;

#[derive(Serialize)]
struct Void {}

pub(crate) async fn render_index(
) -> Result<impl axum::response::IntoResponse, webapp::error::WebAppError> {
    let body = webapp::templates::handlebars()
        .render("index", &Void {})
        .map_err(webapp::error::WebAppError::from)?;
    Ok(axum::response::Html(body))
}
