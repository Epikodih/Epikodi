use rusqlite::{params, OptionalExtension, Row};

use epikodi_domain::ports::{NewSource, SourceRepo};
use epikodi_domain::source::{Credentials, Source, SourceStatus};
use epikodi_domain::{DomainError, DomainResult, Id};

use super::{db_err, iso_opt, parse_col, unique_err, Store};

const COLS: &str =
    "id, name, protocol, uri, credentials_encrypted IS NOT NULL, status, last_scan_at";

fn row(r: &Row) -> rusqlite::Result<Source> {
    Ok(Source {
        id: r.get(0)?,
        name: r.get(1)?,
        protocol: parse_col(2, r.get(2)?)?,
        uri: r.get(3)?,
        has_credentials: r.get(4)?,
        status: parse_col(5, r.get(5)?)?,
        last_scan_at: iso_opt(r.get(6)?),
    })
}

impl SourceRepo for Store {
    fn list_sources(&self) -> DomainResult<Vec<Source>> {
        let mut st = self
            .conn
            .prepare(&format!(
                "SELECT {COLS} FROM source ORDER BY name COLLATE TITLE"
            ))
            .map_err(db_err)?;
        let rows = st.query_map([], row).map_err(db_err)?;
        rows.collect::<Result<_, _>>().map_err(db_err)
    }

    fn get_source(&self, id: Id) -> DomainResult<Option<Source>> {
        self.conn
            .query_row(
                &format!("SELECT {COLS} FROM source WHERE id = ?1"),
                [id],
                row,
            )
            .optional()
            .map_err(db_err)
    }

    fn create_source(&self, s: &NewSource) -> DomainResult<Source> {
        let blob = match &s.credentials {
            Some(c) => Some(self.secrets()?.encrypt_credentials(c)?),
            None => None,
        };
        self.conn
            .execute(
                "INSERT INTO source (name, protocol, uri, credentials_encrypted) VALUES (?1, ?2, ?3, ?4)",
                params![s.name, s.protocol.as_str(), s.uri, blob],
            )
            .map_err(unique_err("source"))?;
        let id = self.conn.last_insert_rowid();
        self.get_source(id)?
            .ok_or(DomainError::NotFound { entity: "source" })
    }

    fn rename_source(&self, id: Id, name: &str) -> DomainResult<()> {
        self.conn
            .execute(
                "UPDATE source SET name = ?2 WHERE id = ?1",
                params![id, name],
            )
            .map_err(db_err)?;
        Ok(())
    }

    fn set_source_credentials(&self, id: Id, creds: Option<&Credentials>) -> DomainResult<()> {
        let blob = match creds {
            Some(c) => Some(self.secrets()?.encrypt_credentials(c)?),
            None => None,
        };
        self.conn
            .execute(
                "UPDATE source SET credentials_encrypted = ?2 WHERE id = ?1",
                params![id, blob],
            )
            .map_err(db_err)?;
        Ok(())
    }

    fn source_credentials(&self, id: Id) -> DomainResult<Option<Credentials>> {
        let blob: Option<Vec<u8>> = self
            .conn
            .query_row(
                "SELECT credentials_encrypted FROM source WHERE id = ?1",
                [id],
                |r| r.get(0),
            )
            .optional()
            .map_err(db_err)?
            .flatten();
        blob.map(|b| self.secrets()?.decrypt_credentials(&b))
            .transpose()
    }

    fn set_source_status(&self, id: Id, status: SourceStatus) -> DomainResult<()> {
        self.conn
            .execute(
                "UPDATE source SET status = ?2 WHERE id = ?1",
                params![id, status.as_str()],
            )
            .map_err(db_err)?;
        Ok(())
    }

    fn delete_source(&self, id: Id) -> DomainResult<()> {
        self.transaction(|tx| {
            // Les médias de la source partent avec elle, puis les conteneurs
            // devenus vides (saison, série, album, artiste).
            tx.execute("DELETE FROM media WHERE source_id = ?1", [id])
                .map_err(db_err)?;
            for _ in 0..3 {
                tx.execute(
                    "DELETE FROM media WHERE uri IS NULL AND kind <> 'podcast'
                     AND NOT EXISTS (SELECT 1 FROM media c WHERE c.parent_id = media.id)",
                    [],
                )
                .map_err(db_err)?;
            }
            let n = tx
                .execute("DELETE FROM source WHERE id = ?1", [id])
                .map_err(db_err)?;
            if n == 0 {
                return Err(DomainError::NotFound { entity: "source" });
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use epikodi_domain::source::Protocol;

    #[test]
    fn identifiants_chiffres_jamais_en_clair() {
        let s = Store::in_memory();
        let creds = Credentials {
            username: "marc".into(),
            password: "TresSecret!".into(),
            domain: None,
        };
        let src = s
            .create_source(&NewSource {
                name: "NAS".into(),
                protocol: Protocol::Smb,
                uri: "smb://nas/films".into(),
                credentials: Some(creds.clone()),
            })
            .unwrap();
        assert!(src.has_credentials);
        let raw: Vec<u8> = s
            .conn
            .query_row("SELECT credentials_encrypted FROM source", [], |r| r.get(0))
            .unwrap();
        assert!(!String::from_utf8_lossy(&raw).contains("TresSecret"));
        assert_eq!(s.source_credentials(src.id).unwrap(), Some(creds));
        s.set_source_credentials(src.id, None).unwrap();
        assert_eq!(s.source_credentials(src.id).unwrap(), None);
    }

    #[test]
    fn uri_unique_et_suppression() {
        let s = Store::in_memory();
        let new = NewSource {
            name: "Films".into(),
            protocol: Protocol::Local,
            uri: "file:///films".into(),
            credentials: None,
        };
        let src = s.create_source(&new).unwrap();
        assert!(matches!(
            s.create_source(&new),
            Err(DomainError::Conflict { .. })
        ));
        s.delete_source(src.id).unwrap();
        assert!(s.list_sources().unwrap().is_empty());
    }
}
