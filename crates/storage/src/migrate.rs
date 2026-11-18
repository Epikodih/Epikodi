//! Migrations, suivies par `PRAGMA user_version`.
//!
//! Ne jamais modifier une migration déjà livrée : on en ajoute une nouvelle
//! à la fin de [`MIGRATIONS`].

use rusqlite::Connection;

const MIGRATIONS: &[&str] = &[
    include_str!("../migrations/schema.sql"),
    include_str!("../migrations/0002_profiles_devices.sql"),
];

/// Version du schéma après toutes les migrations.
pub const LATEST: usize = MIGRATIONS.len();

pub fn run(conn: &Connection) -> rusqlite::Result<()> {
    let current: usize = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(current) {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(sql)?;
        tx.pragma_update(None, "user_version", i + 1)?;
        tx.commit()?;
        tracing::info!(version = i + 1, "migration appliquée");
    }
    Ok(())
}
