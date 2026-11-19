//! Lecteur : la télécommande commande, le cœur décide, l'app TV exécute.
//!
//! - La télécommande envoie ses commandes (`/api/player/*`) ; le cœur met à
//!   jour la file, le volume, les modes, puis envoie une instruction à la TV
//!   (événement `player.command`).
//! - L'app TV lit avec son moteur (libmpv) et publie son état
//!   (`POST /api/player/state`) ; le cœur enregistre la progression du profil
//!   (au plus toutes les 10 s, et toujours à la pause / l'arrêt / la fin),
//!   diffuse l'état à tous (`player.state`) et enchaîne le média suivant.

use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use futures_util::StreamExt;
use rand::Rng;
use serde::Deserialize;
use tokio::io::AsyncWriteExt;

use epikodi_domain::app_state::check_volume;
use epikodi_domain::media::Media;
use epikodi_domain::playback::{apply_position, resume_position, PlayerStatus, WatchProgress};
use epikodi_domain::ports::{
    AppStateRepo, MediaRepo, PlaylistRepo, ProfileRepo, ProgressRepo, QueueRepo,
};
use epikodi_domain::protocol::{
    CastRequest, Event, InputRequest, NowPlaying, PlayRequest, PlayerCommand, PlayerReport,
    PlayerState, TvCommand,
};
use epikodi_domain::queue::{next_in_queue, previous_in_queue, shuffled, QueueMode, RepeatMode};
use epikodi_domain::source::media_extension;
use epikodi_domain::Id;

use crate::auth::{Auth, Ctx, LocalOnly};
use crate::error::{ApiError, ApiResult};
use crate::extract::{JsonBody, QueryParams};
use crate::http::media::next_of;
use crate::http::queue::{emit_changed, expand, view as queue_view};
use crate::state::{lock, AppState, Selection};
use crate::views;

/// Intervalle minimal entre deux sauvegardes de la progression.
pub const SAVE_EVERY: Duration = Duration::from_secs(10);
/// Taille maximale d'un fichier envoyé depuis le téléphone.
pub const MAX_CAST_UPLOAD: u64 = 500 * 1024 * 1024;
/// Les fichiers envoyés sont supprimés après ce délai.
pub const CAST_MAX_AGE: Duration = Duration::from_secs(24 * 3600);

/// Supprime les fichiers envoyés depuis le téléphone plus vieux que
/// `max_age` (le dossier ne grossit pas indéfiniment). Renvoie le nombre
/// de fichiers supprimés.
pub fn prune_cast_dir(dir: &std::path::Path, max_age: Duration) -> usize {
    let Ok(read) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut removed = 0;
    for e in read.flatten() {
        let old = e
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age >= max_age);
        if old && e.path().is_file() && std::fs::remove_file(e.path()).is_ok() {
            removed += 1;
        }
    }
    removed
}

pub struct PlayerRuntime {
    pub state: PlayerState,
    pub last_save: Option<Instant>,
    pub shuffle_seed: u64,
}

impl Default for PlayerRuntime {
    fn default() -> Self {
        Self {
            state: PlayerState {
                status: PlayerStatus::Stopped,
                now_playing: None,
                position_s: 0.0,
                duration_s: None,
                volume: 80,
                muted: false,
                shuffle: false,
                repeat: RepeatMode::Off,
                audio_tracks: Vec::new(),
                subtitle_tracks: Vec::new(),
                audio_track: None,
                subtitle_track: None,
                updated_at: String::new(),
            },
            last_save: None,
            shuffle_seed: 1,
        }
    }
}

fn snapshot(state: &AppState) -> PlayerState {
    let mut p = lock(&state.player);
    p.state.updated_at = state.now_iso();
    p.state.clone()
}

fn publish(state: &AppState) -> PlayerState {
    let s = snapshot(state);
    state.emit(Event::PlayerState(Box::new(s.clone())));
    s
}

