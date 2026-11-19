//! Comptes : liste (page des comptes), création, lecture, modification,
//! suppression. Le statut (invité, restreint, non restreint) est toujours
//! vérifié ici, jamais déduit de ce qu'affiche l'app.

use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;

use epikodi_domain::parental::check_pin_format;
use epikodi_domain::ports::{AppStateRepo, ProfileRepo};
use epikodi_domain::profile::{
    check_age, check_icon, check_language, check_pin_for_restriction, normalize_name,
    ActiveProfile, NewProfile, Profile,
};
use epikodi_domain::protocol::{
    CreateProfileRequest, CreateProfileResponse, DeleteProfileRequest, Event, GetProfileResponse,
    ProfileResponse, ProfilesList, UpdateProfileRequest,
};
use epikodi_storage::secrets::hash_pin;

use crate::auth::{check_pin, Ctx};
use crate::error::{ApiError, ApiResult};
use crate::extract::{JsonBody, PathId};
use crate::state::{AppState, Selection};
use crate::views;

/// `GET /api/profiles` : tout ce qu'il faut pour la page des comptes.
pub async fn list(ctx: Ctx, State(state): State<AppState>) -> ApiResult<Json<ProfilesList>> {
    let profiles = state.db(|s| Ok(s.list_profiles()?)).await?;
    Ok(Json(ProfilesList {
        guest: views::guest_info(),
        can_create: !ctx.profile.as_ref().is_some_and(ActiveProfile::is_guest),
        profiles: profiles.iter().map(views::profile_list_item).collect(),
    }))
}

async fn any_admin_account(state: &AppState) -> ApiResult<bool> {
    state
        .db(|s| Ok(s.list_profiles()?.iter().any(|p| !p.is_restricted)))
        .await
}

/// Rend un profil actif (base + mémoire + événements).
pub async fn activate(state: &AppState, selection: Selection) -> ApiResult<()> {
    let user = match selection {
        Selection::Account(id) => Some(id),
        _ => None,
    };
    state.db(move |s| Ok(s.set_active_user(user)?)).await?;
    state.set_selection(selection);
    let profile = state.active_profile().await?;
    state.emit(Event::ProfileChanged {
        profile: profile.as_ref().map(views::profile_view),
    });
    Ok(())
}

/// `POST /api/profiles` (201) : le compte créé devient actif.
pub async fn create(
    ctx: Ctx,
    State(state): State<AppState>,
    JsonBody(req): JsonBody<CreateProfileRequest>,
) -> ApiResult<(StatusCode, Json<CreateProfileResponse>)> {
    if ctx.profile.as_ref().is_some_and(ActiveProfile::is_guest) {
        return Err(ApiError::guest_forbidden());
    }
    let name = normalize_name(&req.name)?;
    check_icon(req.icon.as_deref())?;
    let language = req.language.clone().unwrap_or_else(|| "fr".into());
    check_language(&language)?;
    let age = check_age(req.age)?;
    check_pin_for_restriction(req.is_restricted, req.pin.as_deref())?;
    // Un profil restreint ne peut pas se créer un compte non restreint
    // (sinon le contrôle parental ne sert à rien).
    let active_is_admin = ctx.profile.as_ref().is_some_and(ActiveProfile::is_admin);
    if !req.is_restricted && !active_is_admin && any_admin_account(&state).await? {
        return Err(ApiError::admin_required());
    }

    let pin = req.pin.clone();
    let new_profile = state
        .db(move |s| {
            let pin_hash = pin.as_deref().map(hash_pin).transpose()?;
            // Créé et rendu actif ensemble : jamais de compte créé « à moitié ».
            s.transaction(|_| {
                let p = s.create_profile(&NewProfile {
                    name,
                    icon: req.icon.clone(),
                    language,
                    age,
                    is_restricted: req.is_restricted,
                    pin_hash,
                })?;
                s.set_active_user(Some(p.id))?;
                Ok::<_, ApiError>(p)
            })
        })
        .await?;
    tracing::info!(profile = new_profile.id, "compte créé");
    state.set_selection(Selection::Account(new_profile.id));
    state.emit(Event::ProfileChanged {
        profile: Some(views::account_view(&new_profile)),
    });
    Ok((
        StatusCode::CREATED,
        Json(CreateProfileResponse {
            profile: views::account_view(&new_profile),
            active: true,
        }),
    ))
}

async fn load(state: &AppState, id: i64) -> ApiResult<Profile> {
    state
        .db(move |s| {
            s.get_profile(id)?
                .ok_or_else(|| ApiError::not_found("profile_not_found", "Profil introuvable."))
        })
        .await
}

/// `GET /api/profiles/{id}`
pub async fn get(
    _: Ctx,
    State(state): State<AppState>,
    PathId(id): PathId,
) -> ApiResult<Json<GetProfileResponse>> {
    let p = load(&state, id).await?;
    Ok(Json(GetProfileResponse {
        profile: views::profile_detail(&p),
    }))
}

