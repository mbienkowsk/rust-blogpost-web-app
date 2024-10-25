mod blogpost;
mod db;
use std::collections::HashMap;

use askama::Template;
use axum::extract::Multipart;
use axum::response::Html;
use axum::routing::get;
use base64::{prelude::BASE64_STANDARD, Engine};
use blogpost::Blogpost;
use url::Url;

#[tokio::main]
async fn main() {
    let app = axum::Router::new()
        .fallback(fallback)
        .route("/home", get(get_home).post(handle_form_submit));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[derive(Template)]
#[template(path = "base.html")]
struct BlogTemplate {
    posts: Vec<Blogpost>,
}

async fn fallback(uri: axum::http::Uri) -> Html<String> {
    format!("No route {}", uri).into()
}

async fn get_home() -> Html<String> {
    let posts = db::get_all_blogposts(&db::create_db().unwrap()).unwrap();
    let template = BlogTemplate {
        posts: posts.clone(),
    };
    template.render().unwrap().into()
}

async fn handle_form_submit(multipart: axum::extract::Multipart) -> Html<&'static str> {
    let multipart_data = parse_multipart(multipart).await;
    let avatar_base64 = download_avatar(multipart_data.get("avatar_base64"))
        .await
        .unwrap(); // TODO: don't unwrap, check
    let new_post = Blogpost::new(
        multipart_data.get("text").unwrap().to_string(),
        multipart_data.get("author_username").unwrap().to_string(),
        multipart_data.get("image_base64").map(|s| s.to_string()),
        avatar_base64,
    );
    db::insert_blogpost(&db::create_db().unwrap(), new_post).unwrap();

    "Success".into()
}

async fn download_avatar(url: Option<&String>) -> Result<Option<String>, &str> {
    match url {
        Some(url) => {
            let url = Url::parse(url).unwrap();
            let response = reqwest::get(url).await.unwrap();
            if !response.status().is_success() {
                Err("Failed to download avatar")
            } else {
                //TODO: parse the image somehow for security
                let bytes = response.bytes().await.unwrap();
                Ok(Some(BASE64_STANDARD.encode(bytes)))
            }
        }
        None => Ok(None),
    }
}

async fn parse_multipart(mut multipart: Multipart) -> HashMap<String, String> {
    let mut multipart_data = HashMap::<String, String>::new();

    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        let value = field.text().await.unwrap();
        multipart_data.insert(name.to_string(), value.to_string());
    }

    multipart_data
}
