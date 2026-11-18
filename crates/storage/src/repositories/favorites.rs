use rusqlite::params;

use epikodi_domain::media::Media;
use epikodi_domain::parental::Visibility;
use epikodi_domain::ports::FavoriteRepo;
use epikodi_domain::{DomainResult, Id};

use super::media::{row, visible, COLS, FROM};
use super::{db_err, Store};

impl FavoriteRepo for Store {
    fn is_favorite(&self, user_id: Id, media_id: Id) -> DomainResult<bool> {
        self.conn
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM favorite WHERE user_id = ?1 AND media_id = ?2)",
                params![user_id, media_id],
                |r| r.get(0),
            )
            .map_err(db_err)
    }

    fn add_favorite(&self, user_id: Id, media_id: Id) -> DomainResult<()> {
        self.conn
            .execute(
                "INSERT OR IGNORE INTO favorite (user_id, media_id) VALUES (?1, ?2)",
                params![user_id, media_id],
            )
            .map_err(db_err)?;
        Ok(())
    }

    fn remove_favorite(&self, user_id: Id, media_id: Id) -> DomainResult<()> {
        self.conn
            .execute(
                "DELETE FROM favorite WHERE user_id = ?1 AND media_id = ?2",
                params![user_id, media_id],
            )
            .map_err(db_err)?;
        Ok(())
    }

    fn favorites(&self, user_id: Id, vis: Visibility) -> DomainResult<Vec<Media>> {
        let mut st = self
            .conn
            .prepare(&format!(
                "SELECT {COLS} {FROM} JOIN favorite f ON f.media_id = m.id
                 WHERE f.user_id = ?1 AND {} ORDER BY m.title COLLATE TITLE",
                visible(vis)
            ))
            .map_err(db_err)?;
        let rows = st.query_map([user_id], row).map_err(db_err)?;
        rows.collect::<Result<_, _>>().map_err(db_err)
    }
}
