//! Ports : traits implémentés par les autres crates.
//!
//! - les dépôts (`*Repo`) sont implémentés par `storage` (SQLite) ;
//! - [`FileSource`] par `sources` (dossier local, partage réseau) ;
//! - [`MetadataProvider`] par `metadata` (TMDB, MusicBrainz, faux fournisseur).
//!
//! `library` et `extensions` ne connaissent que ces traits : on les teste avec
//! de faux dépôts, sans base ni réseau.
//!
//! Choix d'équipe : `MetadataProvider` vit ici (et pas dans `metadata`) pour
//! que `library` ne dépende que de `domain`.

use crate::app_state::AppState;
use crate::device::{Device, NewDevice};
use crate::extension::{Extension, ExtensionStatus};
use crate::media::{
    Media, MediaDetails, MediaKind, MediaPage, MediaPatch, MediaQuery, MediaStatus, NewMedia,
};
use crate::parental::Visibility;
use crate::playback::WatchProgress;
use crate::playlist::{Playlist, PlaylistItem};
use crate::profile::{NewProfile, Preferences, Profile};
use crate::queue::{QueueItem, QueueMode, RepeatMode};
use crate::source::{AudioTags, Credentials, FileEntry, Protocol, Source, SourceStatus};
use crate::{DomainResult, Id};

// ------------------------------------------------------------------ profils

pub trait ProfileRepo {
    fn list_profiles(&self) -> DomainResult<Vec<Profile>>;
    fn get_profile(&self, id: Id) -> DomainResult<Option<Profile>>;
    /// Erreur `Conflict` si le nom est déjà pris.
    fn create_profile(&self, p: &NewProfile) -> DomainResult<Profile>;
    /// Enregistre tous les champs (nom, icône, âge, restriction, PIN...).
    fn update_profile(&self, p: &Profile) -> DomainResult<()>;
    fn delete_profile(&self, id: Id) -> DomainResult<()>;
    fn set_preferences(&self, id: Id, prefs: &Preferences) -> DomainResult<()>;
}

// --------------------------------------------------------------- appareils

pub trait DeviceRepo {
    fn create_device(&self, d: &NewDevice) -> DomainResult<Device>;
    /// Appareil non révoqué dont le jeton a ce hash.
    /// Appareil dont le jeton a ce hash, ni révoqué ni expiré à la date
    /// `now` (ISO 8601 UTC, horloge du serveur).
    fn find_device_by_token(&self, token_hash: &str, now: &str) -> DomainResult<Option<Device>>;
    fn get_device(&self, id: Id) -> DomainResult<Option<Device>>;
    /// L'appareil est-il encore autorisé (ni révoqué ni expiré) à `now` ?
    fn device_valid(&self, id: Id, now: &str) -> DomainResult<bool>;
    /// Appareils ni révoqués ni expirés à la date `now`.
    fn list_devices(&self, now: &str) -> DomainResult<Vec<Device>>;
    fn revoke_device(&self, id: Id, at: &str) -> DomainResult<()>;
    /// Révoque les anciens jetons d'une même installation (ré-appairage).
    fn revoke_install(&self, install_id: &str, at: &str) -> DomainResult<()>;
    fn touch_device(&self, id: Id, at: &str) -> DomainResult<()>;
}

// ------------------------------------------------------------- catalogue

pub trait MediaRepo {
    /// Sans filtre : réservé au cœur (scan, lecture déjà autorisée...).
    fn get_media(&self, id: Id) -> DomainResult<Option<Media>>;
    /// Avec le filtre parental (classification effective).
    fn get_visible_media(&self, id: Id, vis: Visibility) -> DomainResult<Option<Media>>;
    fn list_media(&self, q: &MediaQuery, vis: Visibility) -> DomainResult<MediaPage>;
    fn children(&self, parent_id: Id, vis: Visibility) -> DomainResult<Vec<Media>>;
    /// Feuilles (lisibles) sous un média, dans l'ordre de lecture. Un média
    /// feuille renvoie lui-même.
    fn leaves_under(&self, id: Id, vis: Visibility) -> DomainResult<Vec<Media>>;
    fn recently_added(&self, vis: Visibility, limit: u32) -> DomainResult<Vec<Media>>;
    fn genres(&self, vis: Visibility) -> DomainResult<Vec<(String, u64)>>;
    fn years(&self, vis: Visibility) -> DomainResult<Vec<(i32, u64)>>;
    fn get_details(&self, id: Id) -> DomainResult<Option<MediaDetails>>;
    fn set_details(&self, id: Id, details: &MediaDetails) -> DomainResult<()>;
    fn patch_media(&self, id: Id, patch: &MediaPatch) -> DomainResult<()>;
}

