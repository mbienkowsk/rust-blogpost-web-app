use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
};

#[derive(Debug, PartialEq)]
pub struct AppError {
    message: String,
    status_code: StatusCode,
}

impl AppError {
    pub fn new(message: &str, status_code: StatusCode) -> Self {
        Self {
            message: message.to_string(),
            status_code,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let body = Html(self.message);
        (self.status_code, body).into_response()
    }
}

pub fn internal_server_error() -> AppError {
    AppError::new(
        "Internal server error. Try again later.",
        StatusCode::INTERNAL_SERVER_ERROR,
    )
}

pub fn avatar_download_error() -> AppError {
    AppError::new("Failed to download avatar.", StatusCode::BAD_REQUEST)
}

pub fn form_error() -> AppError {
    AppError::new("Invalid form data.", StatusCode::BAD_REQUEST)
}

pub fn invalid_image_format_error() -> AppError {
    AppError::new(
        "Invalid image format. Accepting only PNG.",
        StatusCode::BAD_REQUEST,
    )
}

pub fn invalid_avatar_url_error() -> AppError {
    AppError::new("Invalid avatar URL.", StatusCode::BAD_REQUEST)
}
