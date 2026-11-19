//! Comptes, PIN, invité, préférences.

mod common;

use axum::http::StatusCode;
use serde_json::json;

use common::{assert_same_shape, fixture, TestApp, Who};

#[tokio::test]
async fn premier_lancement_invite_et_creer_un_compte() {
    let app = TestApp::new();
    let r = app.pair().await;
    let (s, b) = app.get("/api/profiles", &r).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(b["profiles"], json!([]));
    assert_eq!(
        b["guest"],
        json!({"available": true, "is_restricted": true})
    );
    assert_eq!(b["can_create"], true);
}

#[tokio::test]
async fn creation_conforme_aux_fixtures() {
    let app = TestApp::new();
    let r = app.pair().await;
    let (s, b) = app
        .post("/api/profiles", fixture("profiles/create_request.json"), &r)
        .await;
    assert_eq!(s, StatusCode::CREATED, "{b}");
    assert_same_shape(&b, &fixture("profiles/create_response.json"), "$");
    assert_eq!(b["active"], true);
    assert_eq!(b["profile"]["kind"], "account");
    let id = b["profile"]["id"].as_i64().unwrap();

    let (_, list) = app.get("/api/profiles", &r).await;
    assert_same_shape(&list, &fixture("profiles/list.json"), "$");
    assert_eq!(list["profiles"][0]["requires_pin"], true);

    let (s, g) = app.get(&format!("/api/profiles/{id}"), &r).await;
    assert_eq!(s, StatusCode::OK);
    assert_same_shape(&g, &fixture("profiles/get_response.json"), "$");

    let (_, cur) = app.get("/api/session", &r).await;
    assert_same_shape(&cur, &fixture("session/current.json"), "$");
    assert_eq!(cur["profile"]["id"], id, "le compte créé est actif");
}