fn send_tv(state: &AppState, cmd: TvCommand) {
    state.emit(Event::PlayerCommand(cmd));
}

fn nothing_to_play() -> ApiError {
    ApiError::conflict("nothing_to_play", "Rien à lire.")
}

/// Ce que le lecteur de la TV doit ouvrir.
fn playback_uri(m: &Media) -> String {
    let uri = m.uri.clone().unwrap_or_default();
    epikodi_sources::playback_path(&uri)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or(uri)
}

/// Ordre de lecture (mélangé si `shuffle`).
fn play_order(state: &AppState, ids: &[Id], current: Option<Id>) -> Vec<Id> {
    let p = lock(&state.player);
    if p.state.shuffle {
        shuffled(ids, current, p.shuffle_seed)
    } else {
        ids.to_vec()
    }
}

/// Charge un élément de la file sur la TV.
async fn load_item(
    state: &AppState,
    user: Option<Id>,
    item_id: Id,
    resume: bool,
) -> ApiResult<PlayerState> {
    let (media, start) = state
        .db(move |s| {
            let item = s
                .queue()?
                .into_iter()
                .find(|i| i.id == item_id)
                .ok_or_else(|| {
                    ApiError::not_found("queue_item_not_found", "Élément de la file introuvable.")
                })?;
            let m = s
                .get_media(item.media_id)?
                .filter(|m| m.kind.is_leaf())
                .ok_or_else(nothing_to_play)?;
            let start = match (user, resume) {
                (Some(u), true) => resume_position(s.get_progress(u, m.id)?.as_ref(), m.duration_s),
                _ => 0,
            };
            s.set_current_queue_item(Some(item_id))?;
            Ok((m, start))
        })
        .await?;
    let now_playing = NowPlaying {
        media: Some(views::media_item(&media)),
        title: media.title.clone(),
        uri: playback_uri(&media),
        queue_item_id: Some(item_id),
    };
    {
        let mut p = lock(&state.player);
        p.state.status = PlayerStatus::Loading;
        p.state.now_playing = Some(now_playing.clone());
        p.state.position_s = start as f64;
        p.state.duration_s = media.duration_s.map(|d| d as f64);
        p.state.audio_tracks.clear();
        p.state.subtitle_tracks.clear();
        p.state.audio_track = None;
        p.state.subtitle_track = None;
        p.last_save = None;
    }
    send_tv(
        state,
        TvCommand::Load {
            now_playing,
            start_s: start as f64,
        },
    );
    state.emit(Event::QueueChanged {
        current_item_id: Some(item_id),
        length: state.db(|s| Ok(s.queue()?.len())).await?,
    });
    Ok(publish(state))
}

fn stop_playback(state: &AppState) -> PlayerState {
    {
        let mut p = lock(&state.player);
        p.state.status = PlayerStatus::Stopped;
        p.state.now_playing = None;
        p.state.position_s = 0.0;
    }
    send_tv(state, TvCommand::Stop);
    publish(state)
}

/// Passe à l'élément suivant / précédent. `auto` : fin de lecture (respecte
/// « répéter un » et enchaîne l'épisode suivant d'une série).
async fn advance(state: &AppState, forward: bool, auto: bool) -> ApiResult<PlayerState> {
    let (ids, current) = state
        .db(|s| {
            Ok((
                s.queue()?.iter().map(|i| i.id).collect::<Vec<_>>(),
                s.app_state()?.current_queue_item_id,
            ))
        })
        .await?;
    let repeat = lock(&state.player).state.repeat;
    let order = play_order(state, &ids, current);
    let effective = if !auto && repeat == RepeatMode::One {
        RepeatMode::Off
    } else {
        repeat
    };
    let target = if forward {
        next_in_queue(&order, current, effective)
    } else {
        previous_in_queue(&order, current, effective)
    };
    let user = match state.selection() {
        Selection::Account(id) => Some(id),
        _ => None,
    };
    if let Some(item) = target {
        return load_item(state, user, item, !auto).await;
    }
    if auto {
        // Fin de la file : épisode suivant, si le profil le veut.
        if let Some(next) = next_episode_for_current(state).await? {
            let item = state
                .db(move |s| {
                    Ok(s.enqueue(&[next], QueueMode::Append, None)?
                        .first()
                        .map(|i| i.id))
                })
                .await?;
            if let Some(item) = item {
                return load_item(state, user, item, true).await;
            }
        }
    }
    Ok(stop_playback(state))
}

