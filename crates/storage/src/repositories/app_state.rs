use rusqlite::params;

use epikodi_domain::app_state::AppState;
use epikodi_domain::ports::AppStateRepo;
use epikodi_domain::queue::RepeatMode;
use epikodi_domain::{DomainResult, Id};

use super::{db_err, parse_col, Store};

impl Store {
    fn set_state(&self, column: &str, value: rusqlite::types::Value) -> DomainResult<()> {
        self.conn
            .execute(
                &format!("UPDATE app_state SET {column} = ?1 WHERE id = 1"),
                params![value],
            )
            .map_err(db_err)?;
        Ok(())
    }
}

impl AppStateRepo for Store {
    fn app_state(&self) -> DomainResult<AppState> {
        self.conn
            .query_row(
                "SELECT active_user_id, current_queue_item_id, volume, shuffle, repeat_mode
                 FROM app_state WHERE id = 1",
                [],
                |r| {
                    Ok(AppState {
                        active_user_id: r.get(0)?,
                        current_queue_item_id: r.get(1)?,
                        volume: r.get(2)?,
                        shuffle: r.get(3)?,
                        repeat_mode: parse_col(4, r.get(4)?)?,
                    })
                },
            )
            .map_err(db_err)
    }

    fn set_active_user(&self, id: Option<Id>) -> DomainResult<()> {
        self.set_state("active_user_id", id.into())
    }

    fn set_current_queue_item(&self, id: Option<Id>) -> DomainResult<()> {
        self.set_state("current_queue_item_id", id.into())
    }

    fn set_volume(&self, v: u8) -> DomainResult<()> {
        self.set_state("volume", i64::from(v.min(100)).into())
    }

    fn set_shuffle(&self, on: bool) -> DomainResult<()> {
        self.set_state("shuffle", i64::from(on).into())
    }

    fn set_repeat(&self, mode: RepeatMode) -> DomainResult<()> {
        self.set_state("repeat_mode", mode.as_str().to_string().into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn etat_global() {
        let s = Store::in_memory();
        assert_eq!(s.app_state().unwrap(), AppState::default());
        s.set_volume(30).unwrap();
        s.set_shuffle(true).unwrap();
        s.set_repeat(RepeatMode::All).unwrap();
        let st = s.app_state().unwrap();
        assert_eq!(
            (st.volume, st.shuffle, st.repeat_mode),
            (30, true, RepeatMode::All)
        );
    }
}