/// Peut-on toucher à la restriction d'âge de `target` ?
async fn authorize_restriction_change(
    state: &AppState,
    ctx: &Ctx,
    target: &Profile,
    pin: Option<&str>,
) -> ApiResult<()> {
    if target.pin_hash.is_some() {
        // compte non restreint : son propre PIN
        return check_pin(state, target, pin).await;
    }
    // compte restreint : il faut qu'un adulte soit connecté (s'il en existe)
    let admin_active = ctx.profile.as_ref().is_some_and(ActiveProfile::is_admin);
    if admin_active || !any_admin_account(state).await? {
        Ok(())
    } else if pin.is_none() {
        Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "pin_required",
            "Un compte adulte doit autoriser ce changement.",
        ))
    } else {
        Err(ApiError::admin_required())
    }
}

/// `PATCH /api/profiles/{id}` : le PIN est obligatoire pour toucher à la
/// restriction d'âge (`age`, `is_restricted`) ou changer de PIN.
pub async fn update(
    ctx: Ctx,
    State(state): State<AppState>,
    PathId(id): PathId,
    JsonBody(req): JsonBody<UpdateProfileRequest>,
) -> ApiResult<Json<ProfileResponse>> {
    let active = ctx.profile()?;
    if active.is_guest() {
        return Err(ApiError::guest_forbidden());
    }
    let current = load(&state, id).await?;
    // un profil restreint ne modifie que lui-même
    if let ActiveProfile::Account(me) = active {
        if me.is_restricted && me.id != id {
            return Err(ApiError::admin_required());
        }
    }

    let mut next = current.clone();
    if let Some(n) = &req.name {
        next.name = normalize_name(n)?;
    }
    if let Some(i) = &req.icon {
        check_icon(Some(i))?;
        next.icon = Some(i.clone());
    }
    if let Some(l) = &req.language {
        check_language(l)?;
        next.language = l.clone();
    }
    if let Some(a) = req.age {
        next.age = check_age(a)?;
    }
    if let Some(r) = req.is_restricted {
        next.is_restricted = r;
    }
    if let Some(p) = &req.new_pin {
        check_pin_format(p).map_err(|_| ApiError::validation("new_pin", "4 chiffres"))?;
    }

    let touches_restriction = next.age != current.age
        || next.is_restricted != current.is_restricted
        || req.new_pin.is_some();
    if touches_restriction {
        authorize_restriction_change(&state, &ctx, &current, req.pin.as_deref()).await?;
    }

    // PIN résultant
    let new_pin = if next.is_restricted {
        None
    } else if let Some(p) = req.new_pin.clone() {
        Some(p)
    } else if current.pin_hash.is_none() {
        // devient non restreint : il lui faut un PIN
        let p = req.pin.clone().ok_or_else(|| {
            ApiError::validation("new_pin", "obligatoire pour un compte non restreint")
        })?;
        check_pin_format(&p).map_err(|_| ApiError::validation("pin", "4 chiffres"))?;
        Some(p)
    } else {
        None // inchangé
    };
    let keep_hash = !next.is_restricted && new_pin.is_none();

    let updated = state
        .db(move |s| {
            next.pin_hash = match (new_pin, keep_hash) {
                (Some(p), _) => Some(hash_pin(&p)?),
                (None, true) => current.pin_hash.clone(),
                (None, false) => None,
            };
            s.update_profile(&next)?;
            Ok(next)
        })
        .await?;
    if state.selection() == Selection::Account(id) {
        state.emit(Event::ProfileChanged {
            profile: Some(views::account_view(&updated)),
        });
        state.notify_tv();
    }
    Ok(Json(ProfileResponse {
        profile: views::account_view(&updated),
    }))
}

/// `DELETE /api/profiles/{id}` (corps facultatif `{"pin": "1234"}`).
pub async fn delete(
    ctx: Ctx,
    State(state): State<AppState>,
    PathId(id): PathId,
    body: Bytes,
) -> ApiResult<StatusCode> {
    let active = ctx.profile()?;
    if active.is_guest() {
        return Err(ApiError::guest_forbidden());
    }
    let req: DeleteProfileRequest = if body.is_empty() {
        DeleteProfileRequest::default()
    } else {
        serde_json::from_slice(&body).map_err(|_| ApiError::validation("body", "JSON invalide"))?
    };
    let target = load(&state, id).await?;
    if target.pin_hash.is_some() {
        check_pin(&state, &target, req.pin.as_deref()).await?;
    } else if !active.is_admin() && active.user_id() != Some(id) {
        return Err(ApiError::admin_required());
    }
    state.db(move |s| Ok(s.delete_profile(id)?)).await?;
    if state.selection() == Selection::Account(id) {
        activate(&state, Selection::None).await?;
    }
    state.notify_tv();
    Ok(StatusCode::NO_CONTENT)
}
