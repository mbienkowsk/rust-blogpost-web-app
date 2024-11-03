use crate::blogpost::Blogpost;
use crate::db;
use crate::error::{
    avatar_download_error, form_error, internal_server_error, invalid_avatar_url_error,
    invalid_image_format_error, AppError,
};
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

pub async fn fallback(uri: axum::http::Uri) -> (axum::http::StatusCode, String) {
    (
        axum::http::StatusCode::NOT_FOUND,
        format!("No route {}", uri),
    )
}

pub async fn get_home() -> Result<Html<String>, AppError> {
    match db::get_all_blogposts() {
        Ok(posts) => BlogTemplate { posts }
            .render()
            .map(Html)
            .map_err(|_| internal_server_error()),
        Err(_) => Err(internal_server_error()),
    }
}

pub async fn handle_form_submit(
    multipart: axum::extract::Multipart,
) -> Result<Html<String>, AppError> {
    let multipart_data = match parse_multipart(multipart).await {
        Ok(data) => data,
        Err(e) => return Err(e),
    };

    let new_post = match create_blogpost(multipart_data).await {
        Ok(post) => post,
        Err(e) => return Err(e),
    };

    if db::insert_blogpost(new_post).is_err() {
        return Err(internal_server_error());
    }

    get_home().await
}

async fn create_blogpost(multipart_data: MultipartData) -> Result<Blogpost, AppError> {
    let mut new_post = Blogpost::new(
        multipart_data.text,
        multipart_data.author_username,
        multipart_data.image_base64,
        None,
    );

    if let Some(url) = multipart_data.avatar_url {
        let parsed_url = Url::parse(&url).map_err(|_| invalid_avatar_url_error())?;

        match download_avatar(parsed_url).await {
            Ok(avatar_base64) => new_post.avatar_base64 = avatar_base64,
            Err(e) => return Err(e),
        }
    }

    Ok(new_post)
}

// Download a png avatar from the given URL and return it as a base64 encoded string
async fn download_avatar(url: Url) -> Result<Option<String>, AppError> {
    let client = reqwest::ClientBuilder::new()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|_| internal_server_error())?;

    let request = client
        .get(url)
        .header("Accept", "image/png")
        .build()
        .map_err(|_| internal_server_error())?;

    let response = client
        .execute(request)
        .await
        .map_err(|_| avatar_download_error())?;

    handle_avatar_response(response).await
}

async fn handle_avatar_response(response: reqwest::Response) -> Result<Option<String>, AppError> {
    if !response.status().is_success() {
        return Err(avatar_download_error());
    }

    validate_png_header(response.headers())?;
    let bytes = response
        .bytes()
        .await
        .map_err(|_| avatar_download_error())?;
    validate_bytes_as_png(&bytes)?;
    let rv = BASE64_STANDARD.encode(bytes);
    Ok(Some(rv))
}

// Verify that the bytes downloaded from a given URL are a valid PNG image
fn validate_bytes_as_png(image_bytes: &Bytes) -> Result<(), AppError> {
    match image::ImageReader::new(Cursor::new(image_bytes))
        .with_guessed_format()
        // Only cursor IO errors here
        .map_err(|_| internal_server_error())?
        .format()
    {
        Some(ImageFormat::Png) => Ok(()),
        Some(_) => Err(invalid_image_format_error()),
        None => Err(invalid_image_format_error()),
    }
}

fn validate_png_header(headers: &axum::http::HeaderMap) -> Result<(), AppError> {
    let content_type = headers
        .get("Content-Type")
        .ok_or(invalid_image_format_error())?
        .to_str()
        .map_err(|_| internal_server_error())?;

    if content_type != "image/png" {
        return Err(invalid_image_format_error());
    }
    Ok(())
}

struct MultipartData {
    author_username: String,
    text: String,
    avatar_url: Option<String>,
    image_base64: Option<String>,
}

async fn parse_multipart(mut multipart: Multipart) -> Result<MultipartData, AppError> {
    let mut data = MultipartData {
        author_username: String::new(),
        text: String::new(),
        avatar_url: None,
        image_base64: None,
    };

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| internal_server_error())?
    {
        let name = field.name().ok_or(form_error())?;

        match name {
            "text" => {
                data.text = field.text().await.map_err(|_| form_error())?;
            }
            "author_username" => {
                data.author_username = field.text().await.map_err(|_| form_error())?;
            }
            "image" => {
                let bytes = field.bytes().await.map_err(|_| form_error())?;
                if !bytes.is_empty() {
                    validate_bytes_as_png(&bytes)?;
                    data.image_base64 = Some(BASE64_STANDARD.encode(bytes));
                }
            }
            "avatar_url" => {
                let text = field.text().await.map_err(|_| form_error())?;
                data.avatar_url = Some(text).filter(|x| !x.is_empty());
            }
            _ => continue,
        }
    }

    Ok(data)
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

        assert_eq!(result_wrong, Err(invalid_image_format_error()));
        assert_eq!(result_none, Err(invalid_image_format_error()));
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
        assert_eq!(result, Err(avatar_download_error()));
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
        assert_eq!(result, Err(invalid_image_format_error()));
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
        assert_eq!(result, Err(invalid_image_format_error()));
    }
}
