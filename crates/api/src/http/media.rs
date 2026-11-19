//! Catalogue : listes filtrées par le profil actif, fiches, affiches,
//! progression, vu / non vu, correction manuelle.

use std::sync::Arc;

use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};

use epikodi_domain::media::{Media, MediaKind, MediaPatch, MediaQuery, MediaSort};
use epikodi_domain::parental::Visibility;
use epikodi_domain::playback::{apply_position, container_watched, next_episode, WatchProgress};
use epikodi_domain::ports::{FavoriteRepo, MediaRepo, ProgressRepo};
use epikodi_domain::protocol::{
    ApplyMatchRequest, ContinueItem, CountItem, GenreList, MatchCandidates, MediaDetailResponse,
    MediaItem, MediaItems, MediaList, MediaPatchRequest, ProgressRequest, ProgressView,
    WatchedRequest, YearList,
};
use epikodi_domain::Id;
use epikodi_library::manual_fix;
use epikodi_storage::Store;

use crate::auth::Ctx;
use crate::error::{ApiError, ApiResult};
use crate::extract::{JsonBody, PathId, QueryParams};
use crate::state::AppState;
use crate::views;

#[derive(Debug, Default, Deserialize)]
pub struct ListQuery {
    pub kind: Option<String>,
    pub parent_id: Option<String>,
    pub limit: Option<String>,
    pub offset: Option<String>,
    pub q: Option<String>,
    pub genre: Option<String>,
    pub year: Option<String>,
    pub sort: Option<String>,
}

fn parse<T: std::str::FromStr>(field: &str, v: &Option<String>) -> ApiResult<Option<T>> {
    v.as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.parse::<T>()
                .map_err(|_| ApiError::invalid_query(field, "valeur invalide"))
        })
        .transpose()
}

fn media_not_found() -> ApiError {
    ApiError::not_found("media_not_found", "Média introuvable.")
}

/// Média visible par le profil (404 sinon : on ne révèle pas qu'il existe).
fn visible(s: &Store, id: Id, vis: Visibility) -> ApiResult<Media> {
    s.get_visible_media(id, vis)?.ok_or_else(media_not_found)
}

/// `GET /api/media` : `kind`, `parent_id`, `limit` (50), `offset`, et en plus
/// `q` (recherche), `genre`, `year`, `sort` (`title`, `year`, `added`).
pub async fn list(
    ctx: Ctx,
    State(state): State<AppState>,
    QueryParams(q): QueryParams<ListQuery>,
) -> ApiResult<Json<MediaList>> {
    let vis = ctx.visibility()?;
    let limit: u32 = parse("limit", &q.limit)?.unwrap_or(MediaQuery::DEFAULT_LIMIT);
    if limit == 0 || limit > MediaQuery::MAX_LIMIT {
        return Err(ApiError::invalid_query(
            "limit",
            format!("entre 1 et {}", MediaQuery::MAX_LIMIT),
        ));
    }
    let offset: u32 = parse("offset", &q.offset)?.unwrap_or(0);
    let kind = q
        .kind
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|k| {
            k.parse::<MediaKind>()
                .map_err(|_| ApiError::invalid_query("kind", "type inconnu"))
        })
        .transpose()?;
    let sort = match q.sort.as_deref() {
        None | Some("") => MediaSort::Default,
        Some("title") => MediaSort::Title,
        Some("year") => MediaSort::Year,
        Some("added") => MediaSort::Added,
        Some(_) => return Err(ApiError::invalid_query("sort", "title, year ou added")),
    };
    let query = MediaQuery {
        kind,
        parent_id: parse("parent_id", &q.parent_id)?,
        search: q.q.clone().filter(|s| !s.trim().is_empty()),
        genre: q.genre.clone().filter(|s| !s.is_empty()),
        year: parse("year", &q.year)?,
        sort,
        limit,
        offset,
    };
    let page = state
        .db(move |s| {
            if let Some(pid) = query.parent_id {
                visible(s, pid, vis)?;
            }
            Ok(s.list_media(&query, vis)?)
        })
        .await?;
    Ok(Json(MediaList {
        items: views::media_items(&page.items),
        total: page.total,
        limit,
        offset,
    }))
}

