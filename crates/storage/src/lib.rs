//! Stockage : SQLite (seule crate qui y touche), migrations, caches, secrets.
//!
//! [`Store`] implémente tous les dépôts de `domain::ports`.

pub mod cache;
pub mod db;
pub mod json;
pub mod migrate;
pub mod repositories;
pub mod secrets;

use std::path::Path;

pub use repositories::{insert_test_media, Store};
pub use secrets::SecretBox;

/// Ouvre la base (créée si absente), pose les PRAGMA et applique les migrations.
pub fn init(path: &Path) -> rusqlite::Result<rusqlite::Connection> {
    let conn = db::open(path)?;
    migrate::run(&conn)?;
    Ok(conn)
}

/// Base en mémoire avec les vraies migrations (tests).
pub fn init_in_memory() -> rusqlite::Result<rusqlite::Connection> {
    let conn = db::open_in_memory()?;
    migrate::run(&conn)?;
    Ok(conn)
}

/// Date courante au format du protocole (ISO 8601 UTC, à la seconde).
pub fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}
