pub mod db;
pub mod migrate;
pub mod json;
pub mod repositories;
pub mod cache;
pub mod secrets;


use std::path::Path;

/// Ouvre la base (créée si absente), pose les PRAGMA et applique les migrations.
pub fn init(path: &Path) -> rusqlite::Result<rusqlite::Connection> {
    let conn = db::open(path)?;
    migrate::run(&conn)?;
    Ok(conn)
}