use chrono::NaiveDateTime;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Blogpost {
    pub text: String,
    pub author_username: String,
    pub published: NaiveDateTime,
    pub image_base64: Option<String>,
    pub avatar_base64: Option<String>,
}

impl Blogpost {
    pub fn new(
        text: String,
        author_username: String,
        image_base64: Option<String>,
        avatar_base64: Option<String>,
    ) -> Self {
        Self {
            text,
            author_username,
            published: chrono::Local::now().naive_local(),
            image_base64,
            avatar_base64,
        }
    }
}
