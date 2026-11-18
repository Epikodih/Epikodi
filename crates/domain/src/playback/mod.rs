//! Lecture : reprise, vu / non vu, état du lecteur.
//!
//! La progression est stockée sur la feuille (film, épisode, morceau). Le
//! « vu » d'une série ou d'une saison se déduit de ses épisodes.

use serde::{Deserialize, Serialize};

use crate::{media::Media, Id};

/// Progression d'un profil sur un média (table `watch_progress`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchProgress {
    pub media_id: Id,
    pub position_s: i64,
    pub is_watched: bool,
    pub last_played_at: Option<String>,
}

/// Au-delà de cette part de la durée, le média est considéré comme vu.
pub const WATCHED_RATIO: f64 = 0.9;
/// En dessous de cette position, on ne propose pas de reprendre.
pub const MIN_RESUME_S: i64 = 30;
/// À moins de ce temps de la fin, on ne propose pas de reprendre.
pub const END_MARGIN_S: i64 = 60;

/// Calcule l'état après une mise à jour de position.
/// Renvoie `(position à stocker, vu ?)`.
pub fn apply_position(
    position_s: i64,
    duration_s: Option<i64>,
    already_watched: bool,
) -> (i64, bool) {
    let position = position_s.max(0);
    let watched = match duration_s {
        Some(d) if d > 0 => {
            let ratio = position as f64 / d as f64;
            ratio >= WATCHED_RATIO || d - position <= END_MARGIN_S.min(d / 10)
        }
        _ => false,
    };
    if watched {
        // fini : on repartira du début la prochaine fois
        (0, true)
    } else {
        (position, already_watched)
    }
}

/// Position à laquelle reprendre la lecture (0 = depuis le début).
pub fn resume_position(progress: Option<&WatchProgress>, duration_s: Option<i64>) -> i64 {
    let Some(p) = progress else { return 0 };
    if p.position_s < MIN_RESUME_S {
        return 0;
    }
    if let Some(d) = duration_s {
        if d > 0 && d - p.position_s <= END_MARGIN_S {
            return 0;
        }
    }
    p.position_s
}

/// Un conteneur est vu quand toutes ses feuilles le sont (et qu'il en a).
pub fn container_watched(leaves_watched: &[bool]) -> bool {
    !leaves_watched.is_empty() && leaves_watched.iter().all(|w| *w)
}

/// Épisode suivant dans une saison triée, puis premier épisode de la saison
/// suivante. `seasons` : saisons triées avec leurs épisodes triés.
pub fn next_episode(seasons: &[(Media, Vec<Media>)], current: Id) -> Option<&Media> {
    let mut found = false;
    for (_, episodes) in seasons {
        for ep in episodes {
            if found {
                return Some(ep);
            }
            if ep.id == current {
                found = true;
            }
        }
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerStatus {
    #[default]
    Stopped,
    Loading,
    Playing,
    Paused,
    /// Fin du média : le cœur enchaîne sur le suivant.
    Ended,
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::{MatchStatus, MediaKind, MediaStatus};

    fn progress(pos: i64) -> WatchProgress {
        WatchProgress {
            media_id: 1,
            position_s: pos,
            is_watched: false,
            last_played_at: None,
        }
    }

    #[test]
    fn position_normale() {
        assert_eq!(apply_position(120, Some(1000), false), (120, false));
    }

    #[test]
    fn fin_proche_marque_vu() {
        assert_eq!(apply_position(950, Some(1000), false), (0, true));
        // 90 % d'un film de 2 h
        assert_eq!(apply_position(6500, Some(7200), false), (0, true));
    }

    #[test]
    fn duree_inconnue_jamais_vu_automatiquement() {
        assert_eq!(apply_position(99_999, None, false), (99_999, false));
    }

    #[test]
    fn deja_vu_reste_vu_en_revisionnant() {
        assert_eq!(apply_position(10, Some(1000), true), (10, true));
    }

    #[test]
    fn position_negative_ramenee_a_zero() {
        assert_eq!(apply_position(-5, Some(1000), false), (0, false));
    }

    #[test]
    fn reprise() {
        assert_eq!(resume_position(None, Some(1000)), 0);
        assert_eq!(resume_position(Some(&progress(10)), Some(1000)), 0);
        assert_eq!(resume_position(Some(&progress(300)), Some(1000)), 300);
        assert_eq!(resume_position(Some(&progress(980)), Some(1000)), 0);
        assert_eq!(resume_position(Some(&progress(300)), None), 300);
    }

    #[test]
    fn conteneur_vu() {
        assert!(container_watched(&[true, true]));
        assert!(!container_watched(&[true, false]));
        assert!(!container_watched(&[]));
    }

    fn media(id: Id, kind: MediaKind) -> Media {
        Media {
            id,
            parent_id: None,
            kind,
            title: format!("m{id}"),
            year: None,
            poster_path: None,
            duration_s: None,
            sort_number: None,
            age_rating: None,
            match_status: MatchStatus::Auto,
            source_id: None,
            uri: None,
            parent_dir: None,
            size_bytes: None,
            modified_at: None,
            fingerprint: None,
            status: MediaStatus::Available,
            added_at: String::new(),
        }
    }

    #[test]
    fn episode_suivant() {
        let s1 = (
            media(10, MediaKind::Season),
            vec![media(1, MediaKind::Episode), media(2, MediaKind::Episode)],
        );
        let s2 = (
            media(20, MediaKind::Season),
            vec![media(3, MediaKind::Episode)],
        );
        let seasons = vec![s1, s2];
        assert_eq!(next_episode(&seasons, 1).unwrap().id, 2);
        assert_eq!(next_episode(&seasons, 2).unwrap().id, 3, "saison suivante");
        assert!(next_episode(&seasons, 3).is_none());
        assert!(next_episode(&seasons, 99).is_none());
    }
}
