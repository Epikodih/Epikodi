//! État global de l'application (une seule ligne, `id = 1`).

use serde::{Deserialize, Serialize};

use crate::{queue::RepeatMode, DomainError, DomainResult, Id};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppState {
    pub active_user_id: Option<Id>,
    pub current_queue_item_id: Option<Id>,
    pub volume: u8,
    pub shuffle: bool,
    pub repeat_mode: RepeatMode,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            active_user_id: None,
            current_queue_item_id: None,
            volume: 80,
            shuffle: false,
            repeat_mode: RepeatMode::Off,
        }
    }
}

pub fn check_volume(v: i64) -> DomainResult<u8> {
    if (0..=100).contains(&v) {
        Ok(v as u8)
    } else {
        Err(DomainError::validation("volume", "entre 0 et 100"))
    }
}