/// Opérations du scan (écrites par `library`).
pub trait LibraryRepo {
    /// Fichiers connus d'une source (feuilles avec `uri`).
    fn known_files(&self, source_id: Id) -> DomainResult<Vec<Media>>;
    fn find_by_uri(&self, uri: &str) -> DomainResult<Option<Media>>;
    /// Média `missing` ayant cette empreinte (fichier déplacé).
    fn find_missing_by_fingerprint(&self, fingerprint: &str) -> DomainResult<Option<Media>>;
    fn insert_media(&self, m: &NewMedia) -> DomainResult<Id>;
    /// Met à jour les infos de fichier (uri, taille, date, empreinte,
    /// dossier, statut `available`).
    fn update_file(&self, id: Id, m: &NewMedia) -> DomainResult<()>;
    fn set_media_status(&self, id: Id, status: MediaStatus) -> DomainResult<()>;
    /// Conteneur existant (série, saison, artiste, album, podcast) par titre
    /// (sans tenir compte de la casse) et parent.
    fn find_container(
        &self,
        kind: MediaKind,
        parent_id: Option<Id>,
        title: &str,
        sort_number: Option<i64>,
    ) -> DomainResult<Option<Id>>;
    /// Médias à enrichir (non reconnus), toutes sources.
    fn unmatched(&self, limit: u32) -> DomainResult<Vec<Media>>;
    fn set_source_scanned(&self, source_id: Id, status: SourceStatus, at: &str)
        -> DomainResult<()>;
    /// Exécute `f` d'un bloc (transaction) : un crash en plein scan ne
    /// laisse pas une série à moitié écrite.
    fn atomically(&self, f: &mut dyn FnMut() -> DomainResult<()>) -> DomainResult<()> {
        f()
    }
}

// --------------------------------------------------------- données profil

pub trait ProgressRepo {
    fn get_progress(&self, user_id: Id, media_id: Id) -> DomainResult<Option<WatchProgress>>;
    fn save_progress(&self, user_id: Id, p: &WatchProgress) -> DomainResult<()>;
    fn set_watched(
        &self,
        user_id: Id,
        media_ids: &[Id],
        watched: bool,
        at: &str,
    ) -> DomainResult<()>;
    /// Médias en cours (position > 0, non vus), du plus récent au plus ancien.
    fn in_progress(
        &self,
        user_id: Id,
        vis: Visibility,
        limit: u32,
    ) -> DomainResult<Vec<(Media, WatchProgress)>>;
    fn progress_for(&self, user_id: Id, media_ids: &[Id]) -> DomainResult<Vec<WatchProgress>>;
}

pub trait FavoriteRepo {
    fn is_favorite(&self, user_id: Id, media_id: Id) -> DomainResult<bool>;
    fn add_favorite(&self, user_id: Id, media_id: Id) -> DomainResult<()>;
    fn remove_favorite(&self, user_id: Id, media_id: Id) -> DomainResult<()>;
    fn favorites(&self, user_id: Id, vis: Visibility) -> DomainResult<Vec<Media>>;
}

pub trait PlaylistRepo {
    fn list_playlists(&self, user_id: Id) -> DomainResult<Vec<Playlist>>;
    fn get_playlist(&self, user_id: Id, id: Id) -> DomainResult<Option<Playlist>>;
    fn create_playlist(&self, user_id: Id, name: &str) -> DomainResult<Playlist>;
    fn rename_playlist(&self, id: Id, name: &str) -> DomainResult<()>;
    fn delete_playlist(&self, id: Id) -> DomainResult<()>;
    fn playlist_items(&self, id: Id) -> DomainResult<Vec<PlaylistItem>>;
    fn add_playlist_items(&self, id: Id, media_ids: &[Id]) -> DomainResult<()>;
    /// Retire l'élément à cette position et renumérote.
    fn remove_playlist_item(&self, id: Id, position: i64) -> DomainResult<bool>;
}

pub trait QueueRepo {
    fn queue(&self) -> DomainResult<Vec<QueueItem>>;
    /// Ajoute selon le mode ; `after` = élément en cours (mode `Next`).
    fn enqueue(
        &self,
        media_ids: &[Id],
        mode: QueueMode,
        after: Option<Id>,
    ) -> DomainResult<Vec<QueueItem>>;
    fn remove_queue_item(&self, id: Id) -> DomainResult<bool>;
    fn move_queue_item(&self, id: Id, position: i64) -> DomainResult<bool>;
    fn clear_queue(&self) -> DomainResult<()>;
    fn reorder_queue(&self, ids_in_order: &[Id]) -> DomainResult<()>;
}

pub trait AppStateRepo {
    fn app_state(&self) -> DomainResult<AppState>;
    fn set_active_user(&self, id: Option<Id>) -> DomainResult<()>;
    fn set_current_queue_item(&self, id: Option<Id>) -> DomainResult<()>;
    fn set_volume(&self, v: u8) -> DomainResult<()>;
    fn set_shuffle(&self, on: bool) -> DomainResult<()>;
    fn set_repeat(&self, mode: RepeatMode) -> DomainResult<()>;
}

// ----------------------------------------------------------------- sources

#[derive(Debug, Clone, PartialEq)]
pub struct NewSource {
    pub name: String,
    pub protocol: Protocol,
    pub uri: String,
    pub credentials: Option<Credentials>,
}

