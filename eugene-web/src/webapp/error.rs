use axum::response::{IntoResponse, Response};
use log::error;

pub struct WebAppError {
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