async fn next_episode_for_current(state: &AppState) -> ApiResult<Option<Id>> {
    let current = lock(&state.player)
        .state
        .now_playing
        .as_ref()
        .and_then(|n| n.media.as_ref().map(|m| m.id));
    let Some(media_id) = current else {
        return Ok(None);
    };
    let profile = state.active_profile().await?;
    let Some(profile) = profile else {
        return Ok(None);
    };
    let autoplay = profile
        .account()
        .is_none_or(|p| p.preferences.autoplay_next);
    if !autoplay {
        return Ok(None);
    }
    let vis = profile.visibility();
    state
        .db(move |s| {
            let Some(m) = s.get_media(media_id)? else {
                return Ok(None);
            };
            Ok(next_of(s, &m, vis)?.map(|n| n.id))
        })
        .await
}

/// `GET /api/player`
pub async fn get(_: Auth, State(state): State<AppState>) -> Json<PlayerState> {
    Json(snapshot(&state))
}

/// `POST /api/player/play` : lance un média (toutes ses feuilles), un
/// élément de la file ou une liste de lecture sur la TV.
pub async fn play(
    ctx: Ctx,
    State(state): State<AppState>,
    JsonBody(req): JsonBody<PlayRequest>,
) -> ApiResult<Json<PlayerState>> {
    let vis = ctx.visibility()?;
    let user = ctx.profile()?.user_id();
    let given = [req.media_id, req.queue_item_id, req.playlist_id]
        .iter()
        .filter(|x| x.is_some())
        .count();
    if given != 1 {
        return Err(ApiError::validation(
            "media_id",
            "un seul parmi media_id, queue_item_id, playlist_id",
        ));
    }
    let resume = req.resume.unwrap_or(true);
    if req.playlist_id.is_some() && user.is_none() {
        return Err(ApiError::guest_forbidden());
    }
    let st = state.clone();
    let item_id = state
        .db(move |s| -> ApiResult<Id> {
            if let Some(qid) = req.queue_item_id {
                return s
                    .queue()?
                    .iter()
                    .find(|i| i.id == qid)
                    .map(|i| i.id)
                    .ok_or_else(|| {
                        ApiError::not_found(
                            "queue_item_not_found",
                            "Élément de la file introuvable.",
                        )
                    });
            }
            let media_ids: Vec<Id> = match (req.media_id, req.playlist_id, user) {
                (Some(id), _, _) => vec![id],
                (None, Some(pid), Some(u)) => {
                    s.get_playlist(u, pid)?.ok_or_else(|| {
                        ApiError::not_found("playlist_not_found", "Liste de lecture introuvable.")
                    })?;
                    s.playlist_items(pid)?.iter().map(|i| i.media_id).collect()
                }
                _ => Vec::new(),
            };
            let leaves = expand(s, &media_ids, vis)?;
            if leaves.is_empty() {
                return Err(nothing_to_play());
            }
            // Une série reprend au premier épisode non vu.
            let start = match (user, req.media_id) {
                (Some(u), Some(_)) if leaves.len() > 1 => {
                    let rows = s.progress_for(u, &leaves)?;
                    leaves
                        .iter()
                        .position(|id| !rows.iter().any(|r| r.media_id == *id && r.is_watched))
                        .unwrap_or(0)
                }
                _ => 0,
            };
            let items = s.enqueue(&leaves, QueueMode::Replace, None)?;
            {
                // nouvelle file : nouveau mélange
                lock(&st.player).shuffle_seed = rand::thread_rng().gen();
            }
            Ok(items[start].id)
        })
        .await?;
    let res = load_item(&state, user, item_id, resume).await?;
    if let Ok(q) = queue_view(&state, vis).await {
        emit_changed(&state, &q);
    }
    Ok(Json(res))
}

