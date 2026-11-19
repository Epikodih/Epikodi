//! Données du profil : accueil, favoris, listes de lecture, préférences, thèmes.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde_json::Value;

use epikodi_domain::extension::{ExtensionType, Permission};
use epikodi_domain::media::{MediaKind, MediaQuery};
use epikodi_domain::playlist::normalize_playlist_name;
use epikodi_domain::ports::{ExtensionRepo, FavoriteRepo, MediaRepo, PlaylistRepo, ProfileRepo};
use epikodi_domain::profile::Preferences;
use epikodi_domain::protocol::{
    Event, HomeResponse, HomeSection, MediaIdsRequest, MediaItems, PlaylistDetail, PlaylistEntry,
    PlaylistList, PlaylistNameRequest, PlaylistView, PreferencesResponse, ThemeList, ThemeView,
    WidgetView,
};
use epikodi_domain::Id;

use crate::auth::Ctx;
use crate::error::{ApiError, ApiResult};
use crate::extract::{JsonBody, PathId};
use crate::http::media::continue_items;
use crate::state::AppState;
use crate::views;

// ----------------------------------------------------------------- accueil

/// `GET /api/home` : reprise, derniers ajouts, favoris, rubriques, widgets.
pub async fn home(ctx: Ctx, State(state): State<AppState>) -> ApiResult<Json<HomeResponse>> {
    let vis = ctx.visibility()?;
    let user = ctx.profile()?.user_id();
    let continue_watching = continue_items(&ctx, &state, 20).await?;
    let (recent, favorites, sections, widgets) = state
        .db(move |s| {
            let recent = s.recently_added(vis, 20)?;
            let favorites = match user {
                Some(u) => s.favorites(u, vis)?,
                None => Vec::new(),
            };
            let mut sections = Vec::new();
            for (id, kind) in [
                ("movies", MediaKind::Movie),
                ("series", MediaKind::Series),
                ("music", MediaKind::Artist),
                ("podcasts", MediaKind::Podcast),
            ] {
                let count = s
                    .list_media(
                        &MediaQuery {
                            kind: Some(kind),
                            limit: 1,
                            ..Default::default()
                        },
                        vis,
                    )?
                    .total;
                sections.push(HomeSection {
                    id: id.into(),
                    kind: Some(kind),
                    count,
                });
            }
            let extensions = s.list_extensions()?;
            sections.push(HomeSection {
                id: "extensions".into(),
                kind: None,
                count: extensions.iter().filter(|e| e.enabled).count() as u64,
            });
            let widgets = extensions
                .iter()
                .filter(|e| {
                    e.enabled
                        && e.kind == ExtensionType::Widget
                        && e.permissions.allows(Permission::UiHome)
                })
                .map(|e| WidgetView {
                    extension_id: e.id.clone(),
                    name: e.name.clone(),
                })
                .collect();
            Ok((recent, favorites, sections, widgets))
        })
        .await?;
    Ok(Json(HomeResponse {
        continue_watching,
        recently_added: views::media_items(&recent),
        favorites: views::media_items(&favorites),
        sections,
        widgets,
    }))
}

// ----------------------------------------------------------------- favoris

/// `GET /api/favorites`
pub async fn favorites(ctx: Ctx, State(state): State<AppState>) -> ApiResult<Json<MediaItems>> {
    let vis = ctx.visibility()?;
    let user = ctx.account()?.id;
    let list = state.db(move |s| Ok(s.favorites(user, vis)?)).await?;
    Ok(Json(MediaItems {
        items: views::media_items(&list),
    }))
}

