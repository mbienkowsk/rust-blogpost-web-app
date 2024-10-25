use crate::blogpost::Blogpost;
use rusqlite::{params, Connection, Result, ToSql};

pub fn create_db() -> Result<Connection> {
    let conn = Connection::open("blog.db")?;
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

    Ok(conn)
}

pub fn insert_blogpost(conn: &Connection, blogpost: Blogpost) -> Result<()> {
    conn.execute(
        "
        INSERT INTO blogposts (text, publication_date, image, username, avatar)
        VALUES (?1, ?2, ?3, ?4, ?5);
        ",
        [
            &blogpost.text,
            &blogpost.published as &dyn ToSql,
            &blogpost.image_base64 as &dyn ToSql,
            &blogpost.author_username,
            &blogpost.avatar_base64 as &dyn ToSql,
        ],
    )?;
    Ok(())
}

pub fn get_all_blogposts(conn: &Connection) -> Result<Vec<Blogpost>> {
    let mut stmt = conn.prepare(
        "
        SELECT text, publication_date, image, username, avatar
        FROM blogposts;
        ",
    )?;
    let blogposts = stmt
        .query_map([], |row| {
            Ok(Blogpost {
                text: row.get(0)?,
                published: row.get(1)?,
                image_base64: row.get(2)?,
                author_username: row.get(3)?,
                avatar_base64: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<Blogpost>>>()?;
    Ok(blogposts)
}
