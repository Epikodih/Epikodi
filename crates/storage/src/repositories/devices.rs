use rusqlite::{params, OptionalExtension, Row};

use epikodi_domain::device::{Device, NewDevice};
use epikodi_domain::ports::DeviceRepo;
use epikodi_domain::{DomainError, DomainResult, Id};

use super::{db_err, iso, iso_opt, unique_err, Store};

/// Jeton encore valide à `?now` : pas révoqué, et sans date d'expiration ou
/// expirant plus tard. Les dates sont comparées par `julianday` (pas comme
/// du texte) ; une date illisible donne NULL, donc un jeton refusé.
const VALID_AT: &str = "revoked_at IS NULL \
    AND (expires_at IS NULL OR julianday(expires_at) > julianday(:now))";

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
                params![d.name, d.install_id, d.platform, d.token_hash, d.paired_at, d.expires_at],
            )
            .map_err(unique_err("appareil"))?;
        let id = self.conn.last_insert_rowid();
        self.get_device(id)?
            .ok_or(DomainError::NotFound { entity: "appareil" })
    }

    fn find_device_by_token(&self, token_hash: &str, now: &str) -> DomainResult<Option<Device>> {
        self.conn
            .query_row(
                &format!("SELECT {COLS} FROM device WHERE token_hash = :hash AND {VALID_AT}"),
                rusqlite::named_params! { ":hash": token_hash, ":now": now },
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

    fn device_valid(&self, id: Id, now: &str) -> DomainResult<bool> {
        self.conn
            .query_row(
                &format!("SELECT EXISTS (SELECT 1 FROM device WHERE id = :id AND {VALID_AT})"),
                rusqlite::named_params! { ":id": id, ":now": now },
                |r| r.get(0),
            )
            .map_err(db_err)
    }

    fn list_devices(&self, now: &str) -> DomainResult<Vec<Device>> {
        let mut st = self
            .conn
            .prepare(&format!(
                "SELECT {COLS} FROM device WHERE {VALID_AT} ORDER BY paired_at DESC, id DESC"
            ))
            .map_err(db_err)?;
        let rows = st
            .query_map(rusqlite::named_params! { ":now": now }, row)
            .map_err(db_err)?;
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

    const NOW: &str = "2026-11-07T18:30:00Z";

    fn new_device(token: &str, install: &str, expires: Option<&str>) -> NewDevice {
        NewDevice {
            name: "Pixel 7".into(),
            install_id: Some(install.into()),
            platform: Some("android".into()),
            token_hash: token.into(),
            paired_at: NOW.into(),
            expires_at: expires.map(Into::into),
        }
    }

    #[test]
    fn appairage_et_revocation() {
        let s = Store::in_memory();
        let d = s
            .create_device(&new_device("h1", "i1", Some("2099-01-01T00:00:00Z")))
            .unwrap();
        assert_eq!(d.paired_at, NOW);
        assert_eq!(s.find_device_by_token("h1", NOW).unwrap().unwrap().id, d.id);
        s.revoke_device(d.id, NOW).unwrap();
        assert!(
            s.find_device_by_token("h1", NOW).unwrap().is_none(),
            "jeton révoqué = refusé"
        );
        assert!(s.list_devices(NOW).unwrap().is_empty());
    }

    /// Régression : un jeton expiré était renvoyé comme valide par le dépôt.
    #[test]
    fn jeton_expire_refuse() {
        let s = Store::in_memory();
        s.create_device(&new_device("h1", "i1", Some("2026-12-07T18:30:00Z")))
            .unwrap();
        // une seconde avant l'expiration : valide
        assert!(s
            .find_device_by_token("h1", "2026-12-07T18:29:59Z")
            .unwrap()
            .is_some());
        // à l'instant de l'expiration, et après : refusé
        assert!(s
            .find_device_by_token("h1", "2026-12-07T18:30:00Z")
            .unwrap()
            .is_none());
        assert!(s
            .find_device_by_token("h1", "2027-01-01T00:00:00Z")
            .unwrap()
            .is_none());
        assert!(s.list_devices("2027-01-01T00:00:00Z").unwrap().is_empty());
        let id = s.list_devices(NOW).unwrap()[0].id;
        assert!(s.device_valid(id, NOW).unwrap());
        assert!(!s.device_valid(id, "2027-01-01T00:00:00Z").unwrap());
        assert_eq!(s.list_devices(NOW).unwrap().len(), 1);
    }

    #[test]
    fn date_d_expiration_illisible_refusee() {
        let s = Store::in_memory();
        s.create_device(&new_device("h1", "i1", Some("pas une date")))
            .unwrap();
        assert!(s.find_device_by_token("h1", NOW).unwrap().is_none());
    }

    #[test]
    fn sans_expiration_reste_valide() {
        let s = Store::in_memory();
        s.create_device(&new_device("h1", "i1", None)).unwrap();
        assert!(s
            .find_device_by_token("h1", "2099-01-01T00:00:00Z")
            .unwrap()
            .is_some());
    }

    #[test]
    fn re_appairage_revoque_l_ancien_jeton() {
        let s = Store::in_memory();
        let exp = Some("2099-01-01T00:00:00Z");
        s.create_device(&new_device("h1", "i1", exp)).unwrap();
        s.revoke_install("i1", NOW).unwrap();
        s.create_device(&new_device("h2", "i1", exp)).unwrap();
        assert!(s.find_device_by_token("h1", NOW).unwrap().is_none());
        assert!(s.find_device_by_token("h2", NOW).unwrap().is_some());
        assert_eq!(s.list_devices(NOW).unwrap().len(), 1);
    }
}
