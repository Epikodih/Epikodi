//! Appareils appairés : liste et révocation (appareil perdu...).

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use epikodi_domain::ports::DeviceRepo;
use epikodi_domain::protocol::DeviceList;

use crate::auth::Ctx;
use crate::error::{ApiError, ApiResult};
use crate::extract::PathId;
use crate::state::{lock, AppState};
use crate::views;

/// `GET /api/devices` (compte non restreint)
pub async fn list(ctx: Ctx, State(state): State<AppState>) -> ApiResult<Json<DeviceList>> {
    ctx.admin()?;
    let current = ctx.auth.device().map(|d| d.id);
    let devices = state.db(|s| Ok(s.list_devices()?)).await?;
    Ok(Json(DeviceList {
        devices: devices
            .iter()
            .map(|d| views::device_view(d, current))
            .collect(),
    }))
}

/// `DELETE /api/devices/{id}` : soi-même, ou n'importe lequel pour un adulte.
pub async fn revoke(
    ctx: Ctx,
    State(state): State<AppState>,
    PathId(id): PathId,
) -> ApiResult<StatusCode> {
    let is_self = ctx.auth.device().is_some_and(|d| d.id == id);
    if !is_self {
        ctx.admin()?;
    }
    let at = state.now_iso();
    state
        .db(move |s| {
            s.get_device(id)?
                .filter(|d| d.revoked_at.is_none())
                .ok_or_else(|| ApiError::not_found("device_not_found", "Appareil introuvable."))?;
            Ok(s.revoke_device(id, &at)?)
        })
        .await?;
    lock(&state.pairing).detach_device(id);
    state.notify_tv();
    Ok(StatusCode::NO_CONTENT)
}
