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

    /// Exécute `f` d'un bloc : tout est validé si `f` réussit, tout est
    /// annulé si `f` échoue (ou panique). Imbricable : un appel à l'intérieur
    /// d'un autre devient un point de sauvegarde (SAVEPOINT) qui n'annule que
    /// sa propre partie.
    ///
    /// La transaction la plus externe prend le verrou d'écriture dès le
    /// départ (`BEGIN IMMEDIATE`, avec attente via `busy_timeout`) : l'API et
    /// le scan écrivent sur deux connexions, et une transaction qui lit puis
    /// écrit échouerait sinon aussitôt si l'autre a écrit entre-temps.
    pub fn transaction<T, E: From<DomainError>>(
        &self,
        f: impl FnOnce(&Connection) -> Result<T, E>,
    ) -> Result<T, E> {
        let outer = self.conn.is_autocommit();
        let (begin, commit) = if outer {
            ("BEGIN IMMEDIATE", "COMMIT")
        } else {
            ("SAVEPOINT tx", "RELEASE tx")
        };
        self.conn.execute_batch(begin).map_err(db_err)?;
        let mut guard = Rollback {
            conn: &self.conn,
            outer,
            armed: true,
        };
        let out = f(&self.conn)?;
        self.conn.execute_batch(commit).map_err(db_err)?;
        guard.armed = false;
        drop(guard);
        Ok(out)
    }
    fn secrets(&self) -> Result<&SecretBox, DomainError> {
        self.secrets
            .as_ref()
            .ok_or_else(|| DomainError::Internal("aucune clé de chiffrement configurée".into()))
    }
}

/// Annule le point de sauvegarde courant s'il n'a pas été validé (erreur,
/// `?` ou panique dans la transaction).
struct Rollback<'a> {
    conn: &'a Connection,
    outer: bool,
    armed: bool,
}

impl Drop for Rollback<'_> {
    fn drop(&mut self) {
        if self.armed {
            let sql = if self.outer {
                "ROLLBACK"
            } else {
                "ROLLBACK TO tx; RELEASE tx"
            };
            if let Err(e) = self.conn.execute_batch(sql) {
                tracing::error!("annulation de transaction impossible : {e}");
            }
        }
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

    fn count(store: &Store) -> i64 {
        store
            .conn()
            .query_row("SELECT COUNT(*) FROM playlist", [], |r| r.get(0))
            .unwrap()
    }

    fn insert(store: &Store, name: &str) -> Result<(), DomainError> {
        store
            .conn()
            .execute(
                "INSERT INTO playlist (user_id, name) VALUES (1, ?1)",
                [name],
            )
            .map(|_| ())
            .map_err(db_err)
    }

    #[test]
    fn transaction_annulee_en_cas_d_erreur() {
        let store = Store::in_memory();
        store
            .conn()
            .execute("INSERT INTO user (name, age) VALUES ('U', 30)", [])
            .unwrap();
        let r: Result<(), _> = store.transaction(|_| {
            insert(&store, "a")?;
            Err(DomainError::Rule("échec".into()))
        });
        assert!(r.is_err());
        assert_eq!(count(&store), 0, "tout est annulé");
        assert!(
            store.conn().is_autocommit(),
            "aucune transaction laissée ouverte"
        );
    }

    #[test]
    fn transactions_imbriquees() {
        let store = Store::in_memory();
        store
            .conn()
            .execute("INSERT INTO user (name, age) VALUES ('U', 30)", [])
            .unwrap();
        store
            .transaction(|_| {
                insert(&store, "a")?;
                // l'échec de la partie interne n'annule qu'elle
                let inner: Result<(), _> = store.transaction(|_| {
                    insert(&store, "b")?;
                    Err(DomainError::Rule("échec".into()))
                });
                assert!(inner.is_err());
                Ok::<_, DomainError>(())
            })
            .unwrap();
        assert_eq!(count(&store), 1);
        // l'échec de la partie externe annule aussi l'interne validée
        let r: Result<(), _> = store.transaction(|_| {
            store.transaction(|_| insert(&store, "c"))?;
            Err(DomainError::Rule("échec".into()))
        });
        assert!(r.is_err());
        assert_eq!(count(&store), 1);
    }

    #[test]
    fn transaction_annulee_en_cas_de_panique() {
        let store = Store::in_memory();
        store
            .conn()
            .execute("INSERT INTO user (name, age) VALUES ('U', 30)", [])
            .unwrap();
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = store.transaction(|_| -> Result<(), DomainError> {
                insert(&store, "a")?;
                panic!("bug");
            });
        }));
        assert!(r.is_err());
        assert_eq!(count(&store), 0);
        assert!(store.conn().is_autocommit());
    }

    /// Deux connexions sur le même fichier (API + scan) : la seconde attend
    /// le verrou au lieu d'échouer.
    #[test]
    fn deux_connexions_ecrivent_sans_conflit() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("t.db");
        let open = || Store::new(crate::init(&path).unwrap(), None);
        let a = open();
        a.conn()
            .execute("INSERT INTO user (name, age) VALUES ('U', 30)", [])
            .unwrap();
        let path2 = path.clone();
        let writer = std::thread::spawn(move || {
            let b = Store::new(crate::init(&path2).unwrap(), None);
            for i in 0..50 {
                b.transaction(|c| {
                    let _: i64 = c
                        .query_row("SELECT COUNT(*) FROM playlist", [], |r| r.get(0))
                        .map_err(db_err)?;
                    c.execute(
                        "INSERT INTO playlist (user_id, name) VALUES (1, ?1)",
                        [format!("b{i}")],
                    )
                    .map_err(db_err)?;
                    Ok::<_, DomainError>(())
                })
                .unwrap();
            }
        });
        for i in 0..50 {
            a.transaction(|c| {
                let _: i64 = c
                    .query_row("SELECT COUNT(*) FROM playlist", [], |r| r.get(0))
                    .map_err(db_err)?;
                c.execute(
                    "INSERT INTO playlist (user_id, name) VALUES (1, ?1)",
                    [format!("a{i}")],
                )
                .map_err(db_err)?;
                Ok::<_, DomainError>(())
            })
            .unwrap();
        }
        writer.join().unwrap();
        assert_eq!(count(&a), 100);
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
