//! Dépôts SQLite : un fichier par table (ou groupe de tables).
//!
//! [`Store`] possède la connexion et implémente les traits de
//! `domain::ports`. Il n'est pas `Sync` : l'API le garde derrière un mutex
//! (un seul écrivain) et l'appelle depuis `spawn_blocking`.

mod app_state;
mod devices;
mod extensions;
mod favorites;
mod library;
mod media;
mod playlists;
mod profiles;
mod progress;
mod queue;
mod sources;

#[doc(hidden)]
pub use media::insert_test_media;

use rusqlite::{Connection, ErrorCode};

use epikodi_domain::DomainError;

use crate::secrets::SecretBox;

pub struct Store {
    conn: Connection,
    secrets: Option<SecretBox>,
}

impl Store {
    /// `secrets` : nécessaire pour enregistrer des identifiants de source.
    pub fn new(conn: Connection, secrets: Option<SecretBox>) -> Self {
        Self { conn, secrets }
    }

    /// Base en mémoire avec les vraies migrations et une clé de test.
    pub fn in_memory() -> Self {
        let conn = crate::init_in_memory().expect("base en mémoire");
        Self::new(conn, Some(SecretBox::from_key([42; 32])))
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Exécute `f` dans une transaction (validée si `f` réussit).
    pub fn transaction<T>(
        &self,
        f: impl FnOnce(&Connection) -> Result<T, DomainError>,
    ) -> Result<T, DomainError> {
        let tx = self.conn.unchecked_transaction().map_err(db_err)?;
        let out = f(&tx)?;
        tx.commit().map_err(db_err)?;
        Ok(out)
    }

    fn secrets(&self) -> Result<&SecretBox, DomainError> {
        self.secrets
            .as_ref()
            .ok_or_else(|| DomainError::Internal("aucune clé de chiffrement configurée".into()))
    }
}

/// Erreur SQLite → erreur du domaine.
pub(crate) fn db_err(e: rusqlite::Error) -> DomainError {
    DomainError::Internal(format!("base de données : {e}"))
}

/// Comme [`db_err`], mais une violation d'unicité devient `Conflict`.
pub(crate) fn unique_err(entity: &'static str) -> impl Fn(rusqlite::Error) -> DomainError {
    move |e| match &e {
        rusqlite::Error::SqliteFailure(f, _) if f.code == ErrorCode::ConstraintViolation => {
            DomainError::Conflict { entity }
        }
        _ => db_err(e),
    }
}

/// Lit un enum stocké en texte.
pub(crate) fn parse_col<T: std::str::FromStr>(idx: usize, text: String) -> rusqlite::Result<T> {
    text.parse().map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            idx,
            rusqlite::types::Type::Text,
            format!("valeur inattendue : {text}").into(),
        )
    })
}

/// Normalise une date SQLite (`2026-11-07 18:31:12`) en ISO 8601 UTC.
pub(crate) fn iso(text: String) -> String {
    if text.contains('T') {
        text
    } else {
        format!("{}Z", text.replacen(' ', "T", 1))
    }
}

pub(crate) fn iso_opt(text: Option<String>) -> Option<String> {
    text.map(iso)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_sqlite_normalisees() {
        assert_eq!(iso("2026-11-07 18:31:12".into()), "2026-11-07T18:31:12Z");
        assert_eq!(iso("2026-11-07T18:31:12Z".into()), "2026-11-07T18:31:12Z");
    }

    #[test]
    fn base_neuve_a_le_bon_schema() {
        let store = Store::in_memory();
        let version: usize = store
            .conn()
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, crate::migrate::LATEST);
        let fk: i64 = store
            .conn()
            .pragma_query_value(None, "foreign_keys", |r| r.get(0))
            .unwrap();
        assert_eq!(fk, 1, "foreign_keys doit être actif");
        let state: i64 = store
            .conn()
            .query_row("SELECT COUNT(*) FROM app_state", [], |r| r.get(0))
            .unwrap();
        assert_eq!(state, 1, "app_state a une seule ligne");
    }

    #[test]
    fn migrations_idempotentes() {
        let store = Store::in_memory();
        crate::migrate::run(store.conn()).unwrap();
        crate::migrate::run(store.conn()).unwrap();
    }

    #[test]
    fn migration_depuis_une_base_v1_existante() {
        // une base créée avant 0002 (user_version = 1) est mise à niveau
        let conn = crate::db::open_in_memory().unwrap();
        conn.execute_batch(include_str!("../../migrations/schema.sql"))
            .unwrap();
        conn.pragma_update(None, "user_version", 1).unwrap();
        conn.execute("INSERT INTO user (name, age) VALUES ('Ancien', 30)", [])
            .unwrap();
        crate::migrate::run(&conn).unwrap();
        let pin: Option<String> = conn
            .query_row("SELECT pin_hash FROM user WHERE name = 'Ancien'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert!(pin.is_none());
    }
}
