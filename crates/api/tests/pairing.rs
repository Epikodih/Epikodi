//! Démarrage, connexion de la TV, appairage, jeton, session.

mod common;

use axum::http::StatusCode;
use serde_json::json;

use common::{assert_same_shape, fixture, TestApp, Who};

#[tokio::test]
async fn sante_503_tant_que_la_base_n_est_pas_prete() {
    let app = TestApp::starting();
    let (s, b) = app.get("/api/health", &Who::Stranger).await;
    assert_eq!(s, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(b, json!({"status": "starting"}));
    // la TV qui se connecte trop tôt réessaie
    let (s, _) = app
        .post(
            "/api/tv/connect",
            fixture("tv/connect_request.json"),
            &Who::Tv,
        )
        .await;
    assert_eq!(s, StatusCode::SERVICE_UNAVAILABLE);

    app.state.open_db().unwrap();
    let (s, b) = app.get("/api/health", &Who::Stranger).await;
    assert_eq!(s, StatusCode::OK);
    assert_same_shape(&b, &fixture("health/ok.json"), "$");
}

#[tokio::test]
async fn connexion_de_la_tv() {
    let app = TestApp::new();
    let (s, b) = app
        .post(
            "/api/tv/connect",
            fixture("tv/connect_request.json"),
            &Who::Tv,
        )
        .await;
    assert_eq!(s, StatusCode::CREATED);
    assert_same_shape(&b, &fixture("tv/connect_response.json"), "$");
    let code = b["pairing"]["code"].as_str().unwrap();
    assert_eq!(code.len(), 6);
    assert_eq!(b["pairing"]["expires_in_s"], 300);
    assert_eq!(b["pairing"]["attempts_left"], 5);
    assert_eq!(b["lan_ip"], "192.168.1.42");
    let qr = b["pairing"]["qr"]["payload"].as_str().unwrap();
    assert!(qr.starts_with("mediacenter://pair?host=192.168.1.42&port=7878&sid="));
    assert!(!qr.contains(code), "le QR ne contient jamais le code");
}

#[tokio::test]
async fn routes_tv_reservees_au_local() {
    let app = TestApp::new();
    let (s, b) = app
        .post(
            "/api/tv/connect",
            fixture("tv/connect_request.json"),
            &Who::Stranger,
        )
        .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    assert_eq!(b["error"]["code"], "localhost_only");
    let (s, _) = app.get("/api/tv/status", &Who::Stranger).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn validation_de_connect() {
    let app = TestApp::new();
    let (s, b) = app
        .post(
            "/api/tv/connect",
            json!({"model": "", "platform": "linux", "app_version": "1"}),
            &Who::Tv,
        )
        .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(b["error"]["code"], "validation_failed");
    assert!(b["error"]["details"]["fields"]["model"].is_string());
    let (s, b) = app
        .post("/api/tv/connect", json!({"model": "x"}), &Who::Tv)
        .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(b["error"]["code"], "validation_failed");
}

#[tokio::test]
async fn rafraichir_le_code() {
    let app = TestApp::new();
    let (sid, code) = app.tv_connect().await;
    let (s, b) = app
        .post(
            "/api/tv/pairing/refresh",
            json!({"tv_session_id": sid}),
            &Who::Tv,
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_same_shape(&b, &fixture("tv/refresh_response.json"), "$");
    let new_code = b["pairing"]["code"].as_str().unwrap();
    // l'ancien code ne fonctionne plus (sauf tirage identique, improbable)
    if new_code != code {
        let (s, _) = app
            .post(
                "/api/remote/pair",
                json!({"code": code, "device": {"name": "x"}}),
                &Who::Stranger,
            )
            .await;
        assert_eq!(s, StatusCode::FORBIDDEN);
    }
    let (s, b) = app
        .post(
            "/api/tv/pairing/refresh",
            json!({"tv_session_id": "inconnue"}),
            &Who::Tv,
        )
        .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    assert_eq!(b["error"]["code"], "tv_session_not_found");
}

#[tokio::test]
async fn appairage_complet_et_phases_de_la_tv() {
    let app = TestApp::new();
    let (sid, code) = app.tv_connect().await;

    let (s, st) = app
        .get(&format!("/api/tv/status?tv_session_id={sid}"), &Who::Tv)
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_same_shape(&st, &fixture("tv/status.json"), "$");
    assert_eq!(st["phase"], "pairing");
    assert_eq!(st["pairing"]["code"], code.as_str());

    let mut req = fixture("pairing/request.json");
    req["code"] = json!(code);
    let (s, b) = app.post("/api/remote/pair", req, &Who::Stranger).await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_same_shape(&b, &fixture("pairing/response.json"), "$");
    assert_eq!(b["token_type"], "Bearer");
    assert_eq!(b["expires_at"], "2026-12-07T18:30:00Z", "30 jours");
    let token = Who::Remote(b["token"].as_str().unwrap().into());

    let (_, st) = app
        .get(&format!("/api/tv/status?tv_session_id={sid}"), &Who::Tv)
        .await;
    assert_eq!(st["phase"], "paired");
    assert!(st["pairing"].is_null());
    assert_eq!(st["device"]["name"], "Pixel 7");

    // code à usage unique
    let (s, _) = app
        .post(
            "/api/remote/pair",
            json!({"code": code, "device": {"name": "Autre"}}),
            &Who::Stranger,
        )
        .await;
    assert_eq!(s, StatusCode::FORBIDDEN);

    // session de la télécommande
    let (s, cur) = app.get("/api/session", &token).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(cur["device"]["name"], "Pixel 7");
    assert!(cur["profile"].is_null());

    // le profil choisi fait passer la TV en « ready »
    let (s, _) = app
        .post("/api/session/profile", json!({"type": "guest"}), &token)
        .await;
    assert_eq!(s, StatusCode::OK);
    let (_, st) = app
        .get(&format!("/api/tv/status?tv_session_id={sid}"), &Who::Tv)
        .await;
    assert_eq!(st["phase"], "ready");
    assert_eq!(st["active_profile"]["kind"], "guest");
}

#[tokio::test]
async fn code_faux_puis_trop_de_tentatives() {
    let app = TestApp::new();
    let (_, code) = app.tv_connect().await;
    let wrong = if code == "000000" { "111111" } else { "000000" };
    for left in (1..5).rev() {
        let (s, b) = app
            .post(
                "/api/remote/pair",
                json!({"code": wrong, "device": {"name": "x"}}),
                &Who::Stranger,
            )
            .await;
        assert_eq!(s, StatusCode::FORBIDDEN);
        assert_eq!(b["error"]["code"], "pairing_code_invalid");
        assert_eq!(b["error"]["details"]["attempts_left"], left);
    }
    let (s, b) = app
        .post(
            "/api/remote/pair",
            json!({"code": wrong, "device": {"name": "x"}}),
            &Who::Stranger,
        )
        .await;
    assert_eq!(s, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(b["error"]["code"], "pairing_too_many_attempts");
    assert!(b["error"]["details"]["retry_after_s"].as_u64().unwrap() > 0);
    // même le bon (ancien) code est refusé
    let (s, _) = app
        .post(
            "/api/remote/pair",
            json!({"code": code, "device": {"name": "x"}}),
            &Who::Stranger,
        )
        .await;
    assert_eq!(s, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn code_expire() {
    let app = TestApp::new();
    let (_, code) = app.tv_connect().await;
    app.advance(301);
    let (s, b) = app
        .post(
            "/api/remote/pair",
            json!({"code": code, "device": {"name": "x"}}),
            &Who::Stranger,
        )
        .await;
    assert_eq!(s, StatusCode::GONE);
    assert_eq!(b["error"]["code"], "pairing_code_expired");
}

#[tokio::test]
async fn jeton_invalide_expire_ou_revoque() {
    let app = TestApp::new();
    let (s, _) = app
        .get("/api/session", &Who::Remote("mc_faux".into()))
        .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    // un jeton faux est refusé même en local
    let (s, _) = app.get("/api/session", &Who::Stranger).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);

    let token = app.pair().await;
    let (s, _) = app.get("/api/session", &token).await;
    assert_eq!(s, StatusCode::OK);

    // déconnexion : jeton révoqué
    let (s, _) = app.delete("/api/session", &token).await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    let (s, b) = app.get("/api/session", &token).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    assert_eq!(b["error"]["code"], "unauthorized");

    // expiration après 30 jours
    let token = app.pair().await;
    app.advance(31 * 24 * 3600);
    let (s, _) = app.get("/api/session", &token).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn reappairage_d_une_meme_installation() {
    let app = TestApp::new();
    let first = app.pair().await;
    let second = app.pair().await; // même install_id
    assert_eq!(
        app.get("/api/session", &first).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(app.get("/api/session", &second).await.0, StatusCode::OK);
}

#[tokio::test]
async fn appareils_listes_et_revoques_par_un_adulte() {
    let app = TestApp::new();
    let token = app.pair().await;
    let (s, b) = app.get("/api/devices", &token).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    assert_eq!(b["error"]["code"], "profile_required");
    app.create_profile(&token, "Marc", 35, false).await;
    let (s, b) = app.get("/api/devices", &token).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(b["devices"][0]["current"], true);
    let id = b["devices"][0]["id"].as_i64().unwrap();
    assert_eq!(
        app.delete(&format!("/api/devices/{id}"), &token).await.0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        app.get("/api/session", &token).await.0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn la_tv_locale_n_a_pas_besoin_de_jeton() {
    let app = TestApp::new();
    let (s, b) = app.get("/api/session", &Who::Tv).await;
    assert_eq!(s, StatusCode::OK);
    assert!(b["device"].is_null());
}