/// Applique une commande de la télécommande (HTTP ou WebSocket).
pub async fn apply_command(state: &AppState, cmd: PlayerCommand) -> ApiResult<PlayerState> {
    use PlayerCommand as C;
    let set_volume = |level: i64| -> ApiResult<TvCommand> {
        let v = check_volume(level)?;
        let mut p = lock(&state.player);
        p.state.volume = v;
        p.state.muted = false;
        Ok(TvCommand::Volume {
            level: v,
            muted: false,
        })
    };
    match cmd {
        C::Play | C::Pause | C::Toggle => {
            let tv = {
                let mut p = lock(&state.player);
                let playing = match cmd {
                    C::Play => true,
                    C::Pause => false,
                    _ => p.state.status != PlayerStatus::Playing,
                };
                if p.state.now_playing.is_some() {
                    p.state.status = if playing {
                        PlayerStatus::Playing
                    } else {
                        PlayerStatus::Paused
                    };
                }
                if playing {
                    TvCommand::Play
                } else {
                    TvCommand::Pause
                }
            };
            send_tv(state, tv);
        }
        C::Stop => return Ok(stop_playback(state)),
        C::Next => return advance(state, true, false).await,
        C::Previous => return advance(state, false, false).await,
        C::Seek { position_s } => {
            if !position_s.is_finite() || position_s < 0.0 {
                return Err(ApiError::validation("position_s", "positive"));
            }
            lock(&state.player).state.position_s = position_s;
            send_tv(state, TvCommand::Seek { position_s });
        }
        C::SeekRelative { delta_s } => {
            if !delta_s.is_finite() {
                return Err(ApiError::validation("delta_s", "nombre attendu"));
            }
            {
                let mut p = lock(&state.player);
                p.state.position_s = (p.state.position_s + delta_s).max(0.0);
            }
            send_tv(state, TvCommand::SeekRelative { delta_s });
        }
        C::Volume { level } => {
            let tv = set_volume(level)?;
            send_tv(state, tv);
        }
        C::VolumeUp | C::VolumeDown => {
            let cur = lock(&state.player).state.volume as i64;
            let step = if matches!(cmd, C::VolumeUp) { 5 } else { -5 };
            let tv = set_volume((cur + step).clamp(0, 100))?;
            send_tv(state, tv);
        }
        C::Mute | C::Unmute => {
            let tv = {
                let mut p = lock(&state.player);
                p.state.muted = matches!(cmd, C::Mute);
                TvCommand::Volume {
                    level: p.state.volume,
                    muted: p.state.muted,
                }
            };
            send_tv(state, tv);
        }
        C::AudioTrack { id } => {
            lock(&state.player).state.audio_track = Some(id);
            send_tv(state, TvCommand::AudioTrack { id });
        }
        C::SubtitleTrack { id } => {
            lock(&state.player).state.subtitle_track = id;
            send_tv(state, TvCommand::SubtitleTrack { id });
        }
        C::SubtitleDelay { delay_ms } => {
            if delay_ms.abs() > 60_000 {
                return Err(ApiError::validation("delay_ms", "entre -60000 et 60000"));
            }
            send_tv(state, TvCommand::SubtitleDelay { delay_ms });
        }
        // Enregistré d'abord : si la base échoue, l'état ne change pas.
        C::Shuffle { enabled } => {
            state.db(move |s| Ok(s.set_shuffle(enabled)?)).await?;
            let mut p = lock(&state.player);
            p.state.shuffle = enabled;
            p.shuffle_seed = rand::thread_rng().gen();
        }
        C::Repeat { mode } => {
            state.db(move |s| Ok(s.set_repeat(mode)?)).await?;
            lock(&state.player).state.repeat = mode;
        }
    }
    // Volume : gardé pour le prochain démarrage.
    if let C::Volume { .. } | C::VolumeUp | C::VolumeDown = cmd {
        let v = lock(&state.player).state.volume;
        state.db(move |s| Ok(s.set_volume(v)?)).await?;
    }
    Ok(publish(state))
}

