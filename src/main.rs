mod backend;
mod blogpost;
mod db;

use crate::backend::{fallback, get_home, handle_form_submit};
use axum::{response::Redirect, routing::get, Router};

fn app() -> Router {
    axum::Router::new()
        .fallback(fallback)
        .route("/", get(|| async { Redirect::permanent("/home") }))
        .route("/home", get(get_home).post(handle_form_submit))
}

#[tokio::main]
async fn main() {
    env_logger::init(); // TODO: setup logging
    db::create_db_schema().unwrap();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app()).await.unwrap();
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, extract::Request};
    use tower::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn nonexisting_url_returns_404() {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/nonexisting")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 404);
    }

    #[tokio::test]
    async fn root_uri_returns_redirect() {
        let response = app()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), 308);
    }
}
