//! Routes de l'app TV (accès local) et appairage de la télécommande.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use epikodi_domain::device::NewDevice;
use epikodi_domain::ports::{DeviceRepo, ProfileRepo};
use epikodi_domain::protocol::{
    Event, PairRequest, PairResponse, PairingInfo, QrInfo, TvConnectRequest, TvConnectResponse,
    TvPhase, TvRefreshRequest, TvRefreshResponse, TvStatus,
};
use epikodi_storage::secrets::hash_token;

use crate::auth::pairing::{PairFailure, PairingCode};
use crate::auth::{new_token, LocalOnly};
use crate::error::{ApiError, ApiResult};
use crate::extract::{JsonBody, QueryParams};
use crate::state::{iso, lock, AppState};
use crate::views;

/// Schéma provisoire de l'URL encodée dans le QR code.
pub const QR_SCHEME: &str = "mediacenter";

fn lan_ip(state: &AppState) -> String {
    state
        .config
        .lan_host
        .clone()
        .unwrap_or_else(|| "127.0.0.1".into())
}

/// Le QR localise le serveur ; il ne contient JAMAIS le code.
fn pairing_info(state: &AppState, code: &PairingCode) -> PairingInfo {
    let now = state.now();
    PairingInfo {
        code: code.code.clone(),
        expires_at: iso(code.expires_at),
        expires_in_s: (code.expires_at - now).num_seconds().max(0),
        attempts_left: code.attempts_left,
        qr: QrInfo {
            payload: format!(
                "{QR_SCHEME}://pair?host={}&port={}&sid={}",
                lan_ip(state),
                state.config.port,
                code.session_id
            ),
            format: "text".into(),
        },
    }
}

fn check_text(field: &str, value: &str, max: usize) -> ApiResult<()> {
    if value.trim().is_empty() || value.chars().count() > max {
        return Err(ApiError::validation(
            field,
            format!("entre 1 et {max} caractères"),
        ));
    }
    Ok(())
}

fn session_not_found() -> ApiError {
    ApiError::not_found(
        "tv_session_not_found",
        "Session TV inconnue (serveur redémarré) : refaites POST /api/tv/connect.",
    )
}

/// `POST /api/tv/connect` (201) : nouvelle session TV + premier code + QR.
pub async fn connect(
    _: LocalOnly,
    State(state): State<AppState>,
    JsonBody(req): JsonBody<TvConnectRequest>,
) -> ApiResult<(StatusCode, Json<TvConnectResponse>)> {
    if !state.is_ready() {
        return Err(ApiError::starting());
    }
    check_text("model", &req.model, 100)?;
    check_text("platform", &req.platform, 32)?;
    check_text("app_version", &req.app_version, 32)?;
    let now = state.now();
    let (session, code) =
        lock(&state.pairing).connect(now, &req.model, &req.platform, &req.app_version);
    tracing::info!(session = %session.id, model = %req.model, "app TV connectée");
    state.notify_tv();
    Ok((
        StatusCode::CREATED,
        Json(TvConnectResponse {
            tv_session_id: session.id,
            lan_ip: lan_ip(&state),
            pairing: pairing_info(&state, &code),
        }),
    ))
}

/// `POST /api/tv/pairing/refresh` : nouveau code pour la session.
pub async fn refresh(
    _: LocalOnly,
    State(state): State<AppState>,
    JsonBody(req): JsonBody<TvRefreshRequest>,
) -> ApiResult<Json<TvRefreshResponse>> {
    check_text("tv_session_id", &req.tv_session_id, 64)?;
    let code = lock(&state.pairing)
        .refresh(state.now(), &req.tv_session_id)
        .ok_or_else(session_not_found)?;
    state.notify_tv();
    Ok(Json(TvRefreshResponse {
        pairing: pairing_info(&state, &code),
    }))
}

#[derive(Debug, Deserialize)]
pub struct StatusQuery {
    pub tv_session_id: Option<String>,
}