/// `POST /api/player/command` : `{"action": "pause"}`, `{"action": "seek", "position_s": 42}`...
pub async fn command(
    _: Auth,
    State(state): State<AppState>,
    JsonBody(cmd): JsonBody<PlayerCommand>,
) -> ApiResult<Json<PlayerState>> {
    Ok(Json(apply_command(&state, cmd).await?))
}

/// Enregistre la progression du compte actif sur le média en cours.
async fn save_progress(
    state: &AppState,
    media_id: Id,
    position: f64,
    duration: Option<f64>,
    finished: bool,
) -> ApiResult<()> {
    let Selection::Account(user) = state.selection() else {
        return Ok(()); // l'invité n'a pas d'historique
    };
    let at = state.now_iso();
    state
        .db(move |s| {
            if s.get_profile(user)?.is_none() {
                return Ok(());
            }
            let Some(m) = s.get_media(media_id)? else {
                return Ok(());
            };
            let duration = duration.map(|d| d as i64).or(m.duration_s);
            let position = if finished {
                duration.unwrap_or(position as i64)
            } else {
                position as i64
            };
            let before = s.get_progress(user, media_id)?;
            let (position_s, is_watched) = apply_position(
                position,
                duration,
                before.as_ref().is_some_and(|p| p.is_watched),
            );
            Ok(s.save_progress(
                user,
                &WatchProgress {
                    media_id,
                    position_s,
                    is_watched,
                    last_played_at: Some(at),
                },
            )?)
        })
        .await
}

/// `POST /api/player/state` (app TV, local) : état réel du lecteur.
pub async fn report(
    _: LocalOnly,
    State(state): State<AppState>,
    JsonBody(r): JsonBody<PlayerReport>,
) -> ApiResult<Json<PlayerState>> {
    if !r.position_s.is_finite() || r.position_s < 0.0 {
        return Err(ApiError::validation("position_s", "positive"));
    }
    let (media_id, must_save) = {
        let mut p = lock(&state.player);
        p.state.status = r.status;
        p.state.position_s = r.position_s;
        if r.duration_s.is_some() {
            p.state.duration_s = r.duration_s;
        }
        p.state.audio_tracks = r.audio_tracks.clone();
        p.state.subtitle_tracks = r.subtitle_tracks.clone();
        p.state.audio_track = r.audio_track;
        p.state.subtitle_track = r.subtitle_track;
        let media_id = p
            .state
            .now_playing
            .as_ref()
            .and_then(|n| n.media.as_ref())
            .map(|m| m.id);
        let important = matches!(
            r.status,
            PlayerStatus::Paused | PlayerStatus::Stopped | PlayerStatus::Ended
        );
        let due = p.last_save.is_none_or(|t| t.elapsed() >= SAVE_EVERY);
        let must_save =
            media_id.is_some() && (important || (r.status == PlayerStatus::Playing && due));
        if must_save {
            p.last_save = Some(Instant::now());
        }
        (media_id, must_save)
    };
    if let (Some(id), true) = (media_id, must_save) {
        let duration = lock(&state.player).state.duration_s;
        save_progress(
            &state,
            id,
            r.position_s,
            duration,
            r.status == PlayerStatus::Ended,
        )
        .await?;
    }
    if r.status == PlayerStatus::Ended {
        return Ok(Json(advance(&state, true, true).await?));
    }
    Ok(Json(publish(&state)))
}

/// `POST /api/remote/input` : touche du pavé directionnel (relayée à la TV).
pub async fn input(
    _: Auth,
    State(state): State<AppState>,
    JsonBody(req): JsonBody<InputRequest>,
) -> StatusCode {
    state.emit(Event::Input { key: req.key });
    StatusCode::NO_CONTENT
}

