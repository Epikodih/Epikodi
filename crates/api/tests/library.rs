//! Sources, scan réel d'un dossier, extensions.

mod common;

use std::fs;
use std::time::Duration;

use axum::http::StatusCode;
use serde_json::{json, Value};

use common::{TestApp, Who};

async fn admin(app: &TestApp) -> Who {
    let r = app.pair().await;
    app.create_profile(&r, "Marc", 35, false).await;
    r
}

async fn wait_scan(app: &TestApp, who: &Who) -> Value {
    for _ in 0..100 {
        let (_, s) = app.get("/api/library/scan", who).await;
        if s["running"] == false && s["finished_at"].is_string() {
            return s;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("scan trop long");
}

fn file_uri(p: &std::path::Path) -> String {
    format!("file://{}", p.to_string_lossy().replace('\\', "/"))
}

#[tokio::test]
async fn ajouter_une_source_et_scanner() {
    let app = TestApp::new();
    let r = admin(&app).await;
    let media = app.dir.path().join("media");
    fs::create_dir_all(media.join("Films")).unwrap();
    fs::create_dir_all(media.join("Séries/Série Démo/Saison 1")).unwrap();
    fs::write(media.join("Films/Sintel (2010).mkv"), b"sintel").unwrap();
    fs::write(media.join("Films/big_buck_bunny_1080p.mp4"), b"bunny").unwrap();
    fs::write(
        media.join("Séries/Série Démo/Saison 1/Série Démo - S01E01.mkv"),
        b"ep1",
    )
    .unwrap();
    fs::write(media.join("Films/notes.txt"), b"x").unwrap();

    let (s, src) = app
        .post(
            "/api/sources",
            json!({"name": "Démo", "protocol": "local", "uri": file_uri(&media)}),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::CREATED, "{src}");
    let sid = src["id"].as_i64().unwrap();
    assert_eq!(src["has_credentials"], false);

    let scan = wait_scan(&app, &r).await;
    assert_eq!(scan["added"], 3, "{scan}");
    assert!(
        scan["enriched"].as_u64().unwrap() >= 2,
        "catalogue démo : {scan}"
    );

    let (_, list) = app.get("/api/media?kind=movie", &r).await;
    let titles: Vec<_> = list["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["title"].as_str().unwrap())
        .collect();
    assert_eq!(titles, ["Big Buck Bunny", "Sintel"]);
    assert!(list["items"]
        .as_array()
        .unwrap()
        .iter()
        .all(|i| i["match_status"] == "auto"));
    let (_, series) = app.get("/api/media?kind=series", &r).await;
    assert_eq!(series["items"][0]["title"], "Série Démo");

    let (_, srcs) = app.get("/api/sources", &r).await;
    assert_eq!(srcs["sources"][0]["status"], "online");

    // fichier supprimé : gardé, marqué disparu
    fs::remove_file(media.join("Films/Sintel (2010).mkv")).unwrap();
    let (s, _) = app
        .post(&format!("/api/sources/{sid}/scan"), json!(null), &r)
        .await;
    assert_eq!(s, StatusCode::ACCEPTED);
    let scan = wait_scan(&app, &r).await;
    assert_eq!(scan["missing"], 1, "{scan}");
    let (_, list) = app.get("/api/media?kind=movie", &r).await;
    let sintel = list["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["title"] == "Sintel")
        .unwrap()
        .clone();
    assert_eq!(sintel["status"], "missing");

    // dossier racine supprimé : rien n'est marqué disparu de plus
    fs::remove_dir_all(&media).unwrap();
    app.post("/api/library/scan", json!(null), &r).await;
    let scan = wait_scan(&app, &r).await;
    assert_eq!(scan["missing"], 0);
    assert!(!scan["errors"].as_array().unwrap().is_empty());

    // suppression de la source
    assert_eq!(
        app.delete(&format!("/api/sources/{sid}"), &r).await.0,
        StatusCode::NO_CONTENT
    );
    let (_, list) = app.get("/api/media", &r).await;
    assert_eq!(list["total"], 0);
}

#[tokio::test]
async fn sources_reservees_aux_adultes_et_identifiants_caches() {
    let app = TestApp::new();
    let r = app.pair().await;
    app.create_profile(&r, "Léa", 10, true).await;
    let (s, b) = app
        .post(
            "/api/sources",
            json!({"name": "NAS", "protocol": "smb", "uri": "smb://nas/films"}),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    assert_eq!(b["error"]["code"], "admin_required");

    let r = admin(&app).await;
    let (s, b) = app
        .post(
            "/api/sources",
            json!({"name": "NAS", "protocol": "smb", "uri": "smb://nas/films",
                   "credentials": {"username": "marc", "password": "TresSecret"}}),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::CREATED, "{b}");
    assert_eq!(b["has_credentials"], true);
    assert!(!b.to_string().contains("TresSecret"), "jamais renvoyé");
    let id = b["id"].as_i64().unwrap();
    wait_scan(&app, &r).await; // partage non monté : statut offline, sans planter
    let (_, src) = app.get(&format!("/api/sources/{id}"), &r).await;
    assert_eq!(src["status"], "offline");
    let (_, b) = app
        .patch(
            &format!("/api/sources/{id}"),
            json!({"credentials": null}),
            &r,
        )
        .await;
    assert_eq!(b["has_credentials"], false);

    let (s, b) = app
        .post(
            "/api/sources",
            json!({"name": "X", "protocol": "smb", "uri": "file:///x"}),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(b["error"]["details"]["fields"]["uri"].is_string());
    let (s, b) = app
        .post(
            "/api/sources",
            json!({"name": "NAS2", "protocol": "smb", "uri": "smb://nas/films"}),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::CONFLICT);
    assert_eq!(b["error"]["code"], "source_exists");
}

fn write_extension(dir: &std::path::Path, version: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("manifest.toml"),
        format!(
            "id = \"weather.widget\"\nname = \"Météo\"\nversion = \"{version}\"\nauthor = \"Équipe\"\n\
             type = \"widget\"\npermissions = [\"network\", \"ui.home\"]\n\n[settings]\ncity = \"Paris\"\n"
        ),
    )
    .unwrap();
    fs::write(dir.join("extension.wasm"), b"\0asm\x01\0\0\0").unwrap();
}

#[tokio::test]
async fn cycle_de_vie_d_une_extension() {
    let app = TestApp::new();
    let r = admin(&app).await;
    let catalog = app.dir.path().join("catalog");
    write_extension(&catalog.join("weather"), "1.0.0");

    let (s, cat) = app.get("/api/extensions/catalog", &r).await;
    assert_eq!(s, StatusCode::OK);
    let entry = &cat["extensions"][0];
    assert_eq!(entry["id"], "weather.widget");
    assert_eq!(entry["permissions"][0]["description"], "Accéder à internet");
    let path = entry["path"].as_str().unwrap().to_string();

    let (s, e) = app
        .post(
            "/api/extensions",
            json!({"path": path, "accept_permissions": ["network", "ui.home"]}),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::CREATED, "{e}");
    assert_eq!(e["enabled"], true);
    assert_eq!(e["status"], "ok");

    // widget sur l'accueil
    let (_, home) = app.get("/api/home", &r).await;
    assert_eq!(home["widgets"][0]["extension_id"], "weather.widget");

    // réglages, désactivation
    let (_, e) = app
        .patch(
            "/api/extensions/weather.widget",
            json!({"settings": {"city": "Lyon"}, "enabled": false}),
            &r,
        )
        .await;
    assert_eq!(e["settings"]["city"], "Lyon");
    assert_eq!(e["enabled"], false);
    let (_, home) = app.get("/api/home", &r).await;
    assert_eq!(home["widgets"], json!([]));
    let (s, _) = app
        .patch(
            "/api/extensions/weather.widget",
            json!({"settings": {"inconnu": 1}}),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);

    // mise à jour sans redémarrer, réglages gardés
    let v2 = app.dir.path().join("v2");
    write_extension(&v2, "1.1.0");
    let (s, e) = app
        .post(
            "/api/extensions/weather.widget/update",
            json!({"path": v2.to_string_lossy(), "accept_permissions": ["network"]}),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{e}");
    assert_eq!(e["version"], "1.1.0");
    assert_eq!(e["settings"]["city"], "Lyon");
    let ui = e["permissions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "ui.home")
        .unwrap()
        .clone();
    assert_eq!(ui["granted"], false);

    // déjà installée
    let (s, b) = app.post("/api/extensions", json!({"path": path}), &r).await;
    assert_eq!(s, StatusCode::CONFLICT);
    assert_eq!(b["error"]["code"], "extension_already_installed");

    // désinstallation
    assert_eq!(
        app.delete("/api/extensions/weather.widget", &r).await.0,
        StatusCode::NO_CONTENT
    );
    let (_, list) = app.get("/api/extensions", &r).await;
    assert_eq!(list["extensions"], json!([]));
    assert_eq!(
        app.get("/api/extensions/weather.widget", &r).await.0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn paquet_invalide() {
    let app = TestApp::new();
    let r = admin(&app).await;
    let bad = app.dir.path().join("bad");
    fs::create_dir_all(&bad).unwrap();
    let (s, b) = app
        .post(
            "/api/extensions",
            json!({"path": bad.to_string_lossy()}),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(b["error"]["code"], "extension_invalid");
    let (s, _) = app
        .post("/api/extensions", json!({"path": "/n/existe/pas"}), &r)
        .await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
}
