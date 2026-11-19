//! Ouverture de la base et réglages posés à chaque connexion.

use std::cmp::Ordering;
use std::path::Path;

use rusqlite::Connection;

/// Collation utilisée pour trier les titres : sans casse ni accents
/// (« Série » entre « Sanglier » et « Sintel »).
pub const TITLE_COLLATION: &str = "TITLE";

pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    configure(&conn, true)?;
    Ok(conn)
}

/// Base en mémoire (tests), avec les mêmes réglages.
pub fn open_in_memory() -> rusqlite::Result<Connection> {
    let conn = Connection::open_in_memory()?;
    configure(&conn, false)?;
    Ok(conn)
}

fn configure(conn: &Connection, wal: bool) -> rusqlite::Result<()> {
    // foreign_keys est indispensable : le schéma repose sur les cascades.
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000;")?;
    if wal {
        conn.pragma_update_and_check(None, "journal_mode", "WAL", |_| Ok(()))?;
    }
    conn.create_collation(TITLE_COLLATION, compare_titles)?;
    // fold(texte) : pour chercher sans tenir compte de la casse ni des accents.
    conn.create_scalar_function(
        "fold",
        1,
        rusqlite::functions::FunctionFlags::SQLITE_UTF8
            | rusqlite::functions::FunctionFlags::SQLITE_DETERMINISTIC,
        |ctx| Ok(ctx.get::<Option<String>>(0)?.map(|s| fold(&s))),
    )?;
    Ok(())
}

pub fn compare_titles(a: &str, b: &str) -> Ordering {
    fold(a).cmp(&fold(b)).then_with(|| a.cmp(b))
}

/// Minuscules sans accents (lettres latines courantes).
pub fn fold(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' => 'a',
            'ç' => 'c',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ñ' => 'n',
            'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'ý' | 'ÿ' => 'y',
            c => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tri_sans_casse_ni_accents() {
        let mut v = vec!["Sintel", "Série Démo", "big buck", "Élan", "Artiste"];
        v.sort_by(|a, b| compare_titles(a, b));
        assert_eq!(v, ["Artiste", "big buck", "Élan", "Série Démo", "Sintel"]);
    }
}
