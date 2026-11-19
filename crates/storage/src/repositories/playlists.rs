use rusqlite::{params, OptionalExtension, Row};

use epikodi_domain::playlist::{Playlist, PlaylistItem};
use epikodi_domain::ports::PlaylistRepo;
use epikodi_domain::{DomainError, DomainResult, Id};

use super::{db_err, unique_err, Store};

const SELECT: &str = "SELECT p.id, p.user_id, p.name,
    (SELECT COUNT(*) FROM playlist_item i WHERE i.playlist_id = p.id) FROM playlist p";

fn row(r: &Row) -> rusqlite::Result<Playlist> {
    Ok(Playlist {
        id: r.get(0)?,
        user_id: r.get(1)?,
        name: r.get(2)?,
        item_count: r.get(3)?,
    })
}

impl Store {
    fn rewrite_playlist(&self, id: Id, media_ids: &[Id]) -> DomainResult<()> {
        self.transaction(|tx| {
            tx.execute("DELETE FROM playlist_item WHERE playlist_id = ?1", [id])
                .map_err(db_err)?;
            let mut st = tx
                .prepare("INSERT INTO playlist_item (playlist_id, position, media_id) VALUES (?1, ?2, ?3)")
                .map_err(db_err)?;
            for (pos, mid) in media_ids.iter().enumerate() {
                st.execute(params![id, pos as i64, mid]).map_err(db_err)?;
            }
            Ok(())
        })
    }
}

impl PlaylistRepo for Store {
    fn list_playlists(&self, user_id: Id) -> DomainResult<Vec<Playlist>> {
        let mut st = self
            .conn
            .prepare(&format!(
                "{SELECT} WHERE p.user_id = ?1 ORDER BY p.name COLLATE TITLE"
            ))
            .map_err(db_err)?;
        let rows = st.query_map([user_id], row).map_err(db_err)?;
        rows.collect::<Result<_, _>>().map_err(db_err)
    }

    fn get_playlist(&self, user_id: Id, id: Id) -> DomainResult<Option<Playlist>> {
        self.conn
            .query_row(
                &format!("{SELECT} WHERE p.user_id = ?1 AND p.id = ?2"),
                params![user_id, id],
                row,
            )
            .optional()
            .map_err(db_err)
    }

    fn create_playlist(&self, user_id: Id, name: &str) -> DomainResult<Playlist> {
        self.conn
            .execute(
                "INSERT INTO playlist (user_id, name) VALUES (?1, ?2)",
                params![user_id, name],
            )
            .map_err(unique_err("liste de lecture"))?;
        let id = self.conn.last_insert_rowid();
        self.get_playlist(user_id, id)?
            .ok_or(DomainError::NotFound {
                entity: "liste de lecture",
            })
    }

    fn rename_playlist(&self, id: Id, name: &str) -> DomainResult<()> {
        self.conn
            .execute(
                "UPDATE playlist SET name = ?2 WHERE id = ?1",
                params![id, name],
            )
            .map_err(unique_err("liste de lecture"))?;
        Ok(())
    }

    fn delete_playlist(&self, id: Id) -> DomainResult<()> {
        self.conn
            .execute("DELETE FROM playlist WHERE id = ?1", [id])
            .map_err(db_err)?;
        Ok(())
    }

    fn playlist_items(&self, id: Id) -> DomainResult<Vec<PlaylistItem>> {
        let mut st = self
            .conn
            .prepare("SELECT position, media_id FROM playlist_item WHERE playlist_id = ?1 ORDER BY position")
            .map_err(db_err)?;
        let rows = st
            .query_map([id], |r| {
                Ok(PlaylistItem {
                    position: r.get(0)?,
                    media_id: r.get(1)?,
                })
            })
            .map_err(db_err)?;
        rows.collect::<Result<_, _>>().map_err(db_err)
    }

    fn add_playlist_items(&self, id: Id, media_ids: &[Id]) -> DomainResult<()> {
        let mut all: Vec<Id> = self
            .playlist_items(id)?
            .into_iter()
            .map(|i| i.media_id)
            .collect();
        all.extend_from_slice(media_ids);
        self.rewrite_playlist(id, &all)
    }

    fn remove_playlist_item(&self, id: Id, position: i64) -> DomainResult<bool> {
        let items = self.playlist_items(id)?;
        let before = items.len();
        let kept: Vec<Id> = items
            .into_iter()
            .filter(|i| i.position != position)
            .map(|i| i.media_id)
            .collect();
        if kept.len() == before {
            return Ok(false);
        }
        self.rewrite_playlist(id, &kept)?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::media::insert_test_media;
    use epikodi_domain::media::MediaKind;
    use epikodi_domain::ports::ProfileRepo;
    use epikodi_domain::profile::NewProfile;

    #[test]
    fn cycle_de_vie_d_une_liste() {
        let s = Store::in_memory();
        let u = s
            .create_profile(&NewProfile {
                name: "Marc".into(),
                icon: None,
                language: "fr".into(),
                age: 30,
                is_restricted: false,
                pin_hash: None,
            })
            .unwrap()
            .id;
        let a = insert_test_media(&s, None, MediaKind::Movie, "A", Some(0));
        let b = insert_test_media(&s, None, MediaKind::Movie, "B", Some(0));
        let p = s.create_playlist(u, "Soirée").unwrap();
        assert!(matches!(
            s.create_playlist(u, "Soirée"),
            Err(DomainError::Conflict { .. })
        ));
        s.add_playlist_items(p.id, &[a, b, a]).unwrap();
        assert_eq!(s.get_playlist(u, p.id).unwrap().unwrap().item_count, 3);
        assert!(s.remove_playlist_item(p.id, 1).unwrap());
        let ids: Vec<_> = s
            .playlist_items(p.id)
            .unwrap()
            .into_iter()
            .map(|i| (i.position, i.media_id))
            .collect();
        assert_eq!(ids, [(0, a), (1, a)], "positions renumérotées");
        assert!(!s.remove_playlist_item(p.id, 9).unwrap());
        s.delete_playlist(p.id).unwrap();
        assert!(s.get_playlist(u, p.id).unwrap().is_none());
    }
}
