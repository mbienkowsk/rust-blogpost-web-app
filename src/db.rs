use crate::blogpost::Blogpost;
use rusqlite::{params, Connection, Result};

pub fn create_db_connection() -> Result<Connection> {
    let conn = Connection::open("blog.db")?;
    Ok(conn)
}

pub fn create_db_schema() -> Result<()> {
    let conn = create_db_connection()?;
    conn.execute(
        "
        CREATE TABLE IF NOT EXISTS blogposts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            text TEXT NOT NULL,
            publication_date DATE NOT NULL,
            image TEXT,
            username TEXT NOT NULL,
            avatar TEXT
        );
        ",
        [],
    )?;

    Ok(())
}

pub fn insert_blogpost(blogpost: Blogpost) -> Result<()> {
    create_db_connection()?.execute(
        "
        INSERT INTO blogposts (text, publication_date, image, username, avatar)
        VALUES (?1, ?2, ?3, ?4, ?5);
        ",
        params![
            blogpost.text,
            blogpost.published,
            blogpost.image_base64,
            blogpost.author_username,
            blogpost.avatar_base64,
        ],
    )?;
    Ok(())
}

pub fn get_all_blogposts() -> Result<Vec<Blogpost>> {
    let conn = create_db_connection()?;
    let mut stmt = conn.prepare(
        "
        SELECT text, publication_date, image, username, avatar
        FROM blogposts;
        ",
    )?;
    let blogposts = stmt
        .query_map([], |row| Ok(Blogpost::from_sqlite_row(row)))?
        .collect::<Result<Vec<Blogpost>>>()?
        .into_iter()
        .rev()
        .collect();

    Ok(blogposts)
}
