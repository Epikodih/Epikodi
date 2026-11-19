//! `GET /api/health` : public. 503 `{"status": "starting"}` tant que la base
//! n'est pas prête (la TV réessaie en boucle).

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

use epikodi_domain::protocol::HealthResponse;

use crate::state::AppState;

pub async fn health(State(state): State<AppState>) -> Response {
    if state.is_ready() {
        Json(HealthResponse {
            status: "ok".into(),
            version: state.config.version.clone(),
        })
        .into_response()
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"status": "starting"})),
        )
            .into_response()
    }
}
