//! File d'attente, lecteur, télécommande, temps réel.

mod common;

use axum::http::StatusCode;
use serde_json::json;

use common::{assert_same_shape, fixture, TestApp, Who};
use epikodi_domain::media::MediaKind::*;
use epikodi_domain::protocol::Event;

async fn setup() -> (TestApp, Who, Vec<i64>) {
    let app = TestApp::new();
    let a = app.media(None, Movie, "A", Some(0)).await;
    let b = app.media(None, Movie, "B", Some(0)).await;
    let c = app.media(None, Movie, "C", Some(0)).await;
    for id in [a, b, c] {
        app.state
            .db(move |s| {
                s.conn()
                    .execute("UPDATE media SET duration_s = 1000 WHERE id = ?1", [id])
                    .unwrap();
                Ok(())
            })
            .await
            .unwrap();
    }
    let r = app.pair().await;
    app.create_profile(&r, "Marc", 35, false).await;
    (app, r, vec![a, b, c])
}

#[tokio::test]
async fn file_d_attente() {
    let (app, r, m) = setup().await;
    let (s, q) = app
        .post(
            "/api/queue",
            fixture("queue/add_request.json")
                .as_object()
                .map(|_| json!({"media_ids": [m[0], m[1]]}))
                .unwrap(),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{q}");
    assert_same_shape(&q, &fixture("queue/list.json"), "$");
    assert_eq!(q["items"].as_array().unwrap().len(), 2);
    let (_, q) = app
        .post(
            "/api/queue",
            json!({"media_ids": [m[2]], "mode": "next"}),
            &r,
        )
        .await;
    let order: Vec<_> = q["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["media"]["id"].as_i64().unwrap())
        .collect();
    assert_eq!(
        order,
        [m[2], m[0], m[1]],
        "« lire ensuite » sans média en cours : en tête"
    );
    let last = q["items"][2]["id"].as_i64().unwrap();
    let (_, q) = app
        .patch(&format!("/api/queue/{last}"), json!({"position": 0}), &r)
        .await;
    assert_eq!(q["items"][0]["media"]["id"], m[1]);
    assert_eq!(
        app.delete(&format!("/api/queue/{last}"), &r).await.0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        app.delete("/api/queue/9999", &r).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(app.delete("/api/queue", &r).await.0, StatusCode::NO_CONTENT);
    let (_, q) = app.get("/api/queue", &r).await;
    assert_eq!(q["items"], json!([]));
}

#[tokio::test]
async fn lecture_commandes_et_etat() {
    let (app, r, m) = setup().await;
    let mut events = app.state.events.subscribe();

    let (s, st) = app
        .post("/api/player/play", json!({"media_id": m[1]}), &r)
        .await;
    assert_eq!(s, StatusCode::OK, "{st}");
    assert_same_shape(&st, &fixture("player/state.json"), "$");
    assert_eq!(st["status"], "loading");
    assert_eq!(st["now_playing"]["media"]["id"], m[1]);

    // la TV a reçu l'ordre de charger le média
    let mut got_load = false;
    while let Ok(e) = events.try_recv() {
        if let Event::PlayerCommand(epikodi_domain::protocol::TvCommand::Load {
            now_playing, ..
        }) = e
        {
            assert_eq!(now_playing.media.unwrap().id, m[1]);
            got_load = true;
        }
    }
    assert!(got_load);

    // la TV publie son état réel
    let (s, st) = app
        .post(
            "/api/player/state",
            json!({"status": "playing", "position_s": 42.0, "duration_s": 1000.0}),
            &Who::Tv,
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{st}");
    assert_eq!(st["position_s"], 42.0);
    // … mais pas une télécommande
    let (s, _) = app
        .post("/api/player/state", json!({"status": "playing"}), &r)
        .await;
    assert_eq!(s, StatusCode::FORBIDDEN);

    let (_, st) = app
        .post("/api/player/command", json!({"action": "pause"}), &r)
        .await;
    assert_eq!(st["status"], "paused");
    let (_, st) = app
        .post(
            "/api/player/command",
            fixture("player/command_request.json"),
            &r,
        )
        .await;
    assert_eq!(st["position_s"], 120.0);
    let (_, st) = app
        .post(
            "/api/player/command",
            json!({"action": "volume", "level": 30}),
            &r,
        )
        .await;
    assert_eq!(st["volume"], 30);
    let (_, st) = app
        .post("/api/player/command", json!({"action": "volume_up"}), &r)
        .await;
    assert_eq!(st["volume"], 35);
    let (_, st) = app
        .post("/api/player/command", json!({"action": "mute"}), &r)
        .await;
    assert_eq!(st["muted"], true);
    let (_, st) = app
        .post(
            "/api/player/command",
            json!({"action": "subtitle_track", "id": 2}),
            &r,
        )
        .await;
    assert_eq!(st["subtitle_track"], 2);
    let (_, st) = app
        .post(
            "/api/player/command",
            json!({"action": "repeat", "mode": "all"}),
            &r,
        )
        .await;
    assert_eq!(st["repeat"], "all");
    let (s, b) = app
        .post(
            "/api/player/command",
            json!({"action": "volume", "level": 300}),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(b["error"]["code"], "validation_failed");
    let (s, _) = app
        .post(
            "/api/player/command",
            json!({"action": "autodestruction"}),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);

    let (_, st) = app.get("/api/player", &r).await;
    assert_eq!(st["volume"], 35);
}

#[tokio::test]
async fn suivant_precedent_et_enchainement() {
    let (app, r, m) = setup().await;
    app.post("/api/queue", json!({"media_ids": m, "mode": "replace"}), &r)
        .await;
    let (_, q) = app.get("/api/queue", &r).await;
    let first = q["items"][0]["id"].as_i64().unwrap();
    app.post("/api/player/play", json!({"queue_item_id": first}), &r)
        .await;
    let (_, st) = app
        .post("/api/player/command", json!({"action": "next"}), &r)
        .await;
    assert_eq!(st["now_playing"]["media"]["id"], m[1]);
    let (_, st) = app
        .post("/api/player/command", json!({"action": "previous"}), &r)
        .await;
    assert_eq!(st["now_playing"]["media"]["id"], m[0]);

    // fin du média : progression enregistrée (vu) et média suivant
    let (_, st) = app
        .post(
            "/api/player/state",
            json!({"status": "ended", "position_s": 1000.0, "duration_s": 1000.0}),
            &Who::Tv,
        )
        .await;
    assert_eq!(st["now_playing"]["media"]["id"], m[1]);
    let (_, d) = app.get(&format!("/api/media/{}", m[0]), &r).await;
    assert_eq!(d["is_watched"], true);

    // pause : la position est sauvegardée pour la reprise
    app.post(
        "/api/player/state",
        json!({"status": "paused", "position_s": 300.0}),
        &Who::Tv,
    )
    .await;
    let (_, d) = app.get(&format!("/api/media/{}", m[1]), &r).await;
    assert_eq!(d["progress"]["resume_at_s"], 300);

    // fin de la file : arrêt
    app.post("/api/player/command", json!({"action": "next"}), &r)
        .await;
    let (_, st) = app
        .post(
            "/api/player/state",
            json!({"status": "ended", "position_s": 1000.0}),
            &Who::Tv,
        )
        .await;
    assert_eq!(st["status"], "stopped");
}

#[tokio::test]
async fn reprise_a_la_position_sauvegardee() {
    let (app, r, m) = setup().await;
    app.put(
        &format!("/api/media/{}/progress", m[0]),
        json!({"position_s": 400}),
        &r,
    )
    .await;
    let (_, st) = app
        .post("/api/player/play", json!({"media_id": m[0]}), &r)
        .await;
    assert_eq!(st["position_s"], 400.0);
    let (_, st) = app
        .post(
            "/api/player/play",
            json!({"media_id": m[0], "resume": false}),
            &r,
        )
        .await;
    assert_eq!(st["position_s"], 0.0);
}

#[tokio::test]
async fn regles_de_lecture() {
    let (app, r, m) = setup().await;
    let (s, _) = app.post("/api/player/play", json!({}), &r).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    let (s, _) = app
        .post(
            "/api/player/play",
            json!({"media_id": m[0], "queue_item_id": 1}),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    let film18 = app.media(None, Movie, "Interdit", Some(18)).await;
    app.post("/api/session/profile", json!({"type": "guest"}), &r)
        .await;
    let (s, b) = app
        .post("/api/player/play", json!({"media_id": film18}), &r)
        .await;
    assert_eq!(s, StatusCode::NOT_FOUND, "{b}");
    let series = app.media(None, Series, "Vide", Some(0)).await;
    let (s, b) = app
        .post("/api/player/play", json!({"media_id": series}), &r)
        .await;
    assert_eq!(s, StatusCode::CONFLICT);
    assert_eq!(b["error"]["code"], "nothing_to_play");
}

#[tokio::test]
async fn envoyer_vers_la_tele_et_touches() {
    let (app, r, _) = setup().await;
    let mut events = app.state.events.subscribe();
    let (s, st) = app
        .post(
            "/api/player/cast",
            json!({"url": "https://example.org/video.webm", "kind": "video"}),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(st["now_playing"]["title"], "video.webm");
    assert!(st["now_playing"]["media"].is_null());
    let (s, _) = app
        .post(
            "/api/player/cast",
            json!({"url": "file:///etc/passwd", "kind": "video"}),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);

    let (s, _) = app
        .post("/api/remote/input", json!({"key": "up"}), &r)
        .await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    let mut keys = vec![];
    while let Ok(e) = events.try_recv() {
        if let Event::Input { key } = e {
            keys.push(key);
        }
    }
    assert_eq!(keys, [epikodi_domain::protocol::RemoteKey::Up]);
    let (s, _) = app
        .post("/api/remote/input", json!({"key": "up"}), &Who::Stranger)
        .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn flux_sse_de_la_tv() {
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    let app = TestApp::new();
    let (sid, _) = app.tv_connect().await;
    let mut req = axum::http::Request::get(format!("/api/tv/events?tv_session_id={sid}"))
        .body(axum::body::Body::empty())
        .unwrap();
    req.extensions_mut()
        .insert(axum::extract::ConnectInfo::<std::net::SocketAddr>(
            "127.0.0.1:1".parse().unwrap(),
        ));
    let res = app.router.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.headers()["content-type"], "text/event-stream");
    let mut body = res.into_body();
    let first = body.frame().await.unwrap().unwrap().into_data().unwrap();
    let text = String::from_utf8_lossy(&first);
    assert!(text.contains("\"phase\":\"pairing\""), "{text}");

    // l'appairage pousse un nouvel état
    app.pair().await;
    let mut seen_paired = false;
    for _ in 0..10 {
        let frame = tokio::time::timeout(std::time::Duration::from_secs(2), body.frame()).await;
        let Ok(Some(Ok(f))) = frame else { break };
        if let Ok(d) = f.into_data() {
            if String::from_utf8_lossy(&d).contains("\"phase\":\"paired\"") {
                seen_paired = true;
                break;
            }
        }
    }
    assert!(seen_paired);

    let (s, _) = app
        .get("/api/tv/events?tv_session_id=inconnue", &Who::Tv)
        .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}
