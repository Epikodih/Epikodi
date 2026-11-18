use rusqlite::params;

use epikodi_domain::ports::QueueRepo;
use epikodi_domain::queue::{QueueItem, QueueMode};
use epikodi_domain::{DomainResult, Id};

use super::{db_err, Store};

impl QueueRepo for Store {
    fn queue(&self) -> DomainResult<Vec<QueueItem>> {
        let mut st = self
            .conn
            .prepare("SELECT id, media_id, position FROM queue_item ORDER BY position, id")
            .map_err(db_err)?;
        let rows = st
            .query_map([], |r| {
                Ok(QueueItem {
                    id: r.get(0)?,
                    media_id: r.get(1)?,
                    position: r.get(2)?,
                })
            })
            .map_err(db_err)?;
        rows.collect::<Result<_, _>>().map_err(db_err)
    }

    fn enqueue(
        &self,
        media_ids: &[Id],
        mode: QueueMode,
        after: Option<Id>,
    ) -> DomainResult<Vec<QueueItem>> {
        if mode == QueueMode::Replace {
            self.clear_queue()?;
        }
        let current = self.queue()?;
        // Rang d'insertion dans la file actuelle.
        let insert_at = match (mode, after) {
            (QueueMode::Next, Some(a)) => current
                .iter()
                .position(|i| i.id == a)
                .map_or(current.len(), |p| p + 1),
            (QueueMode::Next, None) => 0,
            _ => current.len(),
        };
        let mut new_ids = Vec::new();
        self.transaction(|tx| {
            let mut st = tx
                .prepare("INSERT INTO queue_item (media_id, position) VALUES (?1, -1)")
                .map_err(db_err)?;
            for mid in media_ids {
                st.execute([mid]).map_err(db_err)?;
                new_ids.push(tx.last_insert_rowid());
            }
            Ok(())
        })?;
        let mut order: Vec<Id> = current.iter().map(|i| i.id).collect();
        order.splice(insert_at..insert_at, new_ids.iter().copied());
        self.reorder_queue(&order)?;
        Ok(self
            .queue()?
            .into_iter()
            .filter(|i| new_ids.contains(&i.id))
            .collect())
    }

    fn remove_queue_item(&self, id: Id) -> DomainResult<bool> {
        let n = self
            .conn
            .execute("DELETE FROM queue_item WHERE id = ?1", [id])
            .map_err(db_err)?;
        if n > 0 {
            let order: Vec<Id> = self.queue()?.into_iter().map(|i| i.id).collect();
            self.reorder_queue(&order)?;
        }
        Ok(n > 0)
    }

    fn move_queue_item(&self, id: Id, position: i64) -> DomainResult<bool> {
        let mut order: Vec<Id> = self.queue()?.into_iter().map(|i| i.id).collect();
        let Some(from) = order.iter().position(|i| *i == id) else {
            return Ok(false);
        };
        order.remove(from);
        let to = (position.max(0) as usize).min(order.len());
        order.insert(to, id);
        self.reorder_queue(&order)?;
        Ok(true)
    }

    fn clear_queue(&self) -> DomainResult<()> {
        self.conn
            .execute("DELETE FROM queue_item", [])
            .map_err(db_err)?;
        Ok(())
    }

    fn reorder_queue(&self, ids_in_order: &[Id]) -> DomainResult<()> {
        self.transaction(|tx| {
            let mut st = tx
                .prepare("UPDATE queue_item SET position = ?2 WHERE id = ?1")
                .map_err(db_err)?;
            for (pos, id) in ids_in_order.iter().enumerate() {
                st.execute(params![id, pos as i64]).map_err(db_err)?;
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::media::insert_test_media;
    use epikodi_domain::media::MediaKind;

    fn media_order(s: &Store) -> Vec<Id> {
        s.queue().unwrap().into_iter().map(|i| i.media_id).collect()
    }

    #[test]
    fn modes_d_ajout() {
        let s = Store::in_memory();
        let m: Vec<Id> = (0..5)
            .map(|i| insert_test_media(&s, None, MediaKind::Movie, &format!("m{i}"), Some(0)))
            .collect();
        let first = s.enqueue(&[m[0], m[1]], QueueMode::Append, None).unwrap();
        assert_eq!(first.len(), 2);
        s.enqueue(&[m[2]], QueueMode::Append, None).unwrap();
        assert_eq!(media_order(&s), [m[0], m[1], m[2]]);
        // « lire ensuite » : juste après l'élément en cours (m0)
        s.enqueue(&[m[3]], QueueMode::Next, Some(first[0].id))
            .unwrap();
        assert_eq!(media_order(&s), [m[0], m[3], m[1], m[2]]);
        s.enqueue(&[m[4]], QueueMode::Replace, None).unwrap();
        assert_eq!(media_order(&s), [m[4]]);
    }

    #[test]
    fn deplacer_et_retirer() {
        let s = Store::in_memory();
        let m: Vec<Id> = (0..3)
            .map(|i| insert_test_media(&s, None, MediaKind::Movie, &format!("m{i}"), Some(0)))
            .collect();
        let items = s.enqueue(&m, QueueMode::Append, None).unwrap();
        assert!(s.move_queue_item(items[2].id, 0).unwrap());
        assert_eq!(media_order(&s), [m[2], m[0], m[1]]);
        assert!(s.remove_queue_item(items[0].id).unwrap());
        assert_eq!(media_order(&s), [m[2], m[1]]);
        let positions: Vec<_> = s.queue().unwrap().into_iter().map(|i| i.position).collect();
        assert_eq!(positions, [0, 1]);
        assert!(!s.remove_queue_item(999).unwrap());
    }
}
