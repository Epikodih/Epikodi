//! Session : appareil courant et profil actif.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use epikodi_domain::ports::{DeviceRepo, ProfileRepo};
use epikodi_domain::protocol::{ProfileResponse, SelectKind, SelectProfileRequest, SessionCurrent};

use crate::auth::{check_pin, Auth, Ctx};
use crate::error::{ApiError, ApiResult};
use crate::extract::JsonBody;
use crate::http::profiles::activate;
use crate::state::{lock, AppState, Selection};
use crate::views;

/// `GET /api/session`
pub async fn current(ctx: Ctx) -> Json<SessionCurrent> {
    Json(SessionCurrent {
        device: ctx.auth.device().map(views::session_device),
        profile: ctx.profile.as_ref().map(views::profile_view),
    })
}

/// `POST /api/session/profile` : invité, compte restreint, ou compte non
/// restreint avec son PIN.
pub async fn select(
    _: Auth,
    State(state): State<AppState>,
    JsonBody(req): JsonBody<SelectProfileRequest>,
) -> ApiResult<Json<ProfileResponse>> {
    match req.kind {
        SelectKind::Guest => {
            if req.profile_id.is_some() || req.pin.is_some() {
                return Err(ApiError::validation(
                    "type",
                    "un invité n'a ni profile_id ni pin",
                ));
            }
            activate(&state, Selection::Guest).await?;
            Ok(Json(ProfileResponse {
                profile: views::guest_view(),
            }))
        }
        SelectKind::Account => {
            let id = req
                .profile_id
                .ok_or_else(|| ApiError::validation("profile_id", "obligatoire"))?;
            let profile = state
                .db(move |s| {
                    s.get_profile(id)?.ok_or_else(|| {
                        ApiError::not_found("profile_not_found", "Profil introuvable.")
                    })
                })
                .await?;
            check_pin(&state, &profile, req.pin.as_deref()).await?;
            activate(&state, Selection::Account(id)).await?;
            Ok(Json(ProfileResponse {
                profile: views::account_view(&profile),
            }))
        }
    }
}

/// `DELETE /api/session/profile` : retour au choix du profil.
pub async fn clear_profile(_: Auth, State(state): State<AppState>) -> ApiResult<StatusCode> {
    activate(&state, Selection::None).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /api/session` : la télécommande se déconnecte (son jeton est
/// révoqué ; il faudra refaire l'appairage).
pub async fn logout(auth: Auth, State(state): State<AppState>) -> ApiResult<StatusCode> {
    if let Some(d) = auth.device() {
        let id = d.id;
        let at = state.now_iso();
        state.db(move |s| Ok(s.revoke_device(id, &at)?)).await?;
        lock(&state.pairing).detach_device(id);
        state.notify_tv();
    }
    Ok(StatusCode::NO_CONTENT)
}
