use rusqlite::{params, OptionalExtension, Row};

use epikodi_domain::ports::ProfileRepo;
use epikodi_domain::profile::{NewProfile, Preferences, Profile};
use epikodi_domain::{DomainError, DomainResult, Id};

use super::{db_err, unique_err, Store};
use crate::json;

const COLS: &str = "id, name, icon, language, age, is_restricted, pin_hash, preferences";

fn row(r: &Row) -> rusqlite::Result<Profile> {
    Ok(Profile {
        id: r.get(0)?,
        name: r.get(1)?,
        icon: r.get(2)?,
        language: r.get(3)?,
        age: r.get(4)?,
        is_restricted: r.get(5)?,
        pin_hash: r.get(6)?,
        preferences: json::from_column(&r.get::<_, String>(7)?, "user.preferences"),
    })
}

impl ProfileRepo for Store {
    fn list_profiles(&self) -> DomainResult<Vec<Profile>> {
        let mut st = self
            .conn
            .prepare(&format!(
                "SELECT {COLS} FROM user ORDER BY name COLLATE NOCASE"
            ))
            .map_err(db_err)?;
        let rows = st.query_map([], row).map_err(db_err)?;
        rows.collect::<Result<_, _>>().map_err(db_err)
    }

    fn get_profile(&self, id: Id) -> DomainResult<Option<Profile>> {
        self.conn
            .query_row(&format!("SELECT {COLS} FROM user WHERE id = ?1"), [id], row)
            .optional()
            .map_err(db_err)
    }

    fn create_profile(&self, p: &NewProfile) -> DomainResult<Profile> {
        // Nom unique sans tenir compte de la casse (« Léa » et « léa »).
        let taken: bool = self
            .conn
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM user WHERE name = ?1 COLLATE NOCASE)",
                [&p.name],
                |r| r.get(0),
            )
            .map_err(db_err)?;
        if taken {
            return Err(DomainError::Conflict { entity: "profil" });
        }
        self.conn
            .execute(
                "INSERT INTO user (name, icon, language, age, is_restricted, pin_hash)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    p.name,
                    p.icon,
                    p.language,
                    p.age,
                    p.is_restricted,
                    p.pin_hash
                ],
            )
            .map_err(unique_err("profil"))?;
        let id = self.conn.last_insert_rowid();
        self.get_profile(id)?
            .ok_or(DomainError::NotFound { entity: "profil" })
    }

    fn update_profile(&self, p: &Profile) -> DomainResult<()> {
        let taken: bool = self
            .conn
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM user WHERE name = ?1 COLLATE NOCASE AND id <> ?2)",
                params![p.name, p.id],
                |r| r.get(0),
            )
            .map_err(db_err)?;
        if taken {
            return Err(DomainError::Conflict { entity: "profil" });
        }
        let n = self
            .conn
            .execute(
                "UPDATE user SET name = ?2, icon = ?3, language = ?4, age = ?5,
                        is_restricted = ?6, pin_hash = ?7, preferences = ?8
                 WHERE id = ?1",
                params![
                    p.id,
                    p.name,
                    p.icon,
                    p.language,
                    p.age,
                    p.is_restricted,
                    p.pin_hash,
                    json::to_column(&p.preferences)?
                ],
            )
            .map_err(unique_err("profil"))?;
        if n == 0 {
            return Err(DomainError::NotFound { entity: "profil" });
        }
        Ok(())
    }

    fn delete_profile(&self, id: Id) -> DomainResult<()> {
        let n = self
            .conn
            .execute("DELETE FROM user WHERE id = ?1", [id])
            .map_err(db_err)?;
        if n == 0 {
            return Err(DomainError::NotFound { entity: "profil" });
        }
        Ok(())
    }

    fn set_preferences(&self, id: Id, prefs: &Preferences) -> DomainResult<()> {
        let n = self
            .conn
            .execute(
                "UPDATE user SET preferences = ?2 WHERE id = ?1",
                params![id, json::to_column(prefs)?],
            )
            .map_err(db_err)?;
        if n == 0 {
            return Err(DomainError::NotFound { entity: "profil" });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn new_profile(name: &str, restricted: bool) -> NewProfile {
        NewProfile {
            name: name.into(),
            icon: Some("avatar_01".into()),
            language: "fr".into(),
            age: if restricted { 10 } else { 35 },
            is_restricted: restricted,
            pin_hash: (!restricted).then(|| "hash".to_string()),
        }
    }

    #[test]
    fn creer_lire_modifier_supprimer() {
        let s = Store::in_memory();
        let p = s.create_profile(&new_profile("Marc", false)).unwrap();
        assert_eq!(p.name, "Marc");
        assert_eq!(p.preferences, Preferences::default());

        let mut p2 = p.clone();
        p2.age = 36;
        p2.preferences.theme = "alternative".into();
        s.update_profile(&p2).unwrap();
        assert_eq!(s.get_profile(p.id).unwrap().unwrap().age, 36);
        assert_eq!(
            s.get_profile(p.id).unwrap().unwrap().preferences.theme,
            "alternative"
        );

        s.delete_profile(p.id).unwrap();
        assert!(s.get_profile(p.id).unwrap().is_none());
        assert!(matches!(
            s.delete_profile(p.id),
            Err(DomainError::NotFound { .. })
        ));
    }

    #[test]
    fn nom_unique_sans_casse() {
        let s = Store::in_memory();
        s.create_profile(&new_profile("Léa", true)).unwrap();
        assert!(matches!(
            s.create_profile(&new_profile("Léa", true)),
            Err(DomainError::Conflict { .. })
        ));
        assert!(matches!(
            s.create_profile(&new_profile("léa", true)),
            Err(DomainError::Conflict { .. })
        ));
        let marc = s.create_profile(&new_profile("Marc", false)).unwrap();
        let mut renamed = marc.clone();
        renamed.name = "Léa".into();
        assert!(matches!(
            s.update_profile(&renamed),
            Err(DomainError::Conflict { .. })
        ));
    }

    #[test]
    fn liste_triee_par_nom() {
        let s = Store::in_memory();
        s.create_profile(&new_profile("zoé", true)).unwrap();
        s.create_profile(&new_profile("Marc", false)).unwrap();
        let names: Vec<_> = s
            .list_profiles()
            .unwrap()
            .into_iter()
            .map(|p| p.name)
            .collect();
        assert_eq!(names, ["Marc", "zoé"]);
    }

    #[test]
    fn supprimer_un_profil_supprime_ses_donnees() {
        let s = Store::in_memory();
        let p = s.create_profile(&new_profile("Marc", false)).unwrap();
        s.conn
            .execute_batch(&format!(
                "INSERT INTO playlist (user_id, name) VALUES ({}, 'x');",
                p.id
            ))
            .unwrap();
        s.delete_profile(p.id).unwrap();
        let n: i64 = s
            .conn
            .query_row("SELECT COUNT(*) FROM playlist", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0, "ON DELETE CASCADE");
    }
}
