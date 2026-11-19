//! File d'attente (une seule, globale) et modes de lecture.

use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{DomainError, Id};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueueItem {
    pub id: Id,
    pub media_id: Id,
    pub position: i64,
}

/// Où insérer des médias dans la file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueMode {
    /// À la fin.
    #[default]
    Append,
    /// Juste après le média en cours.
    Next,
    /// Vide la file puis ajoute.
    Replace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepeatMode {
    #[default]
    Off,
    One,
    All,
}

impl RepeatMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::One => "one",
            Self::All => "all",
        }
    }
}

impl FromStr for RepeatMode {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "off" => Ok(Self::Off),
            "one" => Ok(Self::One),
            "all" => Ok(Self::All),
            _ => Err(DomainError::validation("repeat", "off, one ou all")),
        }
    }
}

/// Élément suivant de la file, selon le mode de répétition.
/// `order` : identifiants des éléments dans l'ordre de lecture.
pub fn next_in_queue(order: &[Id], current: Option<Id>, repeat: RepeatMode) -> Option<Id> {
    if order.is_empty() {
        return None;
    }
    let Some(cur) = current else {
        return order.first().copied();
    };
    if repeat == RepeatMode::One {
        return Some(cur);
    }
    match order.iter().position(|id| *id == cur) {
        Some(i) if i + 1 < order.len() => Some(order[i + 1]),
        Some(_) if repeat == RepeatMode::All => order.first().copied(),
        Some(_) => None,
        None => order.first().copied(),
    }
}

/// Élément précédent (reste sur le premier en début de file, sauf `All`).
pub fn previous_in_queue(order: &[Id], current: Option<Id>, repeat: RepeatMode) -> Option<Id> {
    let cur = current?;
    match order.iter().position(|id| *id == cur) {
        Some(0) if repeat == RepeatMode::All => order.last().copied(),
        Some(0) => Some(cur),
        Some(i) => Some(order[i - 1]),
        None => order.first().copied(),
    }
}

/// Ordre aléatoire déterministe à partir d'une graine (testable) :
/// l'élément en cours reste en tête.
pub fn shuffled(order: &[Id], current: Option<Id>, seed: u64) -> Vec<Id> {
    let mut rest: Vec<Id> = order
        .iter()
        .copied()
        .filter(|id| Some(*id) != current)
        .collect();
    // xorshift : suffisant pour mélanger une file de lecture
    let mut state = seed | 1;
    for i in (1..rest.len()).rev() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let j = (state % (i as u64 + 1)) as usize;
        rest.swap(i, j);
    }
    current.into_iter().chain(rest).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suivant() {
        let o = [1, 2, 3];
        assert_eq!(next_in_queue(&o, None, RepeatMode::Off), Some(1));
        assert_eq!(next_in_queue(&o, Some(1), RepeatMode::Off), Some(2));
        assert_eq!(next_in_queue(&o, Some(3), RepeatMode::Off), None);
        assert_eq!(next_in_queue(&o, Some(3), RepeatMode::All), Some(1));
        assert_eq!(next_in_queue(&o, Some(2), RepeatMode::One), Some(2));
        assert_eq!(next_in_queue(&[], None, RepeatMode::All), None);
    }

    #[test]
    fn precedent() {
        let o = [1, 2, 3];
        assert_eq!(previous_in_queue(&o, Some(2), RepeatMode::Off), Some(1));
        assert_eq!(previous_in_queue(&o, Some(1), RepeatMode::Off), Some(1));
        assert_eq!(previous_in_queue(&o, Some(1), RepeatMode::All), Some(3));
        assert_eq!(previous_in_queue(&o, None, RepeatMode::Off), None);
    }

    #[test]
    fn melange_garde_tous_les_elements_et_le_courant_en_tete() {
        let o: Vec<Id> = (1..=20).collect();
        let s = shuffled(&o, Some(5), 42);
        assert_eq!(s[0], 5);
        let mut sorted = s.clone();
        sorted.sort();
        assert_eq!(sorted, o);
        assert_ne!(s, o);
        assert_eq!(s, shuffled(&o, Some(5), 42), "déterministe");
    }

    #[test]
    fn repeat_texte() {
        assert_eq!("all".parse::<RepeatMode>().unwrap(), RepeatMode::All);
        assert!("toujours".parse::<RepeatMode>().is_err());
    }
}
