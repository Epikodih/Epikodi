//! Médias : film, série, saison, épisode, artiste, album, morceau, podcast.
//!
//! Tout est un `Media` ; la hiérarchie passe par `parent_id`. Les règles que
//! le SQL ne vérifie pas (quel type sous quel parent) sont ici.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{DomainError, DomainResult, Id};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    Movie,
    Series,
    Season,
    Episode,
    Artist,
    Album,
    Track,
    Podcast,
    PodcastEpisode,
}

impl MediaKind {
    pub const ALL: [MediaKind; 9] = [
        Self::Movie,
        Self::Series,
        Self::Season,
        Self::Episode,
        Self::Artist,
        Self::Album,
        Self::Track,
        Self::Podcast,
        Self::PodcastEpisode,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Movie => "movie",
            Self::Series => "series",
            Self::Season => "season",
            Self::Episode => "episode",
            Self::Artist => "artist",
            Self::Album => "album",
            Self::Track => "track",
            Self::Podcast => "podcast",
            Self::PodcastEpisode => "podcast_episode",
        }
    }

    /// Type de parent attendu (`None` = média de premier niveau).
    pub fn expected_parent(self) -> Option<MediaKind> {
        match self {
            Self::Season => Some(Self::Series),
            Self::Episode => Some(Self::Season),
            Self::Album => Some(Self::Artist),
            Self::Track => Some(Self::Album),
            Self::PodcastEpisode => Some(Self::Podcast),
            Self::Movie | Self::Series | Self::Artist | Self::Podcast => None,
        }
    }

    /// Une feuille se lit (elle a un fichier ou une URL) ; un conteneur non.
    pub fn is_leaf(self) -> bool {
        matches!(
            self,
            Self::Movie | Self::Episode | Self::Track | Self::PodcastEpisode
        )
    }

    pub fn is_top_level(self) -> bool {
        self.expected_parent().is_none()
    }

    pub fn is_video(self) -> bool {
        matches!(self, Self::Movie | Self::Episode)
    }
}

impl fmt::Display for MediaKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for MediaKind {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|k| k.as_str() == s)
            .ok_or_else(|| DomainError::validation("kind", format!("type inconnu : {s}")))
    }
}

/// Vérifie qu'un média de type `kind` peut être rangé sous `parent`.
pub fn check_hierarchy(kind: MediaKind, parent: Option<MediaKind>) -> DomainResult<()> {
    match (kind.expected_parent(), parent) {
        (None, None) => Ok(()),
        (Some(expected), Some(actual)) if expected == actual => Ok(()),
        (None, Some(actual)) => Err(DomainError::Rule(format!(
            "un média « {kind} » ne peut pas être rangé sous « {actual} »"
        ))),
        (Some(expected), actual) => Err(DomainError::Rule(format!(
            "un média « {kind} » doit être rangé sous « {expected} » (reçu : {})",
            actual.map_or("aucun parent".to_string(), |k| k.to_string())
        ))),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchStatus {
    /// Pas reconnu : le titre est le nom du fichier.
    Unmatched,
    /// Reconnu automatiquement.
    Auto,
    /// Corrigé à la main : jamais écrasé par un enrichissement automatique.
    Manual,
}

impl MatchStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unmatched => "unmatched",
            Self::Auto => "auto",
            Self::Manual => "manual",
        }
    }

    /// Un enrichissement automatique peut-il remplacer les infos actuelles ?
    pub fn allows_auto_enrichment(self) -> bool {
        self != Self::Manual
    }
}

impl FromStr for MatchStatus {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "unmatched" => Ok(Self::Unmatched),
            "auto" => Ok(Self::Auto),
            "manual" => Ok(Self::Manual),
            _ => Err(DomainError::validation("match_status", s.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaStatus {
    Available,
    /// Fichier disparu (NAS débranché...) : la ligne est gardée.
    Missing,
}

impl MediaStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Missing => "missing",
        }
    }
}

impl FromStr for MediaStatus {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "available" => Ok(Self::Available),
            "missing" => Ok(Self::Missing),
            _ => Err(DomainError::validation("status", s.to_string())),
        }
    }
}

/// Classifications autorisées (en années).
pub const AGE_RATINGS: [u8; 5] = [0, 10, 12, 16, 18];

pub fn check_age_rating(rating: Option<u8>) -> DomainResult<()> {
    match rating {
        Some(r) if !AGE_RATINGS.contains(&r) => Err(DomainError::validation(
            "age_rating",
            "valeurs possibles : 0, 10, 12, 16, 18",
        )),
        _ => Ok(()),
    }
}

/// Un média tel qu'il est stocké (table `media`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Media {
    pub id: Id,
    pub parent_id: Option<Id>,
    pub kind: MediaKind,
    pub title: String,
    pub year: Option<i32>,
    pub poster_path: Option<String>,
    pub duration_s: Option<i64>,
    pub sort_number: Option<i64>,
    pub age_rating: Option<u8>,
    pub match_status: MatchStatus,
    pub source_id: Option<Id>,
    pub uri: Option<String>,
    pub parent_dir: Option<String>,
    pub size_bytes: Option<i64>,
    pub modified_at: Option<String>,
    pub fingerprint: Option<String>,
    pub status: MediaStatus,
    pub added_at: String,
}

