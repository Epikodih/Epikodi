use rusqlite::{params, OptionalExtension, Row};

use epikodi_domain::device::{Device, NewDevice};
use epikodi_domain::ports::DeviceRepo;
use epikodi_domain::{DomainError, DomainResult, Id};

use super::{db_err, iso, iso_opt, unique_err, Store};

const COLS: &str =
    "id, name, install_id, platform, paired_at, expires_at, last_seen_at, revoked_at";

fn row(r: &Row) -> rusqlite::Result<Device> {
    Ok(Device {
        id: r.get(0)?,
        name: r.get(1)?,
        install_id: r.get(2)?,
        platform: r.get(3)?,
        paired_at: iso(r.get(4)?),
        expires_at: iso_opt(r.get(5)?),
        last_seen_at: iso_opt(r.get(6)?),
        revoked_at: iso_opt(r.get(7)?),
    })
}

impl DeviceRepo for Store {
    fn create_device(&self, d: &NewDevice) -> DomainResult<Device> {
        self.conn
            .execute(
                "INSERT INTO device (name, install_id, platform, token_hash, paired_at, expires_at, last_seen_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?5)",
                params![d.name, d.install_id, d.platform, d.token_hash, crate::now_iso(), d.expires_at],
            )
            .map_err(unique_err("appareil"))?;
        let id = self.conn.last_insert_rowid();
        self.get_device(id)?
            .ok_or(DomainError::NotFound { entity: "appareil" })
    }

    fn find_device_by_token(&self, token_hash: &str) -> DomainResult<Option<Device>> {
        self.conn
            .query_row(
                &format!("SELECT {COLS} FROM device WHERE token_hash = ?1 AND revoked_at IS NULL"),
                [token_hash],
                row,
            )
            .optional()
            .map_err(db_err)
    }

    fn get_device(&self, id: Id) -> DomainResult<Option<Device>> {
        self.conn
            .query_row(
                &format!("SELECT {COLS} FROM device WHERE id = ?1"),
                [id],
                row,
            )
            .optional()
            .map_err(db_err)
    }

    fn list_devices(&self) -> DomainResult<Vec<Device>> {
        let mut st = self
            .conn
            .prepare(&format!(
                "SELECT {COLS} FROM device WHERE revoked_at IS NULL ORDER BY paired_at DESC, id DESC"
            ))
            .map_err(db_err)?;
        let rows = st.query_map([], row).map_err(db_err)?;
        rows.collect::<Result<_, _>>().map_err(db_err)
    }

    fn revoke_device(&self, id: Id, at: &str) -> DomainResult<()> {
        let n = self
            .conn
            .execute(
                "UPDATE device SET revoked_at = ?2 WHERE id = ?1 AND revoked_at IS NULL",
                params![id, at],
            )
            .map_err(db_err)?;
        if n == 0 {
            return Err(DomainError::NotFound { entity: "appareil" });
        }
        Ok(())
    }

    fn revoke_install(&self, install_id: &str, at: &str) -> DomainResult<()> {
        self.conn
            .execute(
                "UPDATE device SET revoked_at = ?2 WHERE install_id = ?1 AND revoked_at IS NULL",
                params![install_id, at],
            )
            .map_err(db_err)?;
        Ok(())
    }

    fn touch_device(&self, id: Id, at: &str) -> DomainResult<()> {
        self.conn
            .execute(
                "UPDATE device SET last_seen_at = ?2 WHERE id = ?1",
                params![id, at],
            )
            .map_err(db_err)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_device(token: &str, install: &str) -> NewDevice {
        NewDevice {
            name: "Pixel 7".into(),
            install_id: Some(install.into()),
            platform: Some("android".into()),
            token_hash: token.into(),
            expires_at: Some("2099-01-01T00:00:00Z".into()),
        }
    }

    #[test]
    fn appairage_et_revocation() {
        let s = Store::in_memory();
        let d = s.create_device(&new_device("h1", "i1")).unwrap();
        assert!(d.paired_at.ends_with('Z'));
        assert_eq!(s.find_device_by_token("h1").unwrap().unwrap().id, d.id);
        s.revoke_device(d.id, "2026-01-01T00:00:00Z").unwrap();
        assert!(
            s.find_device_by_token("h1").unwrap().is_none(),
            "jeton révoqué = refusé"
        );
        assert!(s.list_devices().unwrap().is_empty());
    }

    #[test]
    fn re_appairage_revoque_l_ancien_jeton() {
        let s = Store::in_memory();
        s.create_device(&new_device("h1", "i1")).unwrap();
        s.revoke_install("i1", "2026-01-01T00:00:00Z").unwrap();
        s.create_device(&new_device("h2", "i1")).unwrap();
        assert!(s.find_device_by_token("h1").unwrap().is_none());
        assert!(s.find_device_by_token("h2").unwrap().is_some());
        assert_eq!(s.list_devices().unwrap().len(), 1);
    }
}
