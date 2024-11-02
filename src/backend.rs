use crate::blogpost::Blogpost;
use crate::db;
use askama::Template;
use axum::extract::Multipart;
use axum::response::Html;
use base64::{prelude::BASE64_STANDARD, Engine};
use url::Url;

#[derive(Template)]
#[template(path = "base.html")]
pub struct BlogTemplate {
    pub posts: Vec<Blogpost>,
}

pub fn get_populated_template_value() -> String {
    let posts = db::get_all_blogposts().unwrap();
    BlogTemplate { posts }.render().unwrap()
}

pub async fn fallback(uri: axum::http::Uri) -> Html<String> {
    format!("No route {}", uri).into()
}

pub async fn get_home() -> Html<String> {
    get_populated_template_value().into()
}

pub async fn handle_form_submit(multipart: axum::extract::Multipart) -> Html<String> {
    let multipart_data = parse_multipart(multipart).await;

    let mut new_post = Blogpost::new(
        multipart_data.text,
        multipart_data.author_username,
        multipart_data.image_base64,
        None,
    );

    let avatar_url = multipart_data.avatar_url;

    if let Some(url) = avatar_url {
        let url = Url::parse(&url).unwrap();
        let download_result = download_avatar(url).await;
        match download_result {
            Ok(value) => new_post.avatar_base64 = value,
            Err(e) => return e.into(),
        }
    }

    db::insert_blogpost(new_post).unwrap();

    get_populated_template_value().into()
}

async fn download_avatar(url: Url) -> Result<Option<String>, String> {
    let response = reqwest::get(url).await.unwrap();
    if !response.status().is_success() {
        Err("Failed to download avatar".to_string())
    } else {
        // TODO: verify file type?
        let bytes = response.bytes().await.unwrap();
        let rv = BASE64_STANDARD.encode(bytes);
        Ok(Some(rv))
    }
}

struct MultipartData {
    author_username: String,
    text: String,
    avatar_url: Option<String>,
    image_base64: Option<String>,
}

async fn parse_multipart(mut multipart: Multipart) -> MultipartData {
    let mut data = MultipartData {
        author_username: String::new(),
        text: String::new(),
        avatar_url: None,
        image_base64: None,
    };

    while let Some(field) = multipart.next_field().await.unwrap() {
        match field.name().unwrap() {
            "text" => {
                data.text = field.text().await.unwrap();
            }
            "author_username" => {
                data.author_username = field.text().await.unwrap();
            }
            "image" => {
                let bytes = field.bytes().await.unwrap();
                if !bytes.is_empty() {
                    data.image_base64 = Some(BASE64_STANDARD.encode(bytes));
                }
            }
            "avatar_url" => {
                let text = field.text().await.unwrap();
                data.avatar_url = Some(text).filter(|x| !x.is_empty());
            }
            _ => continue,
        }
    }

    data
}
