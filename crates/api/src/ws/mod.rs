//! Temps réel.
//!
//! - `GET /api/ws` (WebSocket, télécommande et TV) : le serveur pousse les
//!   événements `{"type", "data"}` (état du lecteur, file, profil, touches,
//!   médiathèque...). Le client peut envoyer `{"type": "ping"}`,
//!   `{"type": "input", "key": "up"}` ou
//!   `{"type": "command", "command": {"action": "pause"}}`.
//! - `GET /api/tv/events?tv_session_id=` (SSE, app TV, local) : l'état de la
//!   TV (`pairing` → `paired` → `ready`) à l'ouverture puis à chaque changement.

use std::convert::Infallible;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::response::Response;
use futures_util::stream::{self, Stream};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::sync::broadcast::error::RecvError;

use epikodi_domain::protocol::{ClientMessage, Event, TvStatus};

use crate::auth::{Auth, LocalOnly};
use crate::error::{ApiError, ApiResult};
use crate::extract::QueryParams;
use crate::http::player::apply_command;
use crate::http::tv::{tv_status, StatusQuery};
use crate::state::AppState;

/// `GET /api/ws`
pub async fn handler(auth: Auth, State(state): State<AppState>, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(move |socket| client(socket, state, auth))
}

async fn client(socket: WebSocket, state: AppState, auth: Auth) {
    let who = auth
        .device()
        .map_or("tv".to_string(), |d| format!("appareil {}", d.id));
    tracing::debug!(%who, "WebSocket ouvert");
    let (mut tx, mut rx) = socket.split();
    let mut events = state.events.subscribe();

    // État courant dès l'ouverture.
    let hello = Event::PlayerState(Box::new(
        state
            .player
            .lock()
            .map(|p| p.state.clone())
            .unwrap_or_else(|e| e.into_inner().state.clone()),
    ));
    if send(&mut tx, &hello).await.is_err() {
        return;
    }

    loop {
        tokio::select! {
            ev = events.recv() => match ev {
                Ok(e) => {
                    if send(&mut tx, &e).await.is_err() {
                        break;
                    }
                }
                Err(RecvError::Lagged(n)) => tracing::debug!(%who, "{n} événements sautés"),
                Err(RecvError::Closed) => break,
            },
            msg = rx.next() => match msg {
                Some(Ok(Message::Text(text))) => {
                    let reply = handle(&state, &text).await;
                    if let Some(r) = reply {
                        if tx.send(Message::Text(r.to_string())).await.is_err() {
                            break;
                        }
                    }
                }
                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                Some(Ok(_)) => {}
            },
        }
    }
    tracing::debug!(%who, "WebSocket fermé");
}

async fn send<S>(tx: &mut S, e: &Event) -> Result<(), ()>
where
    S: SinkExt<Message> + Unpin,
{
    let text = serde_json::to_string(e).map_err(|_| ())?;
    tx.send(Message::Text(text)).await.map_err(|_| ())
}

fn error_message(e: &ApiError) -> serde_json::Value {
    json!({"type": "error", "data": {"code": e.code, "message": e.message, "details": e.details}})
}

async fn handle(state: &AppState, text: &str) -> Option<serde_json::Value> {
    let msg: ClientMessage = match serde_json::from_str(text) {
        Ok(m) => m,
        Err(e) => {
            return Some(error_message(&ApiError::validation(
                "message",
                e.to_string(),
            )))
        }
    };
    match msg {
        ClientMessage::Ping => Some(json!({"type": "pong"})),
        ClientMessage::Input { key } => {
            state.emit(Event::Input { key });
            None
        }
        ClientMessage::Command { command } => match apply_command(state, command).await {
            Ok(_) => None, // l'état est diffusé à tous
            Err(e) => Some(error_message(&e)),
        },
    }
}

fn sse_event(status: &TvStatus) -> Result<SseEvent, Infallible> {
    Ok(SseEvent::default()
        .event("status")
        .data(serde_json::to_string(status).unwrap_or_else(|_| "{}".into())))
}

/// `GET /api/tv/events?tv_session_id=...`
pub async fn tv_events(
    _: LocalOnly,
    State(state): State<AppState>,
    QueryParams(q): QueryParams<StatusQuery>,
) -> ApiResult<Sse<impl Stream<Item = Result<SseEvent, Infallible>>>> {
    let sid = q
        .tv_session_id
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ApiError::invalid_query("tv_session_id", "obligatoire"))?;
    let first = tv_status(&state, Some(&sid)).await?;
    let rx = state.tv_changed.subscribe();
    let stream = stream::unfold(
        (state, sid, rx, Some(first)),
        |(state, sid, mut rx, first)| async move {
            if let Some(f) = first {
                return Some((sse_event(&f), (state, sid, rx, None)));
            }
            rx.changed().await.ok()?;
            let status = tv_status(&state, Some(&sid)).await.ok()?;
            Some((sse_event(&status), (state, sid, rx, None)))
        },
    );
    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}
