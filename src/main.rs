mod blogpost;
mod db;
use askama::Template;
use axum::response::Html;
use axum::routing::get;
use blogpost::Blogpost;
use db::insert_blogpost;
use serde::Deserialize;

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

#[derive(Deserialize)]
struct FormData {
    author_username: String,
    text: String,
    avatar_url: String,
    image: String,
}

async fn handle_form_submit(form: axum::extract::Form<FormData>) -> Html<&'static str> {
    let form_data: FormData = form.0;
    let new_post = Blogpost::from_form_data(form_data);
    insert_blogpost(&db::create_db().unwrap(), new_post).unwrap();
    "Success".into()
}
