//! Catalogue filtré par profil, fiches, progression, favoris, listes.

mod common;

use axum::http::StatusCode;
use serde_json::{json, Value};

use common::{assert_same_shape, fixture, TestApp, Who};
use epikodi_domain::media::MediaKind::*;
use epikodi_domain::Id;

struct Demo {
    sintel: Id,
    film16: Id,
    series: Id,
    ep1: Id,
    ep2: Id,
}

async fn demo(app: &TestApp) -> Demo {
    let sintel = app.media(None, Movie, "Sintel", Some(10)).await;
    app.media(None, Movie, "Big Buck Bunny", Some(0)).await;
    let film16 = app.media(None, Movie, "Film 16 ans", Some(16)).await;
    app.media(None, Movie, "mon_film_2019_final", None).await;
    let series = app.media(None, Series, "Série Démo", Some(10)).await;
    let s1 = app.media(Some(series), Season, "Saison 1", None).await;
    let ep1 = app.media(Some(s1), Episode, "Épisode 1", None).await;
    let ep2 = app.media(Some(s1), Episode, "Épisode 2", None).await;
    app.state
        .db(move |s| {
            s.conn()
                .execute(
                    "UPDATE media SET sort_number = 1, duration_s = 1000 WHERE id = ?1",
                    [ep1],
                )
                .unwrap();
            s.conn()
                .execute(
                    "UPDATE media SET sort_number = 2, duration_s = 1000 WHERE id = ?1",
                    [ep2],
                )
                .unwrap();
            s.conn()
                .execute("UPDATE media SET duration_s = 888 WHERE id = ?1", [sintel])
                .unwrap();
            Ok(())
        })
        .await
        .unwrap();
    Demo {
        sintel,
        film16,
        series,
        ep1,
        ep2,
    }
}

fn titles(b: &Value) -> Vec<String> {
    b["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["title"].as_str().unwrap().to_string())
        .collect()
}

/// Télécommande appairée avec Marc (adulte) et Léa (10 ans) ; Léa active.
async fn family(app: &TestApp) -> (Who, Id, Id) {
    let r = app.pair().await;
    let marc = app.create_profile(&r, "Marc", 35, false).await;
    let lea = app.create_profile(&r, "Léa", 10, true).await;
    (r, marc, lea)
}

#[tokio::test]
async fn liste_conforme_et_filtree_par_le_serveur() {
    let app = TestApp::new();
    let d = demo(&app).await;
    let (r, marc, _) = family(&app).await;

    // Léa (10 ans) : connu ET <= 10
    let (s, b) = app.get("/api/media", &r).await;
    assert_eq!(s, StatusCode::OK);
    assert_same_shape(&b, &fixture("media/list.json"), "$");
    assert_eq!(titles(&b), ["Big Buck Bunny", "Série Démo", "Sintel"]);
    assert_eq!(b["total"], 3);
    assert_eq!(b["limit"], 50);

    // un film masqué est introuvable (on ne révèle pas son existence)
    let (s, b) = app.get(&format!("/api/media/{}", d.film16), &r).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    assert_eq!(b["error"]["code"], "media_not_found");

    // Marc voit tout
    app.select(&r, marc, Some("1234")).await;
    let (_, b) = app.get("/api/media", &r).await;
    assert_eq!(b["total"], 5);
    assert_eq!(titles(&b)[3], "Série Démo", "tri sans accents");

    // l'invité : âge 0
    app.post("/api/session/profile", json!({"type": "guest"}), &r)
        .await;
    let (_, b) = app.get("/api/media", &r).await;
    assert_eq!(titles(&b), ["Big Buck Bunny"]);
}

#[tokio::test]
async fn parametres_de_liste() {
    let app = TestApp::new();
    let d = demo(&app).await;
    let (r, marc, _) = family(&app).await;
    app.select(&r, marc, Some("1234")).await;

    let (_, b) = app.get("/api/media?limit=2&offset=1", &r).await;
    assert_eq!(b["items"].as_array().unwrap().len(), 2);
    assert_eq!(
        (
            b["limit"].as_i64(),
            b["offset"].as_i64(),
            b["total"].as_i64()
        ),
        (Some(2), Some(1), Some(5))
    );

    let (_, b) = app.get("/api/media?kind=episode", &r).await;
    assert_eq!(b["total"], 2);

    let (_, seasons) = app
        .get(&format!("/api/media?parent_id={}", d.series), &r)
        .await;
    let season = seasons["items"][0]["id"].as_i64().unwrap();
    let (_, eps) = app.get(&format!("/api/media?parent_id={season}"), &r).await;
    assert_eq!(
        titles(&eps),
        ["Épisode 1", "Épisode 2"],
        "tri par sort_number"
    );

    let (_, b) = app.get("/api/media?q=sint", &r).await;
    assert_eq!(titles(&b), ["Sintel"]);

    for bad in ["limit=0", "limit=abc", "kind=film", "sort=n", "offset=-1"] {
        let (s, b) = app.get(&format!("/api/media?{bad}"), &r).await;
        assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY, "{bad}");
        assert_eq!(b["error"]["code"], "invalid_query", "{bad}");
    }
    let (s, b) = app.get("/api/media?parent_id=9999", &r).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    assert_eq!(b["error"]["code"], "media_not_found");
}

