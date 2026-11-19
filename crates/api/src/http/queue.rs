//! File d'attente (une seule, globale : celle de la télévision).

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use epikodi_domain::parental::Visibility;
use epikodi_domain::ports::{AppStateRepo, MediaRepo, QueueRepo};
use epikodi_domain::protocol::{
    EnqueueRequest, Event, MoveQueueItemRequest, QueueEntry, QueueView,
};
use epikodi_domain::queue::QueueMode;
use epikodi_domain::Id;
use epikodi_storage::Store;

use crate::auth::Ctx;
use crate::error::{ApiError, ApiResult};
use crate::extract::{JsonBody, PathId};
use crate::state::{lock, AppState};
use crate::views;

pub fn build_view(s: &Store, state: &AppState, vis: Visibility) -> ApiResult<QueueView> {
    let current = s.app_state()?.current_queue_item_id;
    let mut items = Vec::new();
    for it in s.queue()? {
        if let Some(m) = s.get_visible_media(it.media_id, vis)? {
            items.push(QueueEntry {
                id: it.id,
                position: it.position,
                media: views::media_item(&m),
            });
        }
    }
    let p = lock(&state.player);
    Ok(QueueView {
        items,
        current_item_id: current,
        shuffle: p.state.shuffle,
        repeat: p.state.repeat,
    })
}

pub async fn view(state: &AppState, vis: Visibility) -> ApiResult<QueueView> {
    let st = state.clone();
    state.db(move |s| build_view(s, &st, vis)).await
}

pub fn emit_changed(state: &AppState, q: &QueueView) {
    state.emit(Event::QueueChanged {
        current_item_id: q.current_item_id,
        length: q.items.len(),
    });
}

/// Médias → feuilles lisibles (un album donne ses morceaux, une série ses
/// épisodes), en vérifiant qu'ils sont visibles.
pub fn expand(s: &Store, media_ids: &[Id], vis: Visibility) -> ApiResult<Vec<Id>> {
    let mut out = Vec::new();
    for id in media_ids {
        let leaves = s.leaves_under(*id, vis).map_err(|e| match e {
            epikodi_domain::DomainError::NotFound { .. } => {
                ApiError::not_found("media_not_found", format!("Média {id} introuvable."))
            }
            e => e.into(),
        })?;
        out.extend(leaves.iter().map(|m| m.id));
    }
    Ok(out)
}

/// `GET /api/queue`
pub async fn get(ctx: Ctx, State(state): State<AppState>) -> ApiResult<Json<QueueView>> {
    Ok(Json(view(&state, ctx.visibility()?).await?))
}

/// `POST /api/queue` : `{"media_ids": [...], "mode": "append"|"next"|"replace"}`
pub async fn add(
    ctx: Ctx,
    State(state): State<AppState>,
    JsonBody(req): JsonBody<EnqueueRequest>,
) -> ApiResult<Json<QueueView>> {
    let vis = ctx.visibility()?;
    if req.media_ids.is_empty() || req.media_ids.len() > 500 {
        return Err(ApiError::validation("media_ids", "entre 1 et 500 médias"));
    }
    let st = state.clone();
    let q = state
        .db(move |s| {
            let ids = expand(s, &req.media_ids, vis)?;
            if ids.is_empty() {
                return Err(ApiError::conflict(
                    "nothing_to_play",
                    "Aucun média lisible.",
                ));
            }
            let current = s.app_state()?.current_queue_item_id;
            s.enqueue(&ids, req.mode, current)?;
            if req.mode == QueueMode::Replace {
                s.set_current_queue_item(None)?;
            }
            build_view(s, &st, vis)
        })
        .await?;
    emit_changed(&state, &q);
    Ok(Json(q))
}

/// `DELETE /api/queue` (204)
pub async fn clear(ctx: Ctx, State(state): State<AppState>) -> ApiResult<StatusCode> {
    ctx.profile()?;
    state
        .db(|s| {
            s.clear_queue()?;
            Ok(s.set_current_queue_item(None)?)
        })
        .await?;
    state.emit(Event::QueueChanged {
        current_item_id: None,
        length: 0,
    });
    Ok(StatusCode::NO_CONTENT)
}

fn item_not_found() -> ApiError {
    ApiError::not_found("queue_item_not_found", "Élément de la file introuvable.")
}

/// `DELETE /api/queue/{item_id}` (204)
pub async fn remove(
    ctx: Ctx,
    State(state): State<AppState>,
    PathId(id): PathId,
) -> ApiResult<StatusCode> {
    let vis = ctx.visibility()?;
    let st = state.clone();
    let q = state
        .db(move |s| {
            if !s.remove_queue_item(id)? {
                return Err(item_not_found());
            }
            build_view(s, &st, vis)
        })
        .await?;
    emit_changed(&state, &q);
    Ok(StatusCode::NO_CONTENT)
}

/// `PATCH /api/queue/{item_id}` : `{"position": 0}` (déplacer)
pub async fn move_item(
    ctx: Ctx,
    State(state): State<AppState>,
    PathId(id): PathId,
    JsonBody(req): JsonBody<MoveQueueItemRequest>,
) -> ApiResult<Json<QueueView>> {
    let vis = ctx.visibility()?;
    if req.position < 0 {
        return Err(ApiError::validation("position", "positive"));
    }
    let st = state.clone();
    let q = state
        .db(move |s| {
            if !s.move_queue_item(id, req.position)? {
                return Err(item_not_found());
            }
            build_view(s, &st, vis)
        })
        .await?;
    emit_changed(&state, &q);
    Ok(Json(q))
}