/// `PUT /api/favorites/{media_id}` (204)
pub async fn add_favorite(
    ctx: Ctx,
    State(state): State<AppState>,
    PathId(id): PathId,
) -> ApiResult<StatusCode> {
    let vis = ctx.visibility()?;
    let user = ctx.account()?.id;
    state
        .db(move |s| {
            s.get_visible_media(id, vis)?
                .ok_or_else(|| ApiError::not_found("media_not_found", "Média introuvable."))?;
            Ok(s.add_favorite(user, id)?)
        })
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /api/favorites/{media_id}` (204)
pub async fn remove_favorite(
    ctx: Ctx,
    State(state): State<AppState>,
    PathId(id): PathId,
) -> ApiResult<StatusCode> {
    let user = ctx.account()?.id;
    state.db(move |s| Ok(s.remove_favorite(user, id)?)).await?;
    Ok(StatusCode::NO_CONTENT)
}

// --------------------------------------------------------- listes de lecture

fn playlist_not_found() -> ApiError {
    ApiError::not_found("playlist_not_found", "Liste de lecture introuvable.")
}

/// `GET /api/playlists`
pub async fn playlists(ctx: Ctx, State(state): State<AppState>) -> ApiResult<Json<PlaylistList>> {
    let user = ctx.account()?.id;
    let list = state.db(move |s| Ok(s.list_playlists(user)?)).await?;
    Ok(Json(PlaylistList {
        playlists: list.iter().map(views::playlist_view).collect(),
    }))
}

/// `POST /api/playlists` (201)
pub async fn create_playlist(
    ctx: Ctx,
    State(state): State<AppState>,
    JsonBody(req): JsonBody<PlaylistNameRequest>,
) -> ApiResult<(StatusCode, Json<PlaylistView>)> {
    let user = ctx.account()?.id;
    let name = normalize_playlist_name(&req.name)?;
    let p = state
        .db(move |s| Ok(s.create_playlist(user, &name)?))
        .await?;
    Ok((StatusCode::CREATED, Json(views::playlist_view(&p))))
}

/// `GET /api/playlists/{id}` : la liste et ses médias (visibles).
pub async fn playlist(
    ctx: Ctx,
    State(state): State<AppState>,
    PathId(id): PathId,
) -> ApiResult<Json<PlaylistDetail>> {
    let vis = ctx.visibility()?;
    let user = ctx.account()?.id;
    let detail = state
        .db(move |s| {
            let p = s.get_playlist(user, id)?.ok_or_else(playlist_not_found)?;
            let mut items = Vec::new();
            for it in s.playlist_items(id)? {
                if let Some(m) = s.get_visible_media(it.media_id, vis)? {
                    items.push(PlaylistEntry {
                        position: it.position,
                        media: views::media_item(&m),
                    });
                }
            }
            Ok(PlaylistDetail {
                playlist: views::playlist_view(&p),
                items,
            })
        })
        .await?;
    Ok(Json(detail))
}

/// `PATCH /api/playlists/{id}` : renommer.
pub async fn rename_playlist(
    ctx: Ctx,
    State(state): State<AppState>,
    PathId(id): PathId,
    JsonBody(req): JsonBody<PlaylistNameRequest>,
) -> ApiResult<Json<PlaylistView>> {
    let user = ctx.account()?.id;
    let name = normalize_playlist_name(&req.name)?;
    let p = state
        .db(move |s| {
            s.get_playlist(user, id)?.ok_or_else(playlist_not_found)?;
            s.rename_playlist(id, &name)?;
            s.get_playlist(user, id)?.ok_or_else(playlist_not_found)
        })
        .await?;
    Ok(Json(views::playlist_view(&p)))
}

/// `DELETE /api/playlists/{id}` (204)
pub async fn delete_playlist(
    ctx: Ctx,
    State(state): State<AppState>,
    PathId(id): PathId,
) -> ApiResult<StatusCode> {
    let user = ctx.account()?.id;
    state
        .db(move |s| {
            s.get_playlist(user, id)?.ok_or_else(playlist_not_found)?;
            Ok(s.delete_playlist(id)?)
        })
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/playlists/{id}/items` : ajoute des médias à la fin.
pub async fn add_playlist_items(
    ctx: Ctx,
    State(state): State<AppState>,
    PathId(id): PathId,
    JsonBody(req): JsonBody<MediaIdsRequest>,
) -> ApiResult<Json<PlaylistView>> {
    let vis = ctx.visibility()?;
    let user = ctx.account()?.id;
    if req.media_ids.is_empty() || req.media_ids.len() > 500 {
        return Err(ApiError::validation("media_ids", "entre 1 et 500 médias"));
    }
    let p = state
        .db(move |s| {
            s.get_playlist(user, id)?.ok_or_else(playlist_not_found)?;
            for mid in &req.media_ids {
                s.get_visible_media(*mid, vis)?.ok_or_else(|| {
                    ApiError::not_found("media_not_found", format!("Média {mid} introuvable."))
                })?;
            }
            s.add_playlist_items(id, &req.media_ids)?;
            s.get_playlist(user, id)?.ok_or_else(playlist_not_found)
        })
        .await?;
    Ok(Json(views::playlist_view(&p)))
}

/// `DELETE /api/playlists/{id}/items/{position}` (204)
pub async fn remove_playlist_item(
    ctx: Ctx,
    State(state): State<AppState>,
    Path((id, position)): Path<(Id, i64)>,
) -> ApiResult<StatusCode> {
    let user = ctx.account()?.id;
    state
        .db(move |s| {
            s.get_playlist(user, id)?.ok_or_else(playlist_not_found)?;
            if !s.remove_playlist_item(id, position)? {
                return Err(ApiError::not_found(
                    "playlist_item_not_found",
                    "Élément introuvable.",
                ));
            }
            Ok(())
        })
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// ------------------------------------------------------------- préférences

/// `GET /api/me/preferences` (valeurs par défaut pour l'invité)
pub async fn get_preferences(ctx: Ctx) -> ApiResult<Json<PreferencesResponse>> {
    let prefs = ctx
        .profile()?
        .account()
        .map(|p| p.preferences.clone())
        .unwrap_or_default();
    Ok(Json(PreferencesResponse { preferences: prefs }))
}

/// Fusionne `patch` dans `base` (objets imbriqués compris).
fn merge(base: &mut Value, patch: &Value) {
    match (base, patch) {
        (Value::Object(b), Value::Object(p)) => {
            for (k, v) in p {
                merge(b.entry(k.clone()).or_insert(Value::Null), v);
            }
        }
        (b, p) => *b = p.clone(),
    }
}

/// `PATCH /api/me/preferences` : seuls les champs envoyés changent.
pub async fn patch_preferences(
    ctx: Ctx,
    State(state): State<AppState>,
    JsonBody(patch): JsonBody<Value>,
) -> ApiResult<Json<PreferencesResponse>> {
    let profile = ctx.account()?.clone();
    if !patch.is_object() {
        return Err(ApiError::validation("body", "objet JSON attendu"));
    }
    let mut value = serde_json::to_value(&profile.preferences).map_err(ApiError::internal)?;
    merge(&mut value, &patch);
    let prefs: Preferences = serde_json::from_value(value)
        .map_err(|e| ApiError::validation("preferences", e.to_string()))?;
    prefs.validate()?;
    let id = profile.id;
    let saved = prefs.clone();
    state
        .db(move |s| Ok(s.set_preferences(id, &saved)?))
        .await?;
    state.emit(Event::PreferencesChanged { profile_id: id });
    Ok(Json(PreferencesResponse { preferences: prefs }))
}

/// `GET /api/themes` : thèmes installés (interchangeables à chaud via
/// `preferences.theme`).
pub async fn themes(_: Ctx, State(state): State<AppState>) -> ApiResult<Json<ThemeList>> {
    let mut themes = Vec::new();
    if let Some(dir) = &state.config.themes_dir {
        if let Ok(read) = std::fs::read_dir(dir) {
            for e in read.flatten().filter(|e| e.path().is_dir()) {
                let id = e.file_name().to_string_lossy().into_owned();
                if id.starts_with('.') {
                    continue;
                }
                let name = std::fs::read_to_string(e.path().join("theme.json"))
                    .ok()
                    .and_then(|t| serde_json::from_str::<Value>(&t).ok())
                    .and_then(|v| v["name"].as_str().map(String::from))
                    .unwrap_or_else(|| id.clone());
                themes.push(ThemeView { id, name });
            }
        }
    }
    if themes.is_empty() {
        themes = vec![
            ThemeView {
                id: "default".into(),
                name: "Par défaut".into(),
            },
            ThemeView {
                id: "alternative".into(),
                name: "Alternatif".into(),
            },
        ];
    }
    themes.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(Json(ThemeList { themes }))
}
