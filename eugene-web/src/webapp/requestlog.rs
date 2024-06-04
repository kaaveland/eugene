use std::time::Duration;

use axum::http::{Request, Response};
use log::info;
use tracing::Span;

pub fn log_request<T>(req: &Request<T>, _: &Span) {
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
}

pub fn log_response<T>(res: &Response<T>, duration: Duration, _: &Span) {
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
}
