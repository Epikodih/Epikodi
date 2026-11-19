//! API du media center (axum) : HTTP + WebSocket + SSE, port 7878.
//!
//! - Publiques : santé, appairage.
//! - App TV (accès local) : `/api/tv/*`, publication de l'état du lecteur.
//! - Le reste : jeton d'un appareil appairé (`Authorization: Bearer`), ou
//!   l'app TV en local.

pub mod auth;
pub mod error;
pub mod extract;
pub mod http;
pub mod state;
pub mod views;
pub mod ws;

use axum::extract::Request;
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::Router;

pub use error::{ApiError, ApiResult};
pub use state::{ApiConfig, AppState, Clock, StoreFactory};

/// Routeur complet, prêt à servir.
pub fn router(state: AppState) -> Router {
    http::routes()
        .fallback(not_found)
        .layer(middleware::from_fn(log_requests))
        .with_state(state)
}

async fn not_found() -> ApiError {
    ApiError::not_found("route_not_found", "Route inconnue.")
}

/// Journal des requêtes (sans en-têtes ni corps : pas de jeton dans les logs).
async fn log_requests(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let started = std::time::Instant::now();
    let res = next.run(req).await;
    let status = res.status();
    if status.is_server_error() {
        tracing::error!(%method, %path, %status, ms = started.elapsed().as_millis() as u64, "requête");
    } else {
        tracing::debug!(%method, %path, %status, ms = started.elapsed().as_millis() as u64, "requête");
    }
    res.into_response()
}
