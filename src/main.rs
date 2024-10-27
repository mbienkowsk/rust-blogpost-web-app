mod backend;
mod blogpost;
mod db;

use crate::backend::{fallback, get_home, handle_form_submit};
use axum::{response::Redirect, routing::get};

#[tokio::main]
async fn main() {
    env_logger::init(); // TODO: setup logging
    db::create_db_schema().unwrap();

    let app = axum::Router::new()
        .fallback(fallback)
        .route("/", get(|| async { Redirect::permanent("/home") }))
        .route("/home", get(get_home).post(handle_form_submit));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