fn start_cast(state: &AppState, title: String, uri: String) -> PlayerState {
    let now_playing = NowPlaying {
        media: None,
        title,
        uri,
        queue_item_id: None,
    };
    {
        let mut p = lock(&state.player);
        p.state.status = PlayerStatus::Loading;
        p.state.now_playing = Some(now_playing.clone());
        p.state.position_s = 0.0;
        p.state.duration_s = None;
        p.last_save = None;
    }
    send_tv(
        state,
        TvCommand::Load {
            now_playing,
            start_s: 0.0,
        },
    );
    publish(state)
}

/// `POST /api/player/cast` : « Envoyer vers la télé » un lien vidéo ou audio.
pub async fn cast(
    _: Auth,
    State(state): State<AppState>,
    JsonBody(req): JsonBody<CastRequest>,
) -> ApiResult<Json<PlayerState>> {
    let url = req.url.trim();
    let lower = url.to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://"))
        || url.len() > 2048
        || url.contains(char::is_whitespace)
    {
        return Err(ApiError::validation("url", "lien http(s) attendu"));
    }
    let title = req
        .title
        .clone()
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| {
            url.rsplit('/')
                .find(|s| !s.is_empty())
                .unwrap_or(url)
                .to_string()
        });
    Ok(Json(start_cast(&state, title, url.to_string())))
}

#[derive(Debug, Deserialize)]
pub struct UploadQuery {
    pub name: String,
}

/// `POST /api/player/cast/upload?name=morceau.mp3` : envoie un fichier du
/// téléphone (corps brut) et le lit sur la TV.
pub async fn cast_upload(
    _: Auth,
    State(state): State<AppState>,
    QueryParams(q): QueryParams<UploadQuery>,
    body: Body,
) -> ApiResult<Json<PlayerState>> {
    let name: String = q
        .name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, '.' | '-' | '_' | ' '))
        .take(120)
        .collect();
    if media_extension(&name).is_none() {
        return Err(ApiError::validation(
            "name",
            "fichier audio ou vidéo attendu",
        ));
    }
    let dir = state.config.cast_dir();
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(ApiError::internal)?;
    let prune_dir = dir.clone();
    let _ = tokio::task::spawn_blocking(move || prune_cast_dir(&prune_dir, CAST_MAX_AGE)).await;
    let path = dir.join(format!("{}-{name}", rand::thread_rng().gen::<u32>()));
    let mut file = tokio::fs::File::create(&path)
        .await
        .map_err(ApiError::internal)?;
    let mut stream = body.into_data_stream();
    let mut total: u64 = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| ApiError::validation("body", "envoi interrompu"))?;
        total += chunk.len() as u64;
        if total > MAX_CAST_UPLOAD {
            drop(file);
            let _ = tokio::fs::remove_file(&path).await;
            return Err(ApiError::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                "payload_too_large",
                "Fichier trop volumineux.",
            ));
        }
        file.write_all(&chunk).await.map_err(ApiError::internal)?;
    }
    file.flush().await.map_err(ApiError::internal)?;
    if total == 0 {
        let _ = tokio::fs::remove_file(&path).await;
        return Err(ApiError::validation("body", "fichier vide"));
    }
    Ok(Json(start_cast(
        &state,
        name,
        path.to_string_lossy().into_owned(),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anciens_fichiers_envoyes_supprimes() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("a.mp3"), b"x").unwrap();
        assert_eq!(
            prune_cast_dir(d.path(), Duration::from_secs(3600)),
            0,
            "récent : gardé"
        );
        assert_eq!(prune_cast_dir(d.path(), Duration::ZERO), 1);
        assert!(!d.path().join("a.mp3").exists());
        assert_eq!(prune_cast_dir(&d.path().join("absent"), Duration::ZERO), 0);
    }
}