#[tokio::test]
async fn regles_de_creation() {
    let app = TestApp::new();
    let r = app.pair().await;
    // PIN obligatoire pour un compte non restreint
    let (s, b) = app
        .post(
            "/api/profiles",
            json!({"name": "Marc", "age": 35, "is_restricted": false}),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(b["error"]["details"]["fields"]["pin"].is_string());
    // PIN interdit pour un compte restreint
    let (s, _) = app
        .post(
            "/api/profiles",
            json!({"name": "Léa", "age": 10, "is_restricted": true, "pin": "1234"}),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    // nom déjà pris (sans tenir compte de la casse)
    app.create_profile(&r, "Marc", 35, false).await;
    let (s, b) = app
        .post(
            "/api/profiles",
            json!({"name": "marc", "age": 10, "is_restricted": true}),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::CONFLICT);
    assert_eq!(b["error"]["code"], "profile_name_taken");
    // âge invalide
    let (s, _) = app
        .post(
            "/api/profiles",
            json!({"name": "Vieux", "age": 200, "is_restricted": true}),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn un_invite_ne_peut_pas_creer_ni_modifier() {
    let app = TestApp::new();
    let r = app.pair().await;
    let marc = app.create_profile(&r, "Marc", 35, false).await;
    app.post("/api/session/profile", json!({"type": "guest"}), &r)
        .await;
    let (_, list) = app.get("/api/profiles", &r).await;
    assert_eq!(list["can_create"], false);
    let (s, b) = app
        .post(
            "/api/profiles",
            json!({"name": "Pirate", "age": 30, "is_restricted": true}),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    assert_eq!(b["error"]["code"], "guest_forbidden");
    let (s, b) = app
        .patch(&format!("/api/profiles/{marc}"), json!({"name": "X"}), &r)
        .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    assert_eq!(b["error"]["code"], "guest_forbidden");
    // l'invité a la forme attendue
    let (_, cur) = app.get("/api/session", &r).await;
    assert_eq!(
        cur["profile"],
        json!({"kind": "guest", "id": null, "name": "Invité", "icon": null, "language": "fr", "age": 0, "is_restricted": true})
    );
}

#[tokio::test]
async fn un_enfant_ne_peut_pas_se_creer_un_compte_adulte() {
    let app = TestApp::new();
    let r = app.pair().await;
    app.create_profile(&r, "Marc", 35, false).await;
    app.create_profile(&r, "Léa", 10, true).await; // Léa est active
    let (s, b) = app
        .post(
            "/api/profiles",
            json!({"name": "Faux adulte", "age": 30, "is_restricted": false, "pin": "0000"}),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    assert_eq!(b["error"]["code"], "admin_required");
}

#[tokio::test]
async fn choix_du_profil_avec_pin() {
    let app = TestApp::new();
    let r = app.pair().await;
    let marc = app.create_profile(&r, "Marc", 35, false).await;
    let lea = app.create_profile(&r, "Léa", 10, true).await;

    // compte restreint : sans PIN
    let (s, b) = app.select(&r, lea, None).await;
    assert_eq!(s, StatusCode::OK);
    assert_same_shape(&b, &fixture("session/select_response.json"), "$");

    // non restreint : PIN obligatoire
    let (s, b) = app.select(&r, marc, None).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    assert_eq!(b["error"]["code"], "pin_required");
    let (s, b) = app.select(&r, marc, Some("9999")).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    assert_eq!(b["error"]["code"], "pin_invalid");
    assert_eq!(b["error"]["details"]["attempts_left"], 4);
    let (s, _) = app
        .post(
            "/api/session/profile",
            fixture("session/select_request.json")
                .as_object()
                .map(|o| {
                    let mut o = o.clone();
                    o.insert("profile_id".into(), json!(marc));
                    serde_json::Value::Object(o)
                })
                .unwrap(),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::OK);

    // profil inconnu
    let (s, b) = app.select(&r, 999, None).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    assert_eq!(b["error"]["code"], "profile_not_found");
}

#[tokio::test]
async fn pin_verrouille_apres_trop_d_essais() {
    let app = TestApp::new();
    let r = app.pair().await;
    let marc = app.create_profile(&r, "Marc", 35, false).await;
    for left in (1..5).rev() {
        let (_, b) = app.select(&r, marc, Some("0000")).await;
        assert_eq!(b["error"]["details"]["attempts_left"], left);
    }
    let (s, b) = app.select(&r, marc, Some("0000")).await;
    assert_eq!(s, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(b["error"]["code"], "pin_locked");
    // même le bon PIN est refusé pendant le verrouillage
    let (s, _) = app.select(&r, marc, Some("1234")).await;
    assert_eq!(s, StatusCode::TOO_MANY_REQUESTS);
    app.advance(61);
    let (s, _) = app.select(&r, marc, Some("1234")).await;
    assert_eq!(s, StatusCode::OK);
}

#[tokio::test]
async fn modification_et_restriction_d_age() {
    let app = TestApp::new();
    let r = app.pair().await;
    let marc = app.create_profile(&r, "Marc", 35, false).await;
    // nom, icône, langue : sans PIN
    let (s, b) = app
        .patch(
            &format!("/api/profiles/{marc}"),
            json!({"icon": "avatar_05", "language": "en"}),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_same_shape(&b, &fixture("profiles/update_response.json"), "$");
    // l'âge : PIN obligatoire
    let (s, b) = app
        .patch(&format!("/api/profiles/{marc}"), json!({"age": 36}), &r)
        .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    assert_eq!(b["error"]["code"], "pin_required");
    let (s, b) = app
        .patch(
            &format!("/api/profiles/{marc}"),
            json!({"age": 36, "pin": "0000"}),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    assert_eq!(b["error"]["code"], "pin_invalid");
    let mut req = fixture("profiles/update_request.json");
    req["name"] = json!("Marc");
    let (s, b) = app.patch(&format!("/api/profiles/{marc}"), req, &r).await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(b["profile"]["age"], 36);

    // changer de PIN
    let (s, _) = app
        .patch(
            &format!("/api/profiles/{marc}"),
            json!({"pin": "1234", "new_pin": "4321"}),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(app.select(&r, marc, Some("4321")).await.0, StatusCode::OK);
}

#[tokio::test]
async fn un_enfant_ne_se_debride_pas() {
    let app = TestApp::new();
    let r = app.pair().await;
    let marc = app.create_profile(&r, "Marc", 35, false).await;
    let lea = app.create_profile(&r, "Léa", 10, true).await;
    // Léa (active) change son icône : oui
    assert_eq!(
        app.patch(
            &format!("/api/profiles/{lea}"),
            json!({"icon": "avatar_09"}),
            &r
        )
        .await
        .0,
        StatusCode::OK
    );
    // Léa change son âge : non (il faut un adulte)
    let (s, _) = app
        .patch(&format!("/api/profiles/{lea}"), json!({"age": 18}), &r)
        .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    // Léa modifie Marc : non
    let (s, b) = app
        .patch(
            &format!("/api/profiles/{marc}"),
            json!({"name": "Papa"}),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    assert_eq!(b["error"]["code"], "admin_required");
    // Marc (adulte connecté) change l'âge de Léa : oui
    app.select(&r, marc, Some("1234")).await;
    let (s, b) = app
        .patch(&format!("/api/profiles/{lea}"), json!({"age": 12}), &r)
        .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(b["profile"]["age"], 12);
}

#[tokio::test]
async fn suppression_d_un_compte() {
    let app = TestApp::new();
    let r = app.pair().await;
    let marc = app.create_profile(&r, "Marc", 35, false).await;
    let (s, _) = app.delete(&format!("/api/profiles/{marc}"), &r).await;
    assert_eq!(s, StatusCode::FORBIDDEN, "PIN obligatoire");
    let (s, _) = app
        .call(
            axum::http::Method::DELETE,
            &format!("/api/profiles/{marc}"),
            Some(json!({"pin": "1234"})),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    let (_, cur) = app.get("/api/session", &r).await;
    assert!(
        cur["profile"].is_null(),
        "le compte actif supprimé : plus de profil"
    );
}

#[tokio::test]
async fn preferences_typees() {
    let app = TestApp::new();
    let r = app.pair().await;
    app.create_profile(&r, "Marc", 35, false).await;
    let (s, b) = app.get("/api/me/preferences", &r).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(b["preferences"]["theme"], "default");
    let (s, b) = app
        .patch(
            "/api/me/preferences",
            json!({"theme": "alternative", "subtitles": {"size": 1.5}}),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(b["preferences"]["theme"], "alternative");
    assert_eq!(b["preferences"]["subtitles"]["size"], 1.5);
    assert_eq!(
        b["preferences"]["subtitles"]["color"], "#FFFFFF",
        "le reste est gardé"
    );
    let (s, _) = app
        .patch("/api/me/preferences", json!({"text_scale": 9}), &r)
        .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    let (_, t) = app.get("/api/themes", &r).await;
    assert!(t["themes"].as_array().unwrap().len() >= 2);
}

#[tokio::test]
async fn sans_profil_les_routes_du_catalogue_refusent() {
    let app = TestApp::new();
    let r = app.pair().await;
    let (s, b) = app.get("/api/media", &r).await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    assert_eq!(b["error"]["code"], "profile_required");
    let _ = Who::Tv;
}
