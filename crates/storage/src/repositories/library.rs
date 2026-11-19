//! Opérations du scan (identité d'un fichier = `media.uri`).

use rusqlite::{params, OptionalExtension};

use epikodi_domain::media::{check_hierarchy, Media, MediaKind, MediaStatus, NewMedia};
use epikodi_domain::ports::{LibraryRepo, MediaRepo};
use epikodi_domain::source::SourceStatus;
use epikodi_domain::{DomainError, DomainResult, Id};

use super::media::{row, COLS};
use super::{db_err, unique_err, Store};

impl LibraryRepo for Store {
    fn known_files(&self, source_id: Id) -> DomainResult<Vec<Media>> {
        let mut st = self
            .conn
            .prepare(&format!(
                "SELECT {COLS} FROM media m WHERE m.source_id = ?1 AND m.uri IS NOT NULL"
            ))
            .map_err(db_err)?;
        let rows = st.query_map([source_id], row).map_err(db_err)?;
        rows.collect::<Result<_, _>>().map_err(db_err)
    }

    fn find_by_uri(&self, uri: &str) -> DomainResult<Option<Media>> {
        self.conn
            .query_row(
                &format!("SELECT {COLS} FROM media m WHERE m.uri = ?1"),
                [uri],
                row,
            )
            .optional()
            .map_err(db_err)
    }

    fn find_missing_by_fingerprint(&self, fingerprint: &str) -> DomainResult<Option<Media>> {
        self.conn
            .query_row(
                &format!(
                    "SELECT {COLS} FROM media m WHERE m.fingerprint = ?1 AND m.status = 'missing' \
                     ORDER BY m.id LIMIT 1"
                ),
                [fingerprint],
                row,
            )
            .optional()
            .map_err(db_err)
    }