/// `GET /api/media/genres`
pub async fn genres(ctx: Ctx, State(state): State<AppState>) -> ApiResult<Json<GenreList>> {
    let vis = ctx.visibility()?;
    let g = state.db(move |s| Ok(s.genres(vis)?)).await?;
    Ok(Json(GenreList {
        genres: g
            .into_iter()
            .map(|(value, count)| CountItem { value, count })
            .collect(),
    }))
}

/// `GET /api/media/years`
pub async fn years(ctx: Ctx, State(state): State<AppState>) -> ApiResult<Json<YearList>> {
    let vis = ctx.visibility()?;
    let y = state.db(move |s| Ok(s.years(vis)?)).await?;
    Ok(Json(YearList {
        years: y
            .into_iter()
            .map(|(value, count)| CountItem { value, count })
            .collect(),
    }))
}

#[derive(Debug, Default, Deserialize)]
pub struct LimitQuery {
    pub limit: Option<u32>,
}

/// `GET /api/media/recent` : derniers ajouts.
pub async fn recent(
    ctx: Ctx,
    State(state): State<AppState>,
    QueryParams(q): QueryParams<LimitQuery>,
) -> ApiResult<Json<MediaItems>> {
    let vis = ctx.visibility()?;
    let limit = q.limit.unwrap_or(20).clamp(1, 100);
    let list = state.db(move |s| Ok(s.recently_added(vis, limit)?)).await?;
    Ok(Json(MediaItems {
        items: views::media_items(&list),
    }))
}

/// Reprise de lecture du profil (vide pour l'invité).
pub async fn continue_items(
    ctx: &Ctx,
    state: &AppState,
    limit: u32,
) -> ApiResult<Vec<ContinueItem>> {
    let vis = ctx.visibility()?;
    let Some(user) = ctx.profile()?.user_id() else {
        return Ok(Vec::new());
    };
    let rows = state
        .db(move |s| Ok(s.in_progress(user, vis, limit)?))
        .await?;
    Ok(rows
        .iter()
        .map(|(m, p)| ContinueItem {
            media: views::media_item(m),
            progress: views::progress_view(p, m.duration_s),
        })
        .collect())
}

#[derive(serde::Serialize)]
pub struct ContinueList {
    pub items: Vec<ContinueItem>,
}

/// `GET /api/media/continue`
pub async fn continue_watching(
    ctx: Ctx,
    State(state): State<AppState>,
    QueryParams(q): QueryParams<LimitQuery>,
) -> ApiResult<Json<ContinueList>> {
    let items = continue_items(&ctx, &state, q.limit.unwrap_or(20).clamp(1, 100)).await?;
    Ok(Json(ContinueList { items }))
}

/// `GET /api/media/{id}` : carte, fiche détaillée, état du profil.
pub async fn detail(
    ctx: Ctx,
    State(state): State<AppState>,
    PathId(id): PathId,
) -> ApiResult<Json<MediaDetailResponse>> {
    let vis = ctx.visibility()?;
    let user = ctx.profile()?.user_id();
    let is_tv = ctx.auth.is_tv();
    let res = state
        .db(move |s| {
            let m = visible(s, id, vis)?;
            let details = s.get_details(id)?.unwrap_or_default();
            let mut ancestors = Vec::new();
            let mut parent = m.parent_id;
            while let Some(pid) = parent {
                let Some(p) = s.get_media(pid)? else { break };
                parent = p.parent_id;
                ancestors.insert(0, views::media_item(&p));
                if ancestors.len() > 3 {
                    break;
                }
            }
            let children_count = s.children(id, vis)?.len() as u64;
            let (progress, is_watched, is_favorite) = match user {
                Some(u) => {
                    let progress = if m.kind.is_leaf() {
                        s.get_progress(u, id)?
                    } else {
                        None
                    };
                    let watched = if m.kind.is_leaf() {
                        progress.as_ref().is_some_and(|p| p.is_watched)
                    } else {
                        let leaves: Vec<Id> =
                            s.leaves_under(id, vis)?.iter().map(|l| l.id).collect();
                        let rows = s.progress_for(u, &leaves)?;
                        let flags: Vec<bool> = leaves
                            .iter()
                            .map(|lid| rows.iter().any(|r| r.media_id == *lid && r.is_watched))
                            .collect();
                        container_watched(&flags)
                    };
                    (progress, watched, s.is_favorite(u, id)?)
                }
                None => (None, false, false),
            };
            let playback_uri = match (&m.uri, is_tv && m.kind.is_leaf()) {
                (Some(uri), true) => Some(
                    epikodi_sources::playback_path(uri)
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_else(|| uri.clone()),
                ),
                _ => None,
            };
            Ok(MediaDetailResponse {
                media: views::media_item(&m),
                details: serde_json::to_value(&details)
                    .unwrap_or(Value::Object(Default::default())),
                ancestors,
                children_count,
                progress: progress
                    .as_ref()
                    .map(|p| views::progress_view(p, m.duration_s)),
                is_watched,
                is_favorite,
                playback_uri,
            })
        })
        .await?;
    Ok(Json(res))
}

