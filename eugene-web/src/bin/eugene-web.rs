use axum::Router;
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;

use eugene_web::webapp;
use eugene_web::webapp::requestlog::{log_request, log_response};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let logger = tower_http::trace::TraceLayer::new_for_http()
        .on_request(log_request)
        .on_response(log_response);

    let app = Router::new()
        .nest("/eugene/app", webapp::routes())
        .layer(ServiceBuilder::new().layer(logger).into_inner())
        .layer(RequestBodyLimitLayer::new(1024 * 50))
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
