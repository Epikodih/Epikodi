use rusqlite::{params, OptionalExtension, Row};

use epikodi_domain::media::Media;
use epikodi_domain::parental::Visibility;
use epikodi_domain::playback::WatchProgress;
use epikodi_domain::ports::ProgressRepo;
use epikodi_domain::{DomainResult, Id};

use super::media::{row as media_row, visible, COLS, FROM};
use super::{db_err, iso_opt, Store};

fn row(r: &Row, offset: usize) -> rusqlite::Result<WatchProgress> {
    Ok(WatchProgress {
        media_id: r.get(offset)?,
        position_s: r.get(offset + 1)?,
        is_watched: r.get(offset + 2)?,
        last_played_at: iso_opt(r.get(offset + 3)?),
    })
}

impl ProgressRepo for Store {
    fn get_progress(&self, user_id: Id, media_id: Id) -> DomainResult<Option<WatchProgress>> {
        self.conn
            .query_row(
                "SELECT media_id, position_s, is_watched, last_played_at FROM watch_progress
                 WHERE user_id = ?1 AND media_id = ?2",
                params![user_id, media_id],
                |r| row(r, 0),
            )
            .optional()
            .map_err(db_err)
    }

    fn save_progress(&self, user_id: Id, p: &WatchProgress) -> DomainResult<()> {
        self.conn
            .execute(
                "INSERT INTO watch_progress (user_id, media_id, position_s, is_watched, last_played_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT (user_id, media_id) DO UPDATE SET position_s = excluded.position_s,
                    is_watched = excluded.is_watched, last_played_at = excluded.last_played_at",
                params![user_id, p.media_id, p.position_s, p.is_watched, p.last_played_at],
            )
            .map_err(db_err)?;
        Ok(())
    }

    fn set_watched(
        &self,
        user_id: Id,
        media_ids: &[Id],
        watched: bool,
        at: &str,
    ) -> DomainResult<()> {
        self.transaction(|tx| {
            let mut st = tx
                .prepare(
                    "INSERT INTO watch_progress (user_id, media_id, position_s, is_watched, last_played_at)
                     VALUES (?1, ?2, 0, ?3, ?4)
                     ON CONFLICT (user_id, media_id) DO UPDATE SET position_s = 0,
                        is_watched = excluded.is_watched,
                        last_played_at = COALESCE(watch_progress.last_played_at, excluded.last_played_at)",
                )
                .map_err(db_err)?;
            for id in media_ids {
                st.execute(params![user_id, id, watched, at]).map_err(db_err)?;
            }
            Ok(())
        })
    }

    fn in_progress(
        &self,
        user_id: Id,
        vis: Visibility,
        limit: u32,
    ) -> DomainResult<Vec<(Media, WatchProgress)>> {
        let mut st = self
            .conn
            .prepare(&format!(
                "SELECT {COLS}, w.media_id, w.position_s, w.is_watched, w.last_played_at
                 {FROM} JOIN watch_progress w ON w.media_id = m.id
                 WHERE w.user_id = ?1 AND w.is_watched = 0 AND w.position_s > 0 AND {}
                 ORDER BY w.last_played_at DESC LIMIT ?2",
                visible(vis)
            ))
            .map_err(db_err)?;
        let rows = st
            .query_map(params![user_id, limit], |r| {
                Ok((media_row(r)?, row(r, 18)?))
            })
            .map_err(db_err)?;
        rows.collect::<Result<_, _>>().map_err(db_err)
    }

    fn progress_for(&self, user_id: Id, media_ids: &[Id]) -> DomainResult<Vec<WatchProgress>> {
        let mut out = Vec::new();
        for id in media_ids {
            if let Some(p) = self.get_progress(user_id, *id)? {
                out.push(p);
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::media::insert_test_media;
    use epikodi_domain::media::MediaKind;
    use epikodi_domain::ports::ProfileRepo;
    use epikodi_domain::profile::NewProfile;

    fn user(s: &Store, name: &str) -> Id {
        s.create_profile(&NewProfile {
            name: name.into(),
            icon: None,
            language: "fr".into(),
            age: 30,
            is_restricted: false,
            pin_hash: None,
        })
        .unwrap()
        .id
    }

    #[test]
    fn reprise_synchronisee_par_profil() {
        let s = Store::in_memory();
        let marc = user(&s, "Marc");
        let lea = user(&s, "Léa");
        let film = insert_test_media(&s, None, MediaKind::Movie, "Sintel", Some(0));
        s.save_progress(
            marc,
            &WatchProgress {
                media_id: film,
                position_s: 300,
                is_watched: false,
                last_played_at: Some("2026-11-01T10:00:00Z".into()),
            },
        )
        .unwrap();
        assert_eq!(s.get_progress(marc, film).unwrap().unwrap().position_s, 300);
        assert!(
            s.get_progress(lea, film).unwrap().is_none(),
            "chaque profil a sa progression"
        );
        let list = s.in_progress(marc, Visibility::All, 10).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].0.id, film);
        assert!(
            s.in_progress(marc, Visibility::MaxAge(0), 10)
                .unwrap()
                .len()
                == 1
        );

        s.set_watched(marc, &[film], true, "2026-11-02T10:00:00Z")
            .unwrap();
        let p = s.get_progress(marc, film).unwrap().unwrap();
        assert!(p.is_watched);
        assert_eq!(p.position_s, 0);
        assert!(s.in_progress(marc, Visibility::All, 10).unwrap().is_empty());
    }
}