/// `GET /api/media/{id}/poster` : l'image du cache local (token requis).
pub async fn poster(
    ctx: Ctx,
    State(state): State<AppState>,
    PathId(id): PathId,
) -> ApiResult<Response> {
    let vis = ctx.visibility()?;
    let m = state.db(move |s| visible(s, id, vis)).await?;
    let not_found = || ApiError::not_found("poster_not_found", "Pas d'affiche pour ce média.");
    let path = m
        .poster_path
        .as_deref()
        .and_then(|p| state.images.resolve(p))
        .ok_or_else(not_found)?;
    let bytes = tokio::fs::read(&path).await.map_err(|_| not_found())?;
    let mime = match path.extension().and_then(|e| e.to_str()) {
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        _ => "image/jpeg",
    };
    Ok((
        [
            (header::CONTENT_TYPE, mime),
            (header::CACHE_CONTROL, "private, max-age=86400"),
        ],
        bytes,
    )
        .into_response())
}

/// Épisode suivant (même saison, puis saison suivante).
pub fn next_of(s: &Store, m: &Media, vis: Visibility) -> ApiResult<Option<Media>> {
    if m.kind != MediaKind::Episode {
        return Ok(None);
    }
    let Some(season) = m.parent_id.and_then(|p| s.get_media(p).ok().flatten()) else {
        return Ok(None);
    };
    let Some(series_id) = season.parent_id else {
        return Ok(None);
    };
    let mut seasons = Vec::new();
    for season in s.children(series_id, vis)? {
        let eps = s.children(season.id, vis)?;
        seasons.push((season, eps));
    }
    Ok(next_episode(&seasons, m.id).cloned())
}

/// `GET /api/media/{id}/next` → `{"media": ...|null}`
pub async fn next(
    ctx: Ctx,
    State(state): State<AppState>,
    PathId(id): PathId,
) -> ApiResult<Json<Value>> {
    let vis = ctx.visibility()?;
    let n = state
        .db(move |s| {
            let m = visible(s, id, vis)?;
            next_of(s, &m, vis)
        })
        .await?;
    Ok(Json(json!({ "media": n.as_ref().map(views::media_item) })))
}

/// `PATCH /api/media/{id}` : correction manuelle (compte non restreint).
pub async fn edit(
    ctx: Ctx,
    State(state): State<AppState>,
    PathId(id): PathId,
    JsonBody(req): JsonBody<MediaPatchRequest>,
) -> ApiResult<Json<MediaItem>> {
    ctx.admin()?;
    let m = state
        .db(move |s| {
            manual_fix::edit(
                s,
                id,
                MediaPatch {
                    title: req.title,
                    year: req.year,
                    age_rating: req.age_rating,
                    sort_number: req.sort_number,
                    ..Default::default()
                },
            )?;
            s.get_media(id)?.ok_or_else(media_not_found)
        })
        .await?;
    Ok(Json(views::media_item(&m)))
}

