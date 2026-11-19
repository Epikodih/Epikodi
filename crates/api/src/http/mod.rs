//! Routes HTTP. Toutes sous `/api`. Voir `fixtures/README.md` (contrat) et
//! `docs/dev/protocol.md` (référence complète).

pub mod devices;
pub mod extensions;
pub mod health;
pub mod library;
pub mod media;
pub mod player;
pub mod profiles;
pub mod queue;
pub mod session;
pub mod sources;
pub mod tv;

use axum::extract::DefaultBodyLimit;
use axum::routing::{delete, get, patch, post, put};
use axum::Router;

use crate::state::AppState;
use crate::ws;

pub fn routes() -> Router<AppState> {
    Router::new()
        // santé, TV, appairage
        .route("/api/health", get(health::health))
        .route("/api/tv/connect", post(tv::connect))
        .route("/api/tv/pairing/refresh", post(tv::refresh))
        .route("/api/tv/status", get(tv::status))
        .route("/api/tv/events", get(ws::tv_events))
        .route("/api/remote/pair", post(tv::pair))
        .route("/api/remote/input", post(player::input))
        // comptes et session
        .route("/api/profiles", get(profiles::list).post(profiles::create))
        .route(
            "/api/profiles/:id",
            get(profiles::get)
                .patch(profiles::update)
                .delete(profiles::delete),
        )
        .route(
            "/api/session",
            get(session::current).delete(session::logout),
        )
        .route(
            "/api/session/profile",
            post(session::select).delete(session::clear_profile),
        )
        .route(
            "/api/me/preferences",
            get(library::get_preferences).patch(library::patch_preferences),
        )
        .route("/api/themes", get(library::themes))
        .route("/api/devices", get(devices::list))
        .route("/api/devices/:id", delete(devices::revoke))
        // médiathèque
        .route("/api/home", get(library::home))
        .route("/api/media", get(media::list))
        .route("/api/media/genres", get(media::genres))
        .route("/api/media/years", get(media::years))
        .route("/api/media/recent", get(media::recent))
        .route("/api/media/continue", get(media::continue_watching))
        .route("/api/media/:id", get(media::detail).patch(media::edit))
        .route("/api/media/:id/poster", get(media::poster))
        .route("/api/media/:id/next", get(media::next))
        .route("/api/media/:id/matches", get(media::matches))
        .route("/api/media/:id/match", post(media::apply_match))
        .route("/api/media/:id/progress", put(media::progress))
        .route("/api/media/:id/watched", put(media::watched))
        .route("/api/favorites", get(library::favorites))
        .route(
            "/api/favorites/:id",
            put(library::add_favorite).delete(library::remove_favorite),
        )
        .route(
            "/api/playlists",
            get(library::playlists).post(library::create_playlist),
        )
        .route(
            "/api/playlists/:id",
            get(library::playlist)
                .patch(library::rename_playlist)
                .delete(library::delete_playlist),
        )
        .route(
            "/api/playlists/:id/items",
            post(library::add_playlist_items),
        )
        .route(
            "/api/playlists/:id/items/:position",
            delete(library::remove_playlist_item),
        )
        // file d'attente et lecteur
        .route(
            "/api/queue",
            get(queue::get).post(queue::add).delete(queue::clear),
        )
        .route(
            "/api/queue/:id",
            patch(queue::move_item).delete(queue::remove),
        )
        .route("/api/player", get(player::get))
        .route("/api/player/play", post(player::play))
        .route("/api/player/command", post(player::command))
        .route("/api/player/state", post(player::report))
        .route("/api/player/cast", post(player::cast))
        .route(
            "/api/player/cast/upload",
            post(player::cast_upload).layer(DefaultBodyLimit::disable()),
        )
        // sources et scan
        .route("/api/sources", get(sources::list).post(sources::create))
        .route("/api/sources/discover", get(sources::discover))
        .route(
            "/api/sources/:id",
            get(sources::get)
                .patch(sources::update)
                .delete(sources::delete),
        )
        .route("/api/sources/:id/scan", post(sources::scan_one))
        .route(
            "/api/library/scan",
            get(sources::scan_status).post(sources::scan_all),
        )
        // extensions
        .route(
            "/api/extensions",
            get(extensions::list).post(extensions::install),
        )
        .route("/api/extensions/catalog", get(extensions::catalog))
        .route("/api/extensions/inspect", post(extensions::inspect))
        .route(
            "/api/extensions/:id",
            get(extensions::get)
                .patch(extensions::update)
                .delete(extensions::uninstall),
        )
        .route(
            "/api/extensions/:id/update",
            post(extensions::update_package),
        )
        // temps réel
        .route("/api/ws", get(ws::handler))
}
