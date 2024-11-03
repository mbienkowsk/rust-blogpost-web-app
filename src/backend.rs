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
        //TODO: status codes??
        .map_err(|e| e.to_string())?;

    let request = client
        .get(url)
        .header("Accept", "image/png")
        .build()
        .map_err(|e| e.to_string())?;

    let response = client.execute(request).await.unwrap();

    if !response.status().is_success() {
        Err("Failed to download avatar".to_string())
    } else {
        validate_png_header(response.headers())?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Bytes;

    // These are used to test png Byte validation
    // Smallest possible valid PNG image
    const MINIMAL_PNG_DATA: [u8; 45] = [
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG Signature
        0x00, 0x00, 0x00, 0x0D, // IHDR Length
        0x49, 0x48, 0x44, 0x52, // IHDR Chunk Type
        0x00, 0x00, 0x00, 0x01, // Width: 1 pixel
        0x00, 0x00, 0x00, 0x01, // Height: 1 pixel
        0x08, // Bit depth: 8
        0x02, // Color type: Truecolor
        0x00, // Compression method: default
        0x00, // Filter method: default
        0x00, // Interlace method: no interlace
        0x90, 0x77, 0x53, 0xDE, // CRC for IHDR
        0x00, 0x00, 0x00, 0x00, // IEND Length
        0x49, 0x45, 0x4E, 0x44, // IEND Chunk Type
        0xAE, 0x42, 0x60, 0x82, // CRC for IEND
    ];

    // Smallest possible valid WebP image
    const MINIMAL_WEBP_DATA: [u8; 12] = [
        0x52, 0x49, 0x46, 0x46, 0x0A, 0x00, 0x00, 0x00, // RIFF Header
        0x57, 0x45, 0x42, 0x50, // "WEBP" Signature
    ];

    #[tokio::test]
    async fn test_download_avatar_success() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock("GET", "/")
            .with_status(200)
            .with_header("Content-Type", "image/png")
            .with_body(Bytes::from_static(&MINIMAL_PNG_DATA))
            .create_async()
            .await;

        let server_url = url::Url::parse(&server.url()).unwrap();
        let result = download_avatar(server_url);
        assert!(result.await.is_ok());
    }

    #[tokio::test]
    async fn test_download_avatar_invalid_response_headers() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock("GET", "/wrong-content-type")
            .with_status(200)
            .with_header("Content-Type", "image/jpeg")
            .create_async()
            .await;

        server
            .mock("GET", "/no-content-type")
            .with_status(200)
            .create_async()
            .await;

        let server_url_wrong =
            url::Url::parse(&format!("{}/wrong-content-type", server.url())).unwrap();
        let server_url_none =
            url::Url::parse(&format!("{}/no-content-type", server.url())).unwrap();

        let result_wrong = download_avatar(server_url_wrong).await;
        let result_none = download_avatar(server_url_none).await;

        assert_eq!(
            result_wrong,
            Err("Invalid content type. Make sure the URL points to a PNG image.".to_string())
        );
        assert_eq!(result_none, Err("No content type header found".to_string()));
    }

    #[tokio::test]
    async fn test_download_avatar_dead_url() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock("GET", "/")
            .with_status(404)
            .create_async()
            .await;

        let server_url = url::Url::parse(&server.url()).unwrap();
        let result = download_avatar(server_url).await;
        assert_eq!(result, Err("Failed to download avatar".to_string()));
    }

    #[tokio::test]
    async fn test_download_avatar_invalid_payload() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock("GET", "/")
            .with_status(200)
            .with_header("Content-Type", "image/png")
            .with_body(Bytes::from("not an image"))
            .create_async()
            .await;

        let server_url = url::Url::parse(&server.url()).unwrap();
        let result = download_avatar(server_url).await;
        assert_eq!(
            result,
            Err(
                "Could not determine image format! Make sure the url points to a png image."
                    .to_string()
            )
        );
    }

    #[tokio::test]
    async fn test_download_avatar_invalid_image_type_png_header() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock("GET", "/")
            .with_status(200)
            .with_header("Content-Type", "image/png")
            .with_body(Bytes::from_static(&MINIMAL_WEBP_DATA))
            .create_async()
            .await;

        let server_url = url::Url::parse(&server.url()).unwrap();
        let result = download_avatar(server_url).await;
        assert_eq!(
            result,
            Err("Invalid image format! Accepting only PNG".to_string())
        );
    }
}