#[derive(Debug, Default, Deserialize)]
pub struct MatchQuery {
    pub q: Option<String>,
    pub year: Option<i32>,
}

/// `GET /api/media/{id}/matches?q=&year=` : résultats proposés.
pub async fn matches(
    ctx: Ctx,
    State(state): State<AppState>,
    PathId(id): PathId,
    QueryParams(q): QueryParams<MatchQuery>,
) -> ApiResult<Json<MatchCandidates>> {
    ctx.admin()?;
    let st = state.clone();
    // Connexion séparée : les appels réseau ne bloquent pas la base de l'API.
    let candidates = tokio::task::spawn_blocking(move || -> ApiResult<_> {
        let store = st.open_background_store()?;
        manual_fix::candidates(&store, &st.providers, id, q.q.as_deref(), q.year).map_err(|e| {
            match e {
                epikodi_domain::DomainError::Rule(m) => {
                    ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "provider_unavailable", m)
                }
                e => e.into(),
            }
        })
    })
    .await
    .map_err(ApiError::internal)??;
    Ok(Json(MatchCandidates { candidates }))
}

/// `POST /api/media/{id}/match` : applique le résultat choisi (`manual`).
pub async fn apply_match(
    ctx: Ctx,
    State(state): State<AppState>,
    PathId(id): PathId,
    JsonBody(req): JsonBody<ApplyMatchRequest>,
) -> ApiResult<Json<MediaItem>> {
    ctx.admin()?;
    let st = state.clone();
    let m = tokio::task::spawn_blocking(move || -> ApiResult<Media> {
        let store = st.open_background_store()?;
        let images: Arc<dyn epikodi_domain::ports::ImageCache> = st.images.clone();
        manual_fix::apply(
            &store,
            &st.providers,
            images.as_ref(),
            id,
            &req.provider,
            &req.external_id,
        )?;
        store.get_media(id)?.ok_or_else(media_not_found)
    })
    .await
    .map_err(ApiError::internal)??;
    Ok(Json(views::media_item(&m)))
}

/// `PUT /api/media/{id}/progress` : position de lecture du compte actif.
pub async fn progress(
    ctx: Ctx,
    State(state): State<AppState>,
    PathId(id): PathId,
    JsonBody(req): JsonBody<ProgressRequest>,
) -> ApiResult<Json<ProgressView>> {
    let vis = ctx.visibility()?;
    let user = ctx.account()?.id;
    if req.position_s < 0 {
        return Err(ApiError::validation("position_s", "positive"));
    }
    let at = state.now_iso();
    let view = state
        .db(move |s| {
            let m = visible(s, id, vis)?;
            if !m.kind.is_leaf() {
                return Err(ApiError::validation(
                    "media_id",
                    "seul un média lisible a une progression",
                ));
            }
            let duration = req.duration_s.or(m.duration_s);
            let before = s.get_progress(user, id)?;
            let (position_s, is_watched) = apply_position(
                req.position_s,
                duration,
                before.as_ref().is_some_and(|p| p.is_watched),
            );
            let p = WatchProgress {
                media_id: id,
                position_s,
                is_watched,
                last_played_at: Some(at),
            };
            s.save_progress(user, &p)?;
            Ok(views::progress_view(&p, duration))
        })
        .await?;
    Ok(Json(view))
}

/// `PUT /api/media/{id}/watched` : vu / non vu (une série : tous ses épisodes).
pub async fn watched(
    ctx: Ctx,
    State(state): State<AppState>,
    PathId(id): PathId,
    JsonBody(req): JsonBody<WatchedRequest>,
) -> ApiResult<Json<Value>> {
    let vis = ctx.visibility()?;
    let user = ctx.account()?.id;
    let at = state.now_iso();
    let count = state
        .db(move |s| {
            let leaves: Vec<Id> = s.leaves_under(id, vis)?.iter().map(|m| m.id).collect();
            s.set_watched(user, &leaves, req.watched, &at)?;
            Ok(leaves.len())
        })
        .await?;
    Ok(Json(json!({ "is_watched": req.watched, "updated": count })))
}
