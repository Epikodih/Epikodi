//! Extensions : catalogue local, installation (permissions acceptées),
//! réglages, activation, mise à jour, désinstallation — à chaud.

use std::path::PathBuf;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use epikodi_domain::extension::{Extension, Manifest};
use epikodi_domain::ports::ExtensionRepo;
use epikodi_domain::protocol::{
    CatalogEntry, Event, ExtensionCatalog, ExtensionList, ExtensionView, InstallExtensionRequest,
    UpdateExtensionRequest,
};
use epikodi_extensions::lifecycle::ExtensionManager;

use crate::auth::Ctx;
use crate::error::{ApiError, ApiResult};
use crate::extract::JsonBody;
use crate::state::AppState;
use crate::views;

fn catalog_entry(
    path: &std::path::Path,
    m: &Manifest,
    installed: Option<&Extension>,
) -> CatalogEntry {
    CatalogEntry {
        id: m.id.clone(),
        name: m.name.clone(),
        version: m.version.clone(),
        author: m.author.clone(),
        description: m.description.clone(),
        kind: m.kind,
        permissions: views::permission_views(&m.permissions, |_| false),
        path: path.to_string_lossy().into_owned(),
        installed_version: installed.map(|e| e.version.clone()),
    }
}

fn notify(state: &AppState, e: &Extension) {
    state.emit(Event::ExtensionStatus {
        extension_id: e.id.clone(),
        status: e.status,
        message: e.last_error.clone(),
    });
}

/// `GET /api/extensions`
pub async fn list(ctx: Ctx, State(state): State<AppState>) -> ApiResult<Json<ExtensionList>> {
    ctx.profile()?;
    let list = state.db(|s| Ok(s.list_extensions()?)).await?;
    Ok(Json(ExtensionList {
        extensions: list.iter().map(views::extension_view).collect(),
    }))
}

/// `GET /api/extensions/{id}`
pub async fn get(
    ctx: Ctx,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<ExtensionView>> {
    ctx.profile()?;
    let e = state
        .db(move |s| {
            s.get_extension(&id)?
                .ok_or_else(|| ApiError::not_found("extension_not_found", "Extension introuvable."))
        })
        .await?;
    Ok(Json(views::extension_view(&e)))
}

/// `GET /api/extensions/catalog` : paquets du catalogue local.
pub async fn catalog(ctx: Ctx, State(state): State<AppState>) -> ApiResult<Json<ExtensionCatalog>> {
    ctx.admin()?;
    let Some(dir) = state.config.extensions_catalog.clone() else {
        return Ok(Json(ExtensionCatalog { extensions: vec![] }));
    };
    let install_dir = state.config.extensions_dir();
    let entries = state
        .db(move |s| {
            let mgr = ExtensionManager::new(s, install_dir);
            let installed = s.list_extensions()?;
            Ok(mgr
                .catalog(&dir)
                .iter()
                .map(|(p, m)| catalog_entry(p, m, installed.iter().find(|e| e.id == m.id)))
                .collect())
        })
        .await?;
    Ok(Json(ExtensionCatalog {
        extensions: entries,
    }))
}

#[derive(Debug, Deserialize)]
pub struct PathRequest {
    pub path: String,
}

fn package_path(raw: &str) -> ApiResult<PathBuf> {
    let p = PathBuf::from(raw.trim());
    if raw.trim().is_empty() || !p.exists() {
        return Err(ApiError::validation(
            "path",
            "dossier ou .zip introuvable sur le media center",
        ));
    }
    Ok(p)
}

/// `POST /api/extensions/inspect` : ce que demande un paquet, avant d'accepter.
pub async fn inspect(
    ctx: Ctx,
    State(state): State<AppState>,
    JsonBody(req): JsonBody<PathRequest>,
) -> ApiResult<Json<CatalogEntry>> {
    ctx.admin()?;
    let path = package_path(&req.path)?;
    let install_dir = state.config.extensions_dir();
    let entry = state
        .db(move |s| {
            let pkg = ExtensionManager::new(s, install_dir).inspect(&path)?;
            let installed = s.get_extension(&pkg.manifest.id)?;
            Ok(catalog_entry(&path, &pkg.manifest, installed.as_ref()))
        })
        .await?;
    Ok(Json(entry))
}

/// `POST /api/extensions` (201) : installe, avec les permissions acceptées.
pub async fn install(
    ctx: Ctx,
    State(state): State<AppState>,
    JsonBody(req): JsonBody<InstallExtensionRequest>,
) -> ApiResult<(StatusCode, Json<ExtensionView>)> {
    ctx.admin()?;
    let path = package_path(&req.path)?;
    let install_dir = state.config.extensions_dir();
    let e = state
        .db(move |s| {
            Ok(ExtensionManager::new(s, install_dir).install(&path, &req.accept_permissions)?)
        })
        .await?;
    notify(&state, &e);
    Ok((StatusCode::CREATED, Json(views::extension_view(&e))))
}

/// `POST /api/extensions/{id}/update` : nouvelle version.
pub async fn update_package(
    ctx: Ctx,
    State(state): State<AppState>,
    Path(id): Path<String>,
    JsonBody(req): JsonBody<InstallExtensionRequest>,
) -> ApiResult<Json<ExtensionView>> {
    ctx.admin()?;
    let path = package_path(&req.path)?;
    let install_dir = state.config.extensions_dir();
    let e =
        state
            .db(move |s| {
                Ok(ExtensionManager::new(s, install_dir).update(
                    &id,
                    &path,
                    &req.accept_permissions,
                )?)
            })
            .await?;
    notify(&state, &e);
    Ok(Json(views::extension_view(&e)))
}

/// `PATCH /api/extensions/{id}` : activer / désactiver, réglages.
pub async fn update(
    ctx: Ctx,
    State(state): State<AppState>,
    Path(id): Path<String>,
    JsonBody(req): JsonBody<UpdateExtensionRequest>,
) -> ApiResult<Json<ExtensionView>> {
    ctx.admin()?;
    let install_dir = state.config.extensions_dir();
    let e = state
        .db(move |s| {
            let mgr = ExtensionManager::new(s, install_dir);
            let mut e = s.get_extension(&id)?.ok_or_else(|| {
                ApiError::not_found("extension_not_found", "Extension introuvable.")
            })?;
            if let Some(settings) = &req.settings {
                e = mgr.set_settings(&id, settings)?;
            }
            if let Some(enabled) = req.enabled {
                e = mgr.set_enabled(&id, enabled)?;
            }
            Ok(e)
        })
        .await?;
    notify(&state, &e);
    Ok(Json(views::extension_view(&e)))
}

/// `DELETE /api/extensions/{id}` (204)
pub async fn uninstall(
    ctx: Ctx,
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    ctx.admin()?;
    let install_dir = state.config.extensions_dir();
    state
        .db(move |s| Ok(ExtensionManager::new(s, install_dir).uninstall(&id)?))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
