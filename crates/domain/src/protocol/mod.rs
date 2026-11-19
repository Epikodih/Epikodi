//! Messages échangés entre le cœur et les apps (HTTP, WebSocket, SSE).
//!
//! C'est le contrat avec le client Dart : chaque structure correspondant à un
//! fichier de `fixtures/` est vérifiée par un test (décodage puis réencodage
//! à l'identique). Les dates sont en ISO 8601 UTC, les champs en snake_case.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::extension::{ExtensionStatus, ExtensionType, Permission};
use crate::media::{MatchStatus, MediaKind, MediaStatus};
use crate::playback::PlayerStatus;
use crate::queue::{QueueMode, RepeatMode};
use crate::source::{Credentials, Protocol, SourceStatus};
use crate::Id;

// ------------------------------------------------------------------ erreurs

/// `{"error": {"code", "message", "details"}}`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: ErrorBody,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
    /// Facultatif : `null` quand il n'y a rien à préciser.
    #[serde(default)]
    pub details: Option<Value>,
}

// ------------------------------------------------------------------- santé

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

// --------------------------------------------------------------------- TV

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TvConnectRequest {
    pub model: String,
    pub platform: String,
    pub app_version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TvConnectResponse {
    pub tv_session_id: String,
    pub lan_ip: String,
    pub pairing: PairingInfo,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PairingInfo {
    pub code: String,
    pub expires_at: String,
    pub expires_in_s: i64,
    pub attempts_left: u32,
    pub qr: QrInfo,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QrInfo {
    pub payload: String,
    pub format: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TvRefreshRequest {
    pub tv_session_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TvRefreshResponse {
    pub pairing: PairingInfo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TvPhase {
    /// Code affiché, aucun téléphone appairé.
    Pairing,
    /// Téléphone appairé, profil pas encore choisi.
    Paired,
    /// Profil actif.
    Ready,
}

/// `GET /api/tv/status` et chaque message du flux `GET /api/tv/events`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TvStatus {
    pub phase: TvPhase,
    pub pairing: Option<PairingInfo>,
    pub device: Option<DeviceSummary>,
    pub active_profile: Option<ProfileView>,
    pub guest: GuestInfo,
    pub can_create: bool,
    pub profiles: Vec<ProfileListItem>,
}

// -------------------------------------------------------------- appairage

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PairRequest {
    pub code: String,
    pub device: PairDevice,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PairDevice {
    pub name: String,
    #[serde(default)]
    pub install_id: Option<String>,
    #[serde(default)]
    pub platform: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PairResponse {
    pub token: String,
    pub token_type: String,
    pub expires_at: Option<String>,
    pub device: DeviceSummary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceSummary {
    pub id: Id,
    pub name: String,
}

/// Appareil dans `GET /api/session` et `GET /api/devices`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionDevice {
    pub id: Id,
    pub name: String,
    pub paired_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceView {
    pub id: Id,
    pub name: String,
    pub platform: Option<String>,
    pub paired_at: String,
    pub expires_at: Option<String>,
    pub last_seen_at: Option<String>,
    /// L'appareil qui fait la requête.
    pub current: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceList {
    pub devices: Vec<DeviceView>,
}

// ----------------------------------------------------------------- profils

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileKind {
    Account,
    Guest,
}

/// Profil dans les réponses (`profile`). Un invité a `id: null`,
/// `icon: null`, `age: 0` et `is_restricted: true`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileView {
    pub kind: ProfileKind,
    pub id: Option<Id>,
    pub name: String,
    pub icon: Option<String>,
    pub language: String,
    pub age: u8,
    pub is_restricted: bool,
}

/// `GET /api/profiles/{id}` : le profil et s'il demande un PIN.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileDetail {
    #[serde(flatten)]
    pub profile: ProfileView,
    pub requires_pin: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileListItem {
    pub id: Id,
    pub name: String,
    pub icon: Option<String>,
    pub language: String,
    pub age: u8,
    pub is_restricted: bool,
    pub requires_pin: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuestInfo {
    pub available: bool,
    pub is_restricted: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfilesList {
    pub guest: GuestInfo,
    pub can_create: bool,
    pub profiles: Vec<ProfileListItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateProfileRequest {
    pub name: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    pub age: i64,
    pub is_restricted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pin: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateProfileResponse {
    pub profile: ProfileView,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetProfileResponse {
    pub profile: ProfileDetail,
}

/// Mêmes champs que la création, tous facultatifs. `pin` (PIN actuel) est
/// obligatoire quand on touche à la restriction d'âge. `new_pin` change le
/// PIN d'un compte non restreint.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UpdateProfileRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub age: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_restricted: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pin: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_pin: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileResponse {
    pub profile: ProfileView,
}

/// `DELETE /api/profiles/{id}` : PIN du compte s'il en a un.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DeleteProfileRequest {
    #[serde(default)]
    pub pin: Option<String>,
}

// ----------------------------------------------------------------- session

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionCurrent {
    /// `null` pour l'app TV (accès local).
    pub device: Option<SessionDevice>,
    pub profile: Option<ProfileView>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectKind {
    Guest,
    Account,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectProfileRequest {
    #[serde(rename = "type")]
    pub kind: SelectKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<Id>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pin: Option<String>,
}

// ------------------------------------------------------------------ médias

/// Carte du catalogue (`fixtures/media/list.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaItem {
    pub id: Id,
    pub parent_id: Option<Id>,
    pub kind: MediaKind,
    pub title: String,
    pub year: Option<i32>,
    pub poster_url: Option<String>,
    pub duration_s: Option<i64>,
    pub sort_number: Option<i64>,
    pub age_rating: Option<u8>,
    pub match_status: MatchStatus,
    pub status: MediaStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaList {
    pub items: Vec<MediaItem>,
    pub total: u64,
    pub limit: u32,
    pub offset: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgressView {
    pub position_s: i64,
    pub is_watched: bool,
    pub last_played_at: Option<String>,
    /// Position à laquelle reprendre (0 = depuis le début).
    pub resume_at_s: i64,
}

/// `GET /api/media/{id}` : la carte, la fiche, et l'état propre au profil.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaDetailResponse {
    pub media: MediaItem,
    pub details: Value,
    /// Chemin du parent jusqu'au premier niveau (fil d'Ariane).
    pub ancestors: Vec<MediaItem>,
    pub children_count: u64,
    pub progress: Option<ProgressView>,
    pub is_watched: bool,
    pub is_favorite: bool,
    /// Ce que le lecteur ouvre (fichier ou URL). Seulement pour l'app TV.
    pub playback_uri: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaPatchRequest {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default, with = "double_option")]
    pub year: Option<Option<i32>>,
    #[serde(default, with = "double_option")]
    pub age_rating: Option<Option<u8>>,
    #[serde(default, with = "double_option")]
    pub sort_number: Option<Option<i64>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchCandidates {
    pub candidates: Vec<crate::ports::MetadataCandidate>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplyMatchRequest {
    pub provider: String,
    pub external_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CountItem<T> {
    pub value: T,
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenreList {
    pub genres: Vec<CountItem<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct YearList {
    pub years: Vec<CountItem<i32>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProgressRequest {
    pub position_s: i64,
    #[serde(default)]
    pub duration_s: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WatchedRequest {
    pub watched: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContinueItem {
    pub media: MediaItem,
    pub progress: ProgressView,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HomeResponse {
    pub continue_watching: Vec<ContinueItem>,
    pub recently_added: Vec<MediaItem>,
    pub favorites: Vec<MediaItem>,
    pub sections: Vec<HomeSection>,
    pub widgets: Vec<WidgetView>,
}

/// Accès aux rubriques (films, séries, musique, podcasts, extensions).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HomeSection {
    pub id: String,
    pub kind: Option<MediaKind>,
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WidgetView {
    pub extension_id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaItems {
    pub items: Vec<MediaItem>,
}

// -------------------------------------------------------- listes de lecture

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaylistView {
    pub id: Id,
    pub name: String,
    pub item_count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaylistList {
    pub playlists: Vec<PlaylistView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaylistEntry {
    pub position: i64,
    pub media: MediaItem,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaylistDetail {
    pub playlist: PlaylistView,
    pub items: Vec<PlaylistEntry>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaylistNameRequest {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MediaIdsRequest {
    pub media_ids: Vec<Id>,
}

// --------------------------------------------------------- file d'attente

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueueEntry {
    pub id: Id,
    pub position: i64,
    pub media: MediaItem,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueueView {
    pub items: Vec<QueueEntry>,
    pub current_item_id: Option<Id>,
    pub shuffle: bool,
    pub repeat: RepeatMode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnqueueRequest {
    pub media_ids: Vec<Id>,
    #[serde(default)]
    pub mode: QueueMode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoveQueueItemRequest {
    pub position: i64,
}

// ------------------------------------------------------------------ lecteur

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackInfo {
    pub id: i64,
    #[serde(default)]
    pub lang: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub codec: Option<String>,
    #[serde(default)]
    pub audio_description: bool,
}

/// Ce qui est en cours de lecture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NowPlaying {
    /// Média de la médiathèque (`null` pour un lien envoyé vers la télé).
    pub media: Option<MediaItem>,
    pub title: String,
    pub uri: String,
    pub queue_item_id: Option<Id>,
}

/// État du lecteur (`GET /api/player`, événement `player.state`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerState {
    pub status: PlayerStatus,
    pub now_playing: Option<NowPlaying>,
    pub position_s: f64,
    pub duration_s: Option<f64>,
    pub volume: u8,
    pub muted: bool,
    pub shuffle: bool,
    pub repeat: RepeatMode,
    pub audio_tracks: Vec<TrackInfo>,
    pub subtitle_tracks: Vec<TrackInfo>,
    pub audio_track: Option<i64>,
    pub subtitle_track: Option<i64>,
    pub updated_at: String,
}

/// Ce que l'app TV publie (`POST /api/player/state`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerReport {
    pub status: PlayerStatus,
    #[serde(default)]
    pub position_s: f64,
    #[serde(default)]
    pub duration_s: Option<f64>,
    #[serde(default)]
    pub audio_tracks: Vec<TrackInfo>,
    #[serde(default)]
    pub subtitle_tracks: Vec<TrackInfo>,
    #[serde(default)]
    pub audio_track: Option<i64>,
    #[serde(default)]
    pub subtitle_track: Option<i64>,
}

/// `POST /api/player/play` : un média (et ses feuilles), un élément de la
/// file, ou une liste de lecture. Un seul des trois.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PlayRequest {
    #[serde(default)]
    pub media_id: Option<Id>,
    #[serde(default)]
    pub queue_item_id: Option<Id>,
    #[serde(default)]
    pub playlist_id: Option<Id>,
    /// Reprendre là où on s'était arrêté (vrai par défaut).
    #[serde(default)]
    pub resume: Option<bool>,
}

/// Commandes de la télécommande (`POST /api/player/command`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum PlayerCommand {
    Play,
    Pause,
    Toggle,
    Stop,
    Next,
    Previous,
    Seek {
        position_s: f64,
    },
    SeekRelative {
        delta_s: f64,
    },
    Volume {
        level: i64,
    },
    VolumeUp,
    VolumeDown,
    Mute,
    Unmute,
    AudioTrack {
        id: i64,
    },
    /// `id: null` désactive les sous-titres.
    SubtitleTrack {
        id: Option<i64>,
    },
    SubtitleDelay {
        delay_ms: i64,
    },
    Shuffle {
        enabled: bool,
    },
    Repeat {
        mode: RepeatMode,
    },
}

/// Instruction envoyée à l'app TV (événement `player.command`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum TvCommand {
    /// Ouvre un média et le lit à partir de `start_s`.
    Load {
        now_playing: NowPlaying,
        start_s: f64,
    },
    Play,
    Pause,
    Toggle,
    Stop,
    Seek {
        position_s: f64,
    },
    SeekRelative {
        delta_s: f64,
    },
    Volume {
        level: u8,
        muted: bool,
    },
    AudioTrack {
        id: i64,
    },
    SubtitleTrack {
        id: Option<i64>,
    },
    SubtitleDelay {
        delay_ms: i64,
    },
}

/// Touches de la télécommande (pavé directionnel...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteKey {
    Up,
    Down,
    Left,
    Right,
    Ok,
    Back,
    Home,
    Menu,
    Info,
    PlayPause,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputRequest {
    pub key: RemoteKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CastKind {
    Video,
    Audio,
}

/// « Envoyer vers la télé » : un lien vidéo ou audio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CastRequest {
    pub url: String,
    #[serde(default)]
    pub title: Option<String>,
    pub kind: CastKind,
}

// ------------------------------------------------------------- préférences

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreferencesResponse {
    pub preferences: crate::profile::Preferences,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeList {
    pub themes: Vec<ThemeView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThemeView {
    pub id: String,
    pub name: String,
}

// ----------------------------------------------------------------- sources

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceView {
    pub id: Id,
    pub name: String,
    pub protocol: Protocol,
    pub uri: String,
    pub has_credentials: bool,
    pub status: SourceStatus,
    pub last_scan_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceList {
    pub sources: Vec<SourceView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateSourceRequest {
    pub name: String,
    pub protocol: Protocol,
    pub uri: String,
    #[serde(default)]
    pub credentials: Option<Credentials>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UpdateSourceRequest {
    #[serde(default)]
    pub name: Option<String>,
    /// `null` efface les identifiants ; absent = inchangés.
    #[serde(default, with = "double_option")]
    pub credentials: Option<Option<Credentials>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiscoveredSourceView {
    pub name: String,
    pub protocol: Protocol,
    pub uri: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiscoveredSources {
    pub sources: Vec<DiscoveredSourceView>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ScanStatus {
    pub running: bool,
    pub source_id: Option<Id>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub added: u64,
    pub updated: u64,
    pub moved: u64,
    pub missing: u64,
    pub unchanged: u64,
    pub enriched: u64,
    pub errors: Vec<String>,
    pub duration_ms: Option<u64>,
}

// -------------------------------------------------------------- extensions

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PermissionView {
    pub id: Permission,
    pub description: String,
    pub granted: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtensionView {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(rename = "type")]
    pub kind: ExtensionType,
    pub enabled: bool,
    pub status: ExtensionStatus,
    pub last_error: Option<String>,
    pub permissions: Vec<PermissionView>,
    pub settings: serde_json::Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtensionList {
    pub extensions: Vec<ExtensionView>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    #[serde(rename = "type")]
    pub kind: ExtensionType,
    pub permissions: Vec<PermissionView>,
    pub path: String,
    pub installed_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtensionCatalog {
    pub extensions: Vec<CatalogEntry>,
}

/// Installer ou mettre à jour depuis un dossier ou un `.zip` (chemin sur la
/// machine du media center), en acceptant des permissions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstallExtensionRequest {
    pub path: String,
    #[serde(default)]
    pub accept_permissions: Vec<Permission>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UpdateExtensionRequest {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub settings: Option<serde_json::Map<String, Value>>,
}

// --------------------------------------------------------------- événements

/// Événements temps réel (`GET /api/ws`) : `{"type": "...", "data": {...}}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Event {
    #[serde(rename = "tv.status")]
    TvStatus(Box<TvStatus>),
    #[serde(rename = "pairing.completed")]
    PairingCompleted(DeviceSummary),
    #[serde(rename = "profile.changed")]
    ProfileChanged { profile: Option<ProfileView> },
    #[serde(rename = "preferences.changed")]
    PreferencesChanged { profile_id: Id },
    #[serde(rename = "player.state")]
    PlayerState(Box<PlayerState>),
    #[serde(rename = "player.command")]
    PlayerCommand(TvCommand),
    #[serde(rename = "queue.changed")]
    QueueChanged {
        current_item_id: Option<Id>,
        length: usize,
    },
    #[serde(rename = "input")]
    Input { key: RemoteKey },
    #[serde(rename = "library.updated")]
    LibraryUpdated(ScanStatus),
    #[serde(rename = "source.status")]
    SourceStatus { source_id: Id, status: SourceStatus },
    #[serde(rename = "extension.status")]
    ExtensionStatus {
        extension_id: String,
        status: ExtensionStatus,
        message: Option<String>,
    },
}

/// Ce qu'un client peut envoyer sur le WebSocket.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Ping,
    Input { key: RemoteKey },
    Command { command: PlayerCommand },
}

/// Distingue « champ absent » (`None`) de « champ à null » (`Some(None)`).
mod double_option {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<T: Serialize, S: Serializer>(
        v: &Option<Option<T>>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        match v {
            None | Some(None) => s.serialize_none(),
            Some(Some(x)) => x.serialize(s),
        }
    }

    pub fn deserialize<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
        d: D,
    ) -> Result<Option<Option<T>>, D::Error> {
        Option::<T>::deserialize(d).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commande_avec_action() {
        let c: PlayerCommand =
            serde_json::from_str(r#"{"action":"seek","position_s":12.5}"#).unwrap();
        assert_eq!(c, PlayerCommand::Seek { position_s: 12.5 });
        let c: PlayerCommand =
            serde_json::from_str(r#"{"action":"subtitle_track","id":null}"#).unwrap();
        assert_eq!(c, PlayerCommand::SubtitleTrack { id: None });
        assert!(serde_json::from_str::<PlayerCommand>(r#"{"action":"explode"}"#).is_err());
    }

    #[test]
    fn evenement_type_et_data() {
        let e = Event::Input { key: RemoteKey::Up };
        assert_eq!(
            serde_json::to_value(&e).unwrap(),
            serde_json::json!({"type": "input", "data": {"key": "up"}})
        );
    }

    #[test]
    fn double_option_distingue_absent_et_null() {
        let r: UpdateSourceRequest = serde_json::from_str("{}").unwrap();
        assert_eq!(r.credentials, None);
        let r: UpdateSourceRequest = serde_json::from_str(r#"{"credentials":null}"#).unwrap();
        assert_eq!(r.credentials, Some(None));
    }
}