#[tokio::test]
async fn fiche_progression_reprise_et_vu() {
    let app = TestApp::new();
    let d = demo(&app).await;
    let (r, marc, _) = family(&app).await;
    app.select(&r, marc, Some("1234")).await;

    let (s, b) = app
        .put(
            &format!("/api/media/{}/progress", d.sintel),
            json!({"position_s": 312}),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(b["resume_at_s"], 312);
    assert_eq!(b["is_watched"], false);

    app.put(&format!("/api/favorites/{}", d.sintel), json!(null), &r)
        .await;
    let (s, b) = app.get(&format!("/api/media/{}", d.sintel), &r).await;
    assert_eq!(s, StatusCode::OK);
    let mut expected = fixture("media/detail.json");
    expected["details"] = Value::Null; // fiche libre (selon le fournisseur)
    assert_same_shape(&b, &expected, "$");
    assert_eq!(b["progress"]["position_s"], 312);
    assert_eq!(b["is_favorite"], true);
    assert!(b["playback_uri"].is_null(), "réservé à l'app TV");

    // reprise sur l'accueil
    let (_, home) = app.get("/api/home", &r).await;
    assert_eq!(home["continue_watching"][0]["media"]["id"], d.sintel);
    assert_eq!(home["favorites"][0]["id"], d.sintel);

    // presque fini : vu, et plus dans la reprise
    app.put(
        &format!("/api/media/{}/progress", d.sintel),
        json!({"position_s": 870}),
        &r,
    )
    .await;
    let (_, b) = app.get(&format!("/api/media/{}", d.sintel), &r).await;
    assert_eq!(b["is_watched"], true);
    let (_, c) = app.get("/api/media/continue", &r).await;
    assert_eq!(c["items"], json!([]));

    // série vue = tous ses épisodes vus
    let (_, b) = app
        .put(
            &format!("/api/media/{}/watched", d.series),
            json!({"watched": true}),
            &r,
        )
        .await;
    assert_eq!(b["updated"], 2);
    let (_, b) = app.get(&format!("/api/media/{}", d.series), &r).await;
    assert_eq!(b["is_watched"], true);
    assert_eq!(b["children_count"], 1);

    // l'app TV reçoit le chemin à lire
    let (_, b) = app.get(&format!("/api/media/{}", d.ep1), &Who::Tv).await;
    assert!(b["playback_uri"].is_string());
    assert_eq!(
        b["ancestors"].as_array().unwrap().len(),
        2,
        "série > saison"
    );
}

#[tokio::test]
async fn progression_par_profil_et_invite() {
    let app = TestApp::new();
    let d = demo(&app).await;
    let (r, marc, _) = family(&app).await;
    // Léa
    app.put(
        &format!("/api/media/{}/progress", d.sintel),
        json!({"position_s": 100}),
        &r,
    )
    .await;
    app.select(&r, marc, Some("1234")).await;
    let (_, b) = app.get(&format!("/api/media/{}", d.sintel), &r).await;
    assert!(b["progress"].is_null(), "chaque profil a sa progression");
    app.post("/api/session/profile", json!({"type": "guest"}), &r)
        .await;
    let (s, b) = app
        .put(
            &format!("/api/media/{}/progress", d.ep1),
            json!({"position_s": 10}),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    assert_eq!(b["error"]["code"], "guest_forbidden");
}

#[tokio::test]
async fn episode_suivant() {
    let app = TestApp::new();
    let d = demo(&app).await;
    let (r, ..) = family(&app).await;
    let (_, b) = app.get(&format!("/api/media/{}/next", d.ep1), &r).await;
    assert_eq!(b["media"]["id"], d.ep2);
    let (_, b) = app.get(&format!("/api/media/{}/next", d.ep2), &r).await;
    assert!(b["media"].is_null());
}

#[tokio::test]
async fn correction_manuelle_reservee_aux_adultes() {
    let app = TestApp::new();
    let id = app.media(None, Movie, "mon_film_2019_final", None).await;
    let (r, marc, _) = family(&app).await;
    let (s, b) = app
        .patch(
            &format!("/api/media/{id}"),
            json!({"title": "Mon film"}),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    assert_eq!(b["error"]["code"], "admin_required");

    app.select(&r, marc, Some("1234")).await;
    let (s, b) = app
        .patch(
            &format!("/api/media/{id}"),
            json!({"title": "Mon film", "year": 2019, "age_rating": 12}),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(b["match_status"], "manual");
    assert_eq!(b["age_rating"], 12);

    // résultats proposés (catalogue démo), puis choix
    let big = app.media(None, Movie, "big buck", None).await;
    let (s, b) = app.get(&format!("/api/media/{big}/matches"), &r).await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(b["candidates"][0]["external_id"], "big-buck-bunny");
    let (s, b) = app
        .post(
            &format!("/api/media/{big}/match"),
            json!({"provider": "demo", "external_id": "big-buck-bunny"}),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(b["title"], "Big Buck Bunny");
    assert_eq!(b["year"], 2008);
    assert_eq!(b["match_status"], "manual");
    let (_, g) = app.get("/api/media/genres", &r).await;
    assert!(g["genres"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["value"] == "Animation"));
}

#[tokio::test]
async fn listes_de_lecture() {
    let app = TestApp::new();
    let d = demo(&app).await;
    let (r, ..) = family(&app).await;
    let (s, p) = app
        .post("/api/playlists", json!({"name": "Soirée"}), &r)
        .await;
    assert_eq!(s, StatusCode::CREATED);
    let pid = p["id"].as_i64().unwrap();
    let (s, b) = app
        .post(
            &format!("/api/playlists/{pid}/items"),
            json!({"media_ids": [d.sintel, d.ep1]}),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(b["item_count"], 2);
    // un média masqué ne peut pas être ajouté
    let (s, _) = app
        .post(
            &format!("/api/playlists/{pid}/items"),
            json!({"media_ids": [d.film16]}),
            &r,
        )
        .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    let (_, b) = app.get(&format!("/api/playlists/{pid}"), &r).await;
    assert_eq!(b["items"].as_array().unwrap().len(), 2);
    assert_eq!(
        app.delete(&format!("/api/playlists/{pid}/items/0"), &r)
            .await
            .0,
        StatusCode::NO_CONTENT
    );
    let (_, b) = app
        .patch(
            &format!("/api/playlists/{pid}"),
            json!({"name": "Ciné"}),
            &r,
        )
        .await;
    assert_eq!(
        (b["name"].as_str(), b["item_count"].as_i64()),
        (Some("Ciné"), Some(1))
    );
    let (s, b) = app
        .post("/api/playlists", json!({"name": "Ciné"}), &r)
        .await;
    assert_eq!(s, StatusCode::CONFLICT);
    assert_eq!(b["error"]["code"], "playlist_name_taken");
    assert_eq!(
        app.delete(&format!("/api/playlists/{pid}"), &r).await.0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        app.get(&format!("/api/playlists/{pid}"), &r).await.0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn affiche_absente() {
    let app = TestApp::new();
    let d = demo(&app).await;
    let (r, ..) = family(&app).await;
    let (s, b) = app
        .get(&format!("/api/media/{}/poster", d.sintel), &r)
        .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    assert_eq!(b["error"]["code"], "poster_not_found");
}
