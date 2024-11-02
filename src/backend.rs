use crate::blogpost::Blogpost;
use crate::db;
use askama::Template;
use axum::response::Html;
use axum::{body::Bytes, extract::Multipart};
use base64::{prelude::BASE64_STANDARD, Engine};
use image::ImageFormat;
use std::io::Cursor;
use std::time::Duration;
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

pub async fn fallback(uri: axum::http::Uri) -> (axum::http::StatusCode, String) {
    (
        axum::http::StatusCode::NOT_FOUND,
        format!("No route {}", uri),
    )
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

// Download a png avatar from the given URL and return it as a base64 encoded string
async fn download_avatar(url: Url) -> Result<Option<String>, String> {
    let client = reqwest::ClientBuilder::new()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let request = client
        .get(url)
        .header("Accept", "image/png")
        .build()
        .map_err(|e| e.to_string())?;

    let response = client.execute(request).await.unwrap();
    validate_png_header(response.headers())?;

    if !response.status().is_success() {
        Err("Failed to download avatar".to_string())
    } else {
        let bytes = response.bytes().await.unwrap();
        validate_bytes_as_png(&bytes)?;
        let rv = BASE64_STANDARD.encode(bytes);
        Ok(Some(rv))
    }
}

// Verify that the bytes downloaded from a given URL are a valid PNG image
fn validate_bytes_as_png(image_bytes: &Bytes) -> Result<(), String> {
    match image::ImageReader::new(Cursor::new(image_bytes))
        .with_guessed_format()
        // Only cursor IO errors here
        .map_err(|_| String::from("Error while parsing the image. Try again."))?
        .format()
    {
        Some(ImageFormat::Png) => Ok(()),
        Some(_) => Err(String::from("Invalid image format! Accepting only PNG")),
        None => Err(String::from(
            "Could not determine image format! Make sure the url points to a png image.",
        )),
    }
}

fn validate_png_header(headers: &axum::http::HeaderMap) -> Result<(), String> {
    let content_type = headers
        .get("Content-Type")
        .ok_or("No content type header found")?
        .to_str()
        .map_err(|e| e.to_string())?;

    if content_type != "image/png" {
        return Err(String::from(
            "Invalid content type. Make sure the URL points to a PNG image.",
        ));
    }
    Ok(())
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