    fn insert_media(&self, m: &NewMedia) -> DomainResult<Id> {
        let kind = m
            .kind
            .ok_or_else(|| DomainError::validation("kind", "obligatoire"))?;
        let parent_kind = match m.parent_id {
            Some(pid) => Some(
                self.get_media(pid)?
                    .ok_or(DomainError::NotFound { entity: "parent" })?
                    .kind,
            ),
            None => None,
        };
        check_hierarchy(kind, parent_kind)?;
        if kind.is_leaf() && m.uri.is_none() {
            return Err(DomainError::validation(
                "uri",
                "obligatoire pour un média lisible",
            ));
        }
        let now = crate::now_iso();
        self.conn
            .execute(
                "INSERT INTO media (parent_id, kind, title, year, duration_s, sort_number, age_rating,
                    match_status, source_id, uri, parent_dir, size_bytes, modified_at, fingerprint,
                    status, last_seen_at, added_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, 'available', ?15, ?15)",
                params![
                    m.parent_id,
                    kind.as_str(),
                    m.title,
                    m.year,
                    m.duration_s,
                    m.sort_number,
                    m.age_rating,
                    m.match_status.unwrap_or(epikodi_domain::media::MatchStatus::Unmatched).as_str(),
                    m.source_id,
                    m.uri,
                    m.parent_dir,
                    m.size_bytes,
                    m.modified_at,
                    m.fingerprint,
                    now,
                ],
            )
            .map_err(unique_err("média"))?;
        Ok(self.conn.last_insert_rowid())
    }

    fn update_file(&self, id: Id, m: &NewMedia) -> DomainResult<()> {
        let n = self
            .conn
            .execute(
                "UPDATE media SET uri = ?2, parent_dir = ?3, size_bytes = ?4, modified_at = ?5,
                    fingerprint = ?6, source_id = COALESCE(?7, source_id),
                    duration_s = COALESCE(?8, duration_s),
                    status = 'available', last_seen_at = ?9
                 WHERE id = ?1",
                params![
                    id,
                    m.uri,
                    m.parent_dir,
                    m.size_bytes,
                    m.modified_at,
                    m.fingerprint,
                    m.source_id,
                    m.duration_s,
                    crate::now_iso()
                ],
            )
            .map_err(unique_err("média"))?;
        if n == 0 {
            return Err(DomainError::NotFound { entity: "média" });
        }
        Ok(())
    }

    fn set_media_status(&self, id: Id, status: MediaStatus) -> DomainResult<()> {
        self.conn
            .execute(
                "UPDATE media SET status = ?2 WHERE id = ?1",
                params![id, status.as_str()],
            )
            .map_err(db_err)?;
        Ok(())
    }

    fn find_container(
        &self,
        kind: MediaKind,
        parent_id: Option<Id>,
        title: &str,
        sort_number: Option<i64>,
    ) -> DomainResult<Option<Id>> {
        // Une saison se retrouve par son numéro, le reste par son titre.
        let found = match sort_number {
            Some(n) => self.conn.query_row(
                "SELECT id FROM media WHERE kind = ?1 AND parent_id IS ?2 AND sort_number = ?3 LIMIT 1",
                params![kind.as_str(), parent_id, n],
                |r| r.get(0),
            ),
            None => self.conn.query_row(
                "SELECT id FROM media WHERE kind = ?1 AND parent_id IS ?2 AND fold(title) = fold(?3) LIMIT 1",
                params![kind.as_str(), parent_id, title],
                |r| r.get(0),
            ),
        };
        found.optional().map_err(db_err)
    }

    fn unmatched(&self, limit: u32) -> DomainResult<Vec<Media>> {
        let mut st = self
            .conn
            .prepare(&format!(
                "SELECT {COLS} FROM media m WHERE m.match_status = 'unmatched' \
                 AND m.kind IN ('movie', 'series', 'artist', 'album') ORDER BY m.id LIMIT ?1"
            ))
            .map_err(db_err)?;
        let rows = st.query_map([limit], row).map_err(db_err)?;
        rows.collect::<Result<_, _>>().map_err(db_err)
    }

    fn atomically(&self, f: &mut dyn FnMut() -> DomainResult<()>) -> DomainResult<()> {
        self.transaction(|_| f())
    }

    fn set_source_scanned(
        &self,
        source_id: Id,
        status: SourceStatus,
        at: &str,
    ) -> DomainResult<()> {
        self.conn
            .execute(
                "UPDATE source SET status = ?2, last_scan_at = ?3 WHERE id = ?1",
                params![source_id, status.as_str(), at],
            )
            .map_err(db_err)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(uri: &str, kind: MediaKind, parent: Option<Id>) -> NewMedia {
        NewMedia {
            kind: Some(kind),
            parent_id: parent,
            title: "x".into(),
            uri: Some(uri.into()),
            fingerprint: Some(format!("fp-{uri}")),
            ..Default::default()
        }
    }

    #[test]
    fn hierarchie_verifiee_a_l_insertion() {
        let s = Store::in_memory();
        assert!(s
            .insert_media(&file("file:///e.mkv", MediaKind::Episode, None))
            .is_err());
        let series = s
            .insert_media(&NewMedia {
                kind: Some(MediaKind::Series),
                title: "S".into(),
                ..Default::default()
            })
            .unwrap();
        assert!(s
            .insert_media(&file("file:///e.mkv", MediaKind::Episode, Some(series)))
            .is_err());
    }

    #[test]
    fn uri_unique() {
        let s = Store::in_memory();
        s.insert_media(&file("file:///a.mkv", MediaKind::Movie, None))
            .unwrap();
        assert!(matches!(
            s.insert_media(&file("file:///a.mkv", MediaKind::Movie, None)),
            Err(DomainError::Conflict { .. })
        ));
    }

    #[test]
    fn conteneur_retrouve_sans_casse_ni_accents() {
        let s = Store::in_memory();
        let id = s
            .insert_media(&NewMedia {
                kind: Some(MediaKind::Series),
                title: "Série Démo".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(
            s.find_container(MediaKind::Series, None, "serie demo", None)
                .unwrap(),
            Some(id)
        );
        assert_eq!(
            s.find_container(MediaKind::Artist, None, "serie demo", None)
                .unwrap(),
            None
        );
    }

    #[test]
    fn fichier_deplace_retrouve_par_empreinte() {
        let s = Store::in_memory();
        let id = s
            .insert_media(&file("file:///a.mkv", MediaKind::Movie, None))
            .unwrap();
        assert!(s
            .find_missing_by_fingerprint("fp-file:///a.mkv")
            .unwrap()
            .is_none());
        s.set_media_status(id, MediaStatus::Missing).unwrap();
        assert_eq!(
            s.find_missing_by_fingerprint("fp-file:///a.mkv")
                .unwrap()
                .unwrap()
                .id,
            id
        );
        let mut moved = file("file:///b.mkv", MediaKind::Movie, None);
        moved.fingerprint = Some("fp-file:///a.mkv".into());
        s.update_file(id, &moved).unwrap();
        let m = s.get_media(id).unwrap().unwrap();
        assert_eq!(m.uri.as_deref(), Some("file:///b.mkv"));
        assert_eq!(m.status, MediaStatus::Available);
    }
}
