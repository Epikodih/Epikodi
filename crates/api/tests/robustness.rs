//! Régressions : accès révoqué, demandes de scan, secrets illisibles.

mod common;

use std::net::SocketAddr;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio_tungstenite::tungstenite::Message;

use common::{TestApp, Who};
use epikodi_domain::ports::DeviceRepo;

/// Un WebSocket ouvert est coupé dès que l'appareil est révoqué.
#[tokio::test]
async fn websocket_coupe_apres_revocation() {
    let app = TestApp::new();
    let Who::Remote(token) = app.pair().await else {
        unreachable!()
    };

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let router = app.router.clone();
    tokio::spawn(async move {
        axum::serve(
            listener,
            router.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });

    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/api/ws?token={token}"))
        .await
        .unwrap();
    // état du lecteur à l'ouverture
    let hello: Value =
        serde_json::from_str(ws.next().await.unwrap().unwrap().to_text().unwrap()).unwrap();
    assert_eq!(hello["type"], "player.state");
    ws.send(Message::Text(json!({"type": "ping"}).to_string()))
        .await
        .unwrap();
    let pong: Value =
        serde_json::from_str(ws.next().await.unwrap().unwrap().to_text().unwrap()).unwrap();
    assert_eq!(pong["type"], "pong");

    // révocation (appareil perdu)
    let at = app.state.now_iso();
    app.state
        .db(move |s| {
            let id = s.list_devices(&at)?[0].id;
            Ok(s.revoke_device(id, &at)?)
        })
        .await
        .unwrap();

    ws.send(Message::Text(
        json!({"type": "command", "command": {"action": "pause"}}).to_string(),
    ))
    .await
    .unwrap();
    let reply = tokio::time::timeout(Duration::from_secs(5), ws.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let reply: Value = serde_json::from_str(reply.to_text().unwrap()).unwrap();
    assert_eq!(reply["data"]["code"], "unauthorized");
    // puis la connexion se ferme
    let end = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            match ws.next().await {
                None | Some(Err(_)) | Some(Ok(Message::Close(_))) => break,
                Some(Ok(_)) => continue,
            }
        }
    })
    .await;
    assert!(end.is_ok(), "la connexion doit se fermer");
}

async fn admin(app: &TestApp) -> Who {
    let r = app.pair().await;
    app.create_profile(&r, "Marc", 35, false).await;
    r
}

async fn wait_all_scanned(app: &TestApp, who: &Who, n: usize) -> Value {
    for _ in 0..200 {
        let (_, src) = app.get("/api/sources", who).await;
        let (_, scan) = app.get("/api/library/scan", who).await;
        let done = src["sources"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|s| s["last_scan_at"].is_string())
            .count();
        if done == n && scan["running"] == false {
            return src;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("toutes les sources n'ont pas été scannées");
}

/// Ajouter une source pendant un scan : son scan n'est pas perdu.
#[tokio::test]
async fn scan_demande_pendant_un_autre_n_est_pas_perdu() {
    let app = TestApp::new();
    let r = admin(&app).await;
    let mut uris = Vec::new();
    for name in ["a", "b", "c"] {
        let dir = app.dir.path().join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("Film {name} (2020).mkv")), name).unwrap();
        uris.push(format!("file://{}", dir.to_string_lossy()));
    }
    // trois ajouts d'affilée : les deux derniers arrivent pendant le premier scan
    for (i, uri) in uris.iter().enumerate() {
        let (s, b) = app
            .post(
                "/api/sources",
                json!({"name": format!("S{i}"), "protocol": "local", "uri": uri}),
                &r,
            )
            .await;
        assert_eq!(s, axum::http::StatusCode::CREATED, "{b}");
    }
    wait_all_scanned(&app, &r, 3).await;
    let (_, list) = app.get("/api/media?kind=movie", &r).await;
    assert_eq!(list["total"], 3);
}

/// Identifiants qui ne se déchiffrent plus : la source passe en erreur
/// (au lieu d'être scannée sans identifiants, en silence).
#[tokio::test]
async fn identifiants_illisibles_signales() {
    let app = TestApp::new();
    let r = admin(&app).await;
    let (_, src) = app
        .post(
            "/api/sources",
            json!({"name": "NAS", "protocol": "smb", "uri": "smb://nas/films",
                   "credentials": {"username": "marc", "password": "x"}}),
            &r,
        )
        .await;
    let id = src["id"].as_i64().unwrap();
    wait_all_scanned(&app, &r, 1).await;
    app.state
        .db(move |s| {
            s.conn()
                .execute(
                    "UPDATE source SET credentials_encrypted = x'00112233445566778899aabbccddeeff' WHERE id = ?1",
                    [id],
                )
                .unwrap();
            Ok(())
        })
        .await
        .unwrap();
    app.post(&format!("/api/sources/{id}/scan"), json!(null), &r)
        .await;
    for _ in 0..100 {
        let (_, scan) = app.get("/api/library/scan", &r).await;
        if scan["running"] == false && scan["errors"].as_array().is_some_and(|e| !e.is_empty()) {
            let errors = scan["errors"].to_string();
            assert!(errors.contains("identifiants illisibles"), "{errors}");
            let (_, s) = app.get(&format!("/api/sources/{id}"), &r).await;
            assert_eq!(s["status"], "error");
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("erreur non signalée");
}