/// Données pour créer un média (scan, création à la demande des conteneurs).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NewMedia {
    pub parent_id: Option<Id>,
    pub kind: Option<MediaKind>,
    pub title: String,
    pub year: Option<i32>,
    pub duration_s: Option<i64>,
    pub sort_number: Option<i64>,
    pub age_rating: Option<u8>,
    pub match_status: Option<MatchStatus>,
    pub source_id: Option<Id>,
    pub uri: Option<String>,
    pub parent_dir: Option<String>,
    pub size_bytes: Option<i64>,
    pub modified_at: Option<String>,
    pub fingerprint: Option<String>,
}

/// Correction manuelle d'un média : les champs absents ne changent pas.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MediaPatch {
    pub title: Option<String>,
    pub year: Option<Option<i32>>,
    pub age_rating: Option<Option<u8>>,
    pub sort_number: Option<Option<i64>>,
    pub duration_s: Option<Option<i64>>,
    pub poster_path: Option<Option<String>>,
    pub match_status: Option<MatchStatus>,
}

/// Fiche détaillée (`media_details.details`), typée. Les clés absentes
/// prennent leur valeur par défaut ; les clés inconnues sont gardées dans
/// `extra` pour ne rien perdre.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MediaDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overview: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tagline: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backdrop: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rating: Option<f64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub genres: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cast: Vec<CastMember>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lyrics: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub audio_tracks: Vec<AudioTrack>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub subtitles: Vec<SubtitleTrack>,
    #[serde(skip_serializing_if = "serde_json::Map::is_empty")]
    pub external_ids: serde_json::Map<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CastMember {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub character: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioTrack {
    pub lang: Option<String>,
    pub codec: Option<String>,
    pub audio_description: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SubtitleTrack {
    pub lang: Option<String>,
    pub format: Option<String>,
    pub embedded: bool,
    pub uri: Option<String>,
}

/// Ordre de tri d'une liste du catalogue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaSort {
    /// Par titre au premier niveau, par `sort_number` dans un parent.
    #[default]
    Default,
    Title,
    Year,
    Added,
}

/// Filtres d'une liste du catalogue (`GET /api/media`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MediaQuery {
    pub kind: Option<MediaKind>,
    pub parent_id: Option<Id>,
    pub search: Option<String>,
    pub genre: Option<String>,
    pub year: Option<i32>,
    pub sort: MediaSort,
    pub limit: u32,
    pub offset: u32,
}

impl MediaQuery {
    pub const DEFAULT_LIMIT: u32 = 50;
    pub const MAX_LIMIT: u32 = 500;
}

#[derive(Debug, Clone, PartialEq)]
pub struct MediaPage {
    pub items: Vec<Media>,
    pub total: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hierarchie_valide() {
        assert!(check_hierarchy(MediaKind::Movie, None).is_ok());
        assert!(check_hierarchy(MediaKind::Season, Some(MediaKind::Series)).is_ok());
        assert!(check_hierarchy(MediaKind::Episode, Some(MediaKind::Season)).is_ok());
        assert!(check_hierarchy(MediaKind::Album, Some(MediaKind::Artist)).is_ok());
        assert!(check_hierarchy(MediaKind::Track, Some(MediaKind::Album)).is_ok());
        assert!(check_hierarchy(MediaKind::PodcastEpisode, Some(MediaKind::Podcast)).is_ok());
    }

    #[test]
    fn hierarchie_invalide() {
        assert!(check_hierarchy(MediaKind::Episode, Some(MediaKind::Series)).is_err());
        assert!(check_hierarchy(MediaKind::Episode, None).is_err());
        assert!(check_hierarchy(MediaKind::Movie, Some(MediaKind::Series)).is_err());
        assert!(check_hierarchy(MediaKind::Track, Some(MediaKind::Artist)).is_err());
    }

    #[test]
    fn feuilles_et_conteneurs() {
        assert!(MediaKind::Movie.is_leaf());
        assert!(MediaKind::Track.is_leaf());
        assert!(!MediaKind::Series.is_leaf());
        assert!(!MediaKind::Album.is_leaf());
    }

    #[test]
    fn kind_aller_retour_texte() {
        for k in MediaKind::ALL {
            assert_eq!(k.as_str().parse::<MediaKind>().unwrap(), k);
        }
        assert!("film".parse::<MediaKind>().is_err());
    }

    #[test]
    fn manuel_jamais_ecrase() {
        assert!(!MatchStatus::Manual.allows_auto_enrichment());
        assert!(MatchStatus::Auto.allows_auto_enrichment());
        assert!(MatchStatus::Unmatched.allows_auto_enrichment());
    }

    #[test]
    fn classifications() {
        assert!(check_age_rating(Some(12)).is_ok());
        assert!(check_age_rating(None).is_ok());
        assert!(check_age_rating(Some(13)).is_err());
    }

    #[test]
    fn details_valeurs_par_defaut_et_cles_inconnues() {
        let d: MediaDetails = serde_json::from_str(r#"{"overview":"x","custom":{"a":1}}"#).unwrap();
        assert_eq!(d.overview.as_deref(), Some("x"));
        assert!(d.genres.is_empty());
        assert_eq!(d.extra["custom"]["a"], 1);
        let back = serde_json::to_value(&d).unwrap();
        assert_eq!(back["custom"]["a"], 1);
        let empty: MediaDetails = serde_json::from_str("{}").unwrap();
        assert_eq!(empty, MediaDetails::default());
    }
}
