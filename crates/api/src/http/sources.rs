//! Sources (dossiers locaux, partages réseau) et scan de la médiathèque.
//!
//! Le scan tourne en tâche de fond, sur sa propre connexion à la base
//! (SQLite en WAL : l'API continue de répondre pendant ce temps).

use std::time::{Duration, Instant};

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;

use epikodi_domain::ports::{ImageCache, LibraryRepo, NewSource, SourceRepo};
use epikodi_domain::protocol::{
    CreateSourceRequest, DiscoveredSourceView, DiscoveredSources, Event, ScanStatus, SourceList,
    SourceView, UpdateSourceRequest,
};
use epikodi_domain::source::{check_source_uri, Protocol, Source, SourceStatus};
use epikodi_domain::Id;
use epikodi_library::enrichment::Enricher;
use epikodi_library::scanner::Scanner;

use crate::auth::Ctx;
use crate::error::{ApiError, ApiResult};
use crate::extract::{JsonBody, PathId, QueryParams};
use crate::state::{lock, AppState};
use crate::views;

fn source_not_found() -> ApiError {
    ApiError::not_found("source_not_found", "Source introuvable.")
}

/// `GET /api/sources`
pub async fn list(ctx: Ctx, State(state): State<AppState>) -> ApiResult<Json<SourceList>> {
    ctx.profile()?;
    let list = state.db(|s| Ok(s.list_sources()?)).await?;
    Ok(Json(SourceList {
        sources: list.iter().map(views::source_view).collect(),
    }))
}

/// `GET /api/sources/{id}`
pub async fn get(
    ctx: Ctx,
    State(state): State<AppState>,
    PathId(id): PathId,
) -> ApiResult<Json<SourceView>> {
    ctx.profile()?;
    let s = state
        .db(move |s| s.get_source(id)?.ok_or_else(source_not_found))
        .await?;
    Ok(Json(views::source_view(&s)))
}

/// `POST /api/sources` (201) : ajoute une source et lance son premier scan.
pub async fn create(
    ctx: Ctx,
    State(state): State<AppState>,
    JsonBody(req): JsonBody<CreateSourceRequest>,
) -> ApiResult<(StatusCode, Json<SourceView>)> {
    ctx.admin()?;
    let name = req.name.trim().to_string();
    if name.is_empty() || name.chars().count() > 64 {
        return Err(ApiError::validation("name", "entre 1 et 64 caractères"));
    }
    let uri = req.uri.trim().trim_end_matches('/').to_string();
    check_source_uri(req.protocol, &uri)?;
    if let Some(c) = &req.credentials {
        if req.protocol == Protocol::Local {
            return Err(ApiError::validation(
                "credentials",
                "inutiles pour un dossier local",
            ));
        }
        if c.username.trim().is_empty() || c.username.len() > 128 || c.password.len() > 256 {
            return Err(ApiError::validation(
                "credentials",
                "identifiant ou mot de passe invalide",
            ));
        }
    }
    let new = NewSource {
        name,
        protocol: req.protocol,
        uri,
        credentials: req.credentials,
    };
    let source = state.db(move |s| Ok(s.create_source(&new)?)).await?;
    tracing::info!(source = source.id, uri = %source.uri, "source ajoutée");
    let _ = start_scan(&state, Some(source.id), None);
    Ok((StatusCode::CREATED, Json(views::source_view(&source))))
}

/// `PATCH /api/sources/{id}` : nom, identifiants (`null` pour les effacer).
pub async fn update(
    ctx: Ctx,
    State(state): State<AppState>,
    PathId(id): PathId,
    JsonBody(req): JsonBody<UpdateSourceRequest>,
) -> ApiResult<Json<SourceView>> {
    ctx.admin()?;
    let s = state
        .db(move |s| {
            s.get_source(id)?.ok_or_else(source_not_found)?;
            if let Some(name) = &req.name {
                let name = name.trim();
                if name.is_empty() || name.chars().count() > 64 {
                    return Err(ApiError::validation("name", "entre 1 et 64 caractères"));
                }
                s.rename_source(id, name)?;
            }
            if let Some(creds) = &req.credentials {
                s.set_source_credentials(id, creds.as_ref())?;
            }
            s.get_source(id)?.ok_or_else(source_not_found)
        })
        .await?;
    Ok(Json(views::source_view(&s)))
}