/// État affiché par la TV : `pairing` → `paired` → `ready`.
pub async fn tv_status(state: &AppState, session_id: Option<&str>) -> ApiResult<TvStatus> {
    let session = match session_id {
        Some(id) => Some(
            lock(&state.pairing)
                .session(id)
                .cloned()
                .ok_or_else(session_not_found)?,
        ),
        None => None,
    };
    let active = state.active_profile().await?;
    let profiles = state.db(|s| Ok(s.list_profiles()?)).await?;
    let device = session.as_ref().and_then(|s| s.device.clone());
    let phase = match (&active, &device) {
        (Some(_), _) => TvPhase::Ready,
        (None, Some(_)) => TvPhase::Paired,
        (None, None) => TvPhase::Pairing,
    };
    let pairing = match (&phase, &session) {
        (TvPhase::Pairing, Some(s)) => lock(&state.pairing)
            .code_for(state.now(), &s.id)
            .map(|c| pairing_info(state, &c)),
        _ => None,
    };
    Ok(TvStatus {
        phase,
        pairing,
        device,
        can_create: !active.as_ref().is_some_and(|a| a.is_guest()),
        active_profile: active.as_ref().map(views::profile_view),
        guest: views::guest_info(),
        profiles: profiles.iter().map(views::profile_list_item).collect(),
    })
}

/// `GET /api/tv/status?tv_session_id=...`
pub async fn status(
    _: LocalOnly,
    State(state): State<AppState>,
    QueryParams(q): QueryParams<StatusQuery>,
) -> ApiResult<Json<TvStatus>> {
    Ok(Json(tv_status(&state, q.tv_session_id.as_deref()).await?))
}

/// `POST /api/remote/pair` : public. Le code prouve que l'utilisateur est
/// devant la TV ; en échange, un jeton de session.
pub async fn pair(
    State(state): State<AppState>,
    JsonBody(req): JsonBody<PairRequest>,
) -> ApiResult<Json<PairResponse>> {
    check_text("device.name", &req.device.name, 64)?;
    if req
        .device
        .install_id
        .as_deref()
        .is_some_and(|i| i.len() > 128)
    {
        return Err(ApiError::validation(
            "device.install_id",
            "128 caractères maximum",
        ));
    }
    if req.device.platform.as_deref().is_some_and(|p| p.len() > 32) {
        return Err(ApiError::validation(
            "device.platform",
            "32 caractères maximum",
        ));
    }
    if !state.is_ready() {
        return Err(ApiError::starting());
    }

    let now = state.now();
    let verified = lock(&state.pairing).verify(now, &req.code);
    if let Err(failure) = verified {
        state.notify_tv(); // un code a pu être renouvelé
        return Err(match failure {
            PairFailure::Invalid { attempts_left } => {
                let failures = state
                    .config
                    .pairing
                    .max_attempts
                    .saturating_sub(attempts_left);
                state.failure_pause(failures).await;
                ApiError::new(
                    StatusCode::FORBIDDEN,
                    "pairing_code_invalid",
                    "Code d'appairage incorrect.",
                )
                .with_details(json!({ "attempts_left": attempts_left }))
            }
            PairFailure::Expired => ApiError::new(
                StatusCode::GONE,
                "pairing_code_expired",
                "Code expiré : la TV affiche un nouveau code.",
            ),
            PairFailure::TooManyAttempts { retry_after_s } => ApiError::new(
                StatusCode::TOO_MANY_REQUESTS,
                "pairing_too_many_attempts",
                "Trop de tentatives : un nouveau code est affiché sur la TV.",
            )
            .with_details(json!({ "retry_after_s": retry_after_s })),
        });
    }

    let token = new_token();
    let expires = now + chrono::Duration::from_std(state.config.token_ttl).unwrap_or_default();
    let new = NewDevice {
        name: req.device.name.trim().to_string(),
        install_id: req.device.install_id.clone(),
        platform: req.device.platform.clone(),
        token_hash: hash_token(&token),
        expires_at: Some(iso(expires)),
    };
    let at = state.now_iso();
    let device = state
        .db(move |s| {
            // Ré-appairage de la même installation : l'ancien jeton est révoqué.
            if let Some(install) = &new.install_id {
                s.revoke_install(install, &at)?;
            }
            Ok(s.create_device(&new)?)
        })
        .await?;
    let summary = views::device_summary(&device);
    lock(&state.pairing).attach_device(summary.clone());
    tracing::info!(device = device.id, name = %device.name, "appareil appairé");
    state.emit(Event::PairingCompleted(summary.clone()));
    state.notify_tv();
    Ok(Json(PairResponse {
        token,
        token_type: "Bearer".into(),
        expires_at: device.expires_at.clone(),
        device: summary,
    }))
}