pub trait SourceRepo {
    fn list_sources(&self) -> DomainResult<Vec<Source>>;
    fn get_source(&self, id: Id) -> DomainResult<Option<Source>>;
    /// Les identifiants sont chiffrés avant d'être écrits.
    fn create_source(&self, s: &NewSource) -> DomainResult<Source>;
    fn rename_source(&self, id: Id, name: &str) -> DomainResult<()>;
    /// `Some(None)` efface les identifiants.
    fn set_source_credentials(&self, id: Id, creds: Option<&Credentials>) -> DomainResult<()>;
    fn source_credentials(&self, id: Id) -> DomainResult<Option<Credentials>>;
    fn set_source_status(&self, id: Id, status: SourceStatus) -> DomainResult<()>;
    fn delete_source(&self, id: Id) -> DomainResult<()>;
}

// -------------------------------------------------------------- extensions

pub trait ExtensionRepo {
    fn list_extensions(&self) -> DomainResult<Vec<Extension>>;
    fn get_extension(&self, id: &str) -> DomainResult<Option<Extension>>;
    /// Insère ou remplace (mise à jour).
    fn save_extension(&self, e: &Extension) -> DomainResult<()>;
    fn set_extension_status(
        &self,
        id: &str,
        status: ExtensionStatus,
        error: Option<&str>,
    ) -> DomainResult<()>;
    fn delete_extension(&self, id: &str) -> DomainResult<()>;
}

// ------------------------------------------------------- accès aux fichiers

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SourceError {
    /// Partage injoignable (réseau, NAS éteint) : la médiathèque reste
    /// consultable, les fichiers ne sont PAS marqués disparus.
    #[error("source injoignable : {0}")]
    Unreachable(String),
    /// Dossier racine supprimé.
    #[error("dossier introuvable : {0}")]
    NotFound(String),
    #[error("accès refusé : {0}")]
    PermissionDenied(String),
    /// Délai dépassé (partage lent).
    #[error("délai dépassé")]
    Timeout,
    #[error("protocole non pris en charge : {0}")]
    Unsupported(String),
}

pub trait FileSource: Send + Sync {
    /// Tous les fichiers de médias de la source.
    fn list(&self) -> Result<Vec<FileEntry>, SourceError>;
    /// Fichiers d'un seul dossier (scan ciblé).
    fn list_dir(&self, dir_uri: &str) -> Result<Vec<FileEntry>, SourceError> {
        Ok(self
            .list()?
            .into_iter()
            .filter(|e| e.parent_dir == dir_uri)
            .collect())
    }
    /// Tags audio, si le fichier en a.
    fn audio_tags(&self, _entry: &FileEntry) -> Option<AudioTags> {
        None
    }
    /// Empreinte stable du contenu (retrouver un fichier déplacé).
    fn fingerprint(&self, entry: &FileEntry) -> Option<String> {
        Some(format!(
            "{}:{}",
            entry.size_bytes,
            file_name(&entry.relative_path)
        ))
    }
}

fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

// ---------------------------------------------------------- métadonnées

/// Ce qu'on cherche chez un fournisseur.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MetadataQuery {
    pub kind: Option<MediaKind>,
    pub title: String,
    pub year: Option<i32>,
    pub artist: Option<String>,
    pub season: Option<i64>,
    pub episode: Option<i64>,
}

/// Un résultat proposé (correction manuelle : l'utilisateur choisit).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MetadataCandidate {
    pub provider: String,
    pub external_id: String,
    pub kind: MediaKind,
    pub title: String,
    pub year: Option<i32>,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
}

/// Fiche complète renvoyée par un fournisseur.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MetadataRecord {
    pub title: Option<String>,
    pub year: Option<i32>,
    pub age_rating: Option<u8>,
    pub duration_s: Option<i64>,
    pub poster_url: Option<String>,
    pub details: MediaDetails,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MetadataError {
    /// Pas de réseau, API en panne : on garde ce qu'on a (mode hors ligne).
    #[error("fournisseur indisponible : {0}")]
    Unavailable(String),
    #[error("introuvable")]
    NotFound,
    #[error("réponse invalide : {0}")]
    Invalid(String),
}

pub trait MetadataProvider: Send + Sync {
    fn name(&self) -> &str;
    fn supports(&self, kind: MediaKind) -> bool;
    fn search(&self, q: &MetadataQuery) -> Result<Vec<MetadataCandidate>, MetadataError>;
    fn fetch(&self, kind: MediaKind, external_id: &str) -> Result<MetadataRecord, MetadataError>;
    fn fetch_image(&self, url: &str) -> Result<Vec<u8>, MetadataError>;
}

/// Cache local des images (affiches) : fichiers, pas SQLite.
pub trait ImageCache: Send + Sync {
    /// Enregistre l'image et renvoie son chemin relatif dans le cache.
    fn store(&self, key: &str, bytes: &[u8]) -> DomainResult<String>;
}
