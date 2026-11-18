use rusqlite::{params, OptionalExtension, Row};

use epikodi_domain::extension::{Extension, ExtensionStatus, Grants};
use epikodi_domain::ports::ExtensionRepo;
use epikodi_domain::{DomainError, DomainResult};

use super::{db_err, parse_col, Store};
use crate::json;

const COLS: &str =
    "id, name, version, type, install_path, enabled, status, last_error, permissions, settings";

fn row(r: &Row) -> rusqlite::Result<Extension> {
    Ok(Extension {
        id: r.get(0)?,
        name: r.get(1)?,
        version: r.get(2)?,
        kind: parse_col(3, r.get(3)?)?,
        install_path: r.get(4)?,
        enabled: r.get(5)?,
        status: parse_col(6, r.get(6)?)?,
        last_error: r.get(7)?,
        permissions: json::from_column::<Grants>(&r.get::<_, String>(8)?, "extension.permissions"),
        settings: json::from_column(&r.get::<_, String>(9)?, "extension.settings"),
    })
}

impl ExtensionRepo for Store {
    fn list_extensions(&self) -> DomainResult<Vec<Extension>> {
        let mut st = self
            .conn
            .prepare(&format!(
                "SELECT {COLS} FROM extension ORDER BY name COLLATE TITLE"
            ))
            .map_err(db_err)?;
        let rows = st.query_map([], row).map_err(db_err)?;
        rows.collect::<Result<_, _>>().map_err(db_err)
    }

    fn get_extension(&self, id: &str) -> DomainResult<Option<Extension>> {
        self.conn
            .query_row(
                &format!("SELECT {COLS} FROM extension WHERE id = ?1"),
                [id],
                row,
            )
            .optional()
            .map_err(db_err)
    }

    fn save_extension(&self, e: &Extension) -> DomainResult<()> {
        self.conn
            .execute(
                "INSERT INTO extension (id, name, version, type, install_path, enabled, status,
                    last_error, permissions, settings)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                 ON CONFLICT (id) DO UPDATE SET name = excluded.name, version = excluded.version,
                    type = excluded.type, install_path = excluded.install_path,
                    enabled = excluded.enabled, status = excluded.status,
                    last_error = excluded.last_error, permissions = excluded.permissions,
                    settings = excluded.settings",
                params![
                    e.id,
                    e.name,
                    e.version,
                    e.kind.as_str(),
                    e.install_path,
                    e.enabled,
                    e.status.as_str(),
                    e.last_error,
                    json::to_column(&e.permissions)?,
                    json::to_column(&e.settings)?
                ],
            )
            .map_err(db_err)?;
        Ok(())
    }

    fn set_extension_status(
        &self,
        id: &str,
        status: ExtensionStatus,
        error: Option<&str>,
    ) -> DomainResult<()> {
        let n = self
            .conn
            .execute(
                "UPDATE extension SET status = ?2, last_error = ?3 WHERE id = ?1",
                params![id, status.as_str(), error],
            )
            .map_err(db_err)?;
        if n == 0 {
            return Err(DomainError::NotFound {
                entity: "extension",
            });
        }
        Ok(())
    }

    fn delete_extension(&self, id: &str) -> DomainResult<()> {
        let n = self
            .conn
            .execute("DELETE FROM extension WHERE id = ?1", [id])
            .map_err(db_err)?;
        if n == 0 {
            return Err(DomainError::NotFound {
                entity: "extension",
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use epikodi_domain::extension::{ExtensionType, Permission};

    #[test]
    fn enregistrer_et_relire() {
        let s = Store::in_memory();
        let e = Extension {
            id: "weather".into(),
            name: "Météo".into(),
            version: "1.0.0".into(),
            kind: ExtensionType::Widget,
            install_path: "/x".into(),
            enabled: true,
            status: ExtensionStatus::Ok,
            last_error: None,
            permissions: Grants::from_acceptance(&[Permission::Network], &[Permission::Network])
                .unwrap(),
            settings: serde_json::from_str(r#"{"city":"Paris"}"#).unwrap(),
        };
        s.save_extension(&e).unwrap();
        assert_eq!(s.get_extension("weather").unwrap().unwrap(), e);
        s.set_extension_status("weather", ExtensionStatus::Crashed, Some("boucle infinie"))
            .unwrap();
        let back = s.get_extension("weather").unwrap().unwrap();
        assert_eq!(back.status, ExtensionStatus::Crashed);
        assert_eq!(back.last_error.as_deref(), Some("boucle infinie"));
        s.delete_extension("weather").unwrap();
        assert!(s.list_extensions().unwrap().is_empty());
    }
}