/// `DELETE /api/sources/{id}` (204) : la source et ses médias.
pub async fn delete(
    ctx: Ctx,
    State(state): State<AppState>,
    PathId(id): PathId,
) -> ApiResult<StatusCode> {
    ctx.admin()?;
    if lock(&state.scan).running && lock(&state.scan).source_id.is_none_or(|s| s == id) {
        return Err(scan_in_progress());
    }
    state.db(move |s| Ok(s.delete_source(id)?)).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/sources/discover` : partages SMB annoncés sur le réseau local.
pub async fn discover(ctx: Ctx) -> ApiResult<Json<DiscoveredSources>> {
    ctx.admin()?;
    let found = tokio::task::spawn_blocking(|| {
        epikodi_sources::discovery::discover_smb(Duration::from_secs(2))
    })
    .await
    .map_err(ApiError::internal)?;
    Ok(Json(DiscoveredSources {
        sources: found
            .into_iter()
            .map(|d| DiscoveredSourceView {
                name: d.name,
                protocol: Protocol::Smb,
                uri: d.uri,
            })
            .collect(),
    }))
}

fn scan_in_progress() -> ApiError {
    ApiError::conflict("scan_in_progress", "Un scan est déjà en cours.")
}

#[derive(Debug, Default, Deserialize)]
pub struct ScanQuery {
    /// Scan ciblé d'un dossier (URI, comme `parent_dir`).
    pub dir: Option<String>,
}

/// `POST /api/sources/{id}/scan` (202) : scan (ciblé avec `?dir=`).
pub async fn scan_one(
    ctx: Ctx,
    State(state): State<AppState>,
    PathId(id): PathId,
    QueryParams(q): QueryParams<ScanQuery>,
) -> ApiResult<(StatusCode, Json<ScanStatus>)> {
    ctx.admin()?;
    state
        .db(move |s| s.get_source(id)?.ok_or_else(source_not_found))
        .await?;
    let status = start_scan(&state, Some(id), q.dir)?;
    Ok((StatusCode::ACCEPTED, Json(status)))
}

/// `POST /api/library/scan` (202) : toutes les sources.
pub async fn scan_all(
    ctx: Ctx,
    State(state): State<AppState>,
) -> ApiResult<(StatusCode, Json<ScanStatus>)> {
    ctx.admin()?;
    let status = start_scan(&state, None, None)?;
    Ok((StatusCode::ACCEPTED, Json(status)))
}

/// `GET /api/library/scan` : état du dernier scan.
pub async fn scan_status(ctx: Ctx, State(state): State<AppState>) -> ApiResult<Json<ScanStatus>> {
    ctx.profile()?;
    Ok(Json(lock(&state.scan).clone()))
}

/// Lance un scan en tâche de fond (erreur si un scan tourne déjà).
pub fn start_scan(
    state: &AppState,
    source_id: Option<Id>,
    dir: Option<String>,
) -> ApiResult<ScanStatus> {
    let status = {
        let mut scan = lock(&state.scan);
        if scan.running {
            return Err(scan_in_progress());
        }
        *scan = ScanStatus {
            running: true,
            source_id,
            started_at: Some(state.now_iso()),
            ..Default::default()
        };
        scan.clone()
    };
    let st = state.clone();
    tokio::task::spawn_blocking(move || {
        let started = Instant::now();
        let result = run_scan(&st, source_id, dir.as_deref());
        let snapshot = {
            let mut scan = lock(&st.scan);
            if let Err(e) = result {
                scan.errors.push(e.message);
            }
            scan.running = false;
            scan.finished_at = Some(st.now_iso());
            scan.duration_ms = Some(started.elapsed().as_millis() as u64);
            scan.clone()
        };
        tracing::info!(?snapshot, "scan terminé");
        st.emit(Event::LibraryUpdated(snapshot));
    });
    Ok(status)
}

fn run_scan(state: &AppState, source_id: Option<Id>, dir: Option<&str>) -> ApiResult<()> {
    let store = state.open_background_store()?;
    let sources: Vec<Source> = match source_id {
        Some(id) => vec![store.get_source(id)?.ok_or_else(source_not_found)?],
        None => store.list_sources()?,
    };
    for src in sources {
        let creds = store.source_credentials(src.id).unwrap_or(None);
        let file_source = match epikodi_sources::open_source(src.protocol, &src.uri, creds) {
            Ok(f) => f,
            Err(e) => {
                store.set_source_scanned(src.id, SourceStatus::Error, &state.now_iso())?;
                state.emit(Event::SourceStatus {
                    source_id: src.id,
                    status: SourceStatus::Error,
                });
                lock(&state.scan).errors.push(format!("{} : {e}", src.name));
                continue;
            }
        };
        let scanner = Scanner::new(&store, file_source.as_ref(), src.id);
        let report = match dir {
            Some(d) => scanner.scan_dir(d)?,
            None => scanner.scan()?,
        };
        let status = if report.source_error.is_some() {
            store
                .get_source(src.id)?
                .map(|s| s.status)
                .unwrap_or(SourceStatus::Error)
        } else {
            SourceStatus::Online
        };
        state.emit(Event::SourceStatus {
            source_id: src.id,
            status,
        });
        let mut scan = lock(&state.scan);
        scan.added += report.added;
        scan.updated += report.updated;
        scan.moved += report.moved;
        scan.missing += report.missing;
        scan.unchanged += report.unchanged;
        scan.errors.extend(report.errors);
        if let Some(e) = report.source_error {
            scan.errors.push(format!("{} : {e}", src.name));
        }
    }
    // Enrichissement : hors connexion, on s'arrête sans erreur.
    if !state.providers.is_empty() {
        let images: &dyn ImageCache = state.images.as_ref();
        match Enricher::new(&store, &state.providers, images).enrich_unmatched(500) {
            Ok(r) => lock(&state.scan).enriched = r.enriched,
            Err(e) => lock(&state.scan)
                .errors
                .push(format!("enrichissement : {e}")),
        }
    }
    Ok(())
}
