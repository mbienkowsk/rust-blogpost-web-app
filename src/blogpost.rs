use crate::FormData;
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
    pub fn from_form_data(form_data: FormData) -> Blogpost {
        // Todo: download avatar image
        Blogpost {
            text: form_data.text,
            author_username: form_data.author_username,
            published: chrono::Local::now().naive_local(),
            avatar_base64: None,
            image_base64: None,
        }
    }
}
