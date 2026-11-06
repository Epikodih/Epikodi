mod state;

pub use state::AppState;

use std::sync::Arc;

use axum::{routing::get, Router};

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .with_state(state)
}