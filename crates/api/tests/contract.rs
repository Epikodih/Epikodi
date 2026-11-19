//! Contrat avec les apps : `fixtures/` est la référence.
//!
//! - chaque fixture se décode dans sa structure Rust et se réencode à
//!   l'identique (si un champ change d'un côté, ce test casse) ;
//! - chaque route de `routes.json` existe dans le vrai routeur ;
//! - les codes d'erreur annoncés sont ceux que le serveur renvoie.

mod common;

use std::collections::BTreeSet;

use axum::http::{Method, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use common::{fixture, fixtures_dir, TestApp, Who};
use epikodi_domain::protocol::*;

fn round_trip<T: DeserializeOwned + Serialize>(rel: &str) {
    let original = fixture(rel);
    let decoded: T = serde_json::from_value(original.clone())
        .unwrap_or_else(|e| panic!("{rel} ne se décode pas : {e}"));
    let encoded = serde_json::to_value(&decoded).unwrap();
    assert_eq!(
        normalize(&encoded),
        normalize(&original),
        "{rel} : aller-retour différent"
    );
}

/// 888 et 888.0 sont le même nombre JSON.
fn normalize(v: &Value) -> Value {
    match v {
        Value::Number(n) => n.as_f64().map(Value::from).unwrap_or(v.clone()),
        Value::Array(a) => Value::Array(a.iter().map(normalize).collect()),
        Value::Object(o) => {
            Value::Object(o.iter().map(|(k, v)| (k.clone(), normalize(v))).collect())
        }
        _ => v.clone(),
    }
}

/// Fixture → structure Rust qui la porte.
const COVERED: &[&str] = &[
    "errors/error.json",
    "health/ok.json",
    "tv/connect_request.json",
    "tv/connect_response.json",
    "tv/refresh_request.json",
    "tv/refresh_response.json",
    "tv/status.json",
    "pairing/request.json",
    "pairing/response.json",
    "profiles/list.json",
    "profiles/create_request.json",
    "profiles/create_response.json",
    "profiles/get_response.json",
    "profiles/update_request.json",
    "profiles/update_response.json",
    "session/current.json",
    "session/select_request.json",
    "session/select_response.json",
    "media/list.json",
    "media/detail.json",
    "player/state.json",
    "player/command_request.json",
    "queue/list.json",
    "queue/add_request.json",
];

#[test]
fn chaque_fixture_fait_l_aller_retour() {
    round_trip::<ErrorResponse>("errors/error.json");
    round_trip::<HealthResponse>("health/ok.json");
    round_trip::<TvConnectRequest>("tv/connect_request.json");
    round_trip::<TvConnectResponse>("tv/connect_response.json");
    round_trip::<TvRefreshRequest>("tv/refresh_request.json");
    round_trip::<TvRefreshResponse>("tv/refresh_response.json");
    round_trip::<TvStatus>("tv/status.json");
    round_trip::<PairRequest>("pairing/request.json");
    round_trip::<PairResponse>("pairing/response.json");
    round_trip::<ProfilesList>("profiles/list.json");
    round_trip::<CreateProfileRequest>("profiles/create_request.json");
    round_trip::<CreateProfileResponse>("profiles/create_response.json");
    round_trip::<GetProfileResponse>("profiles/get_response.json");
    round_trip::<UpdateProfileRequest>("profiles/update_request.json");
    round_trip::<ProfileResponse>("profiles/update_response.json");
    round_trip::<SessionCurrent>("session/current.json");
    round_trip::<SelectProfileRequest>("session/select_request.json");
    round_trip::<ProfileResponse>("session/select_response.json");
    round_trip::<MediaList>("media/list.json");
    round_trip::<MediaDetailResponse>("media/detail.json");
    round_trip::<PlayerState>("player/state.json");
    round_trip::<PlayerCommand>("player/command_request.json");
    round_trip::<QueueView>("queue/list.json");
    round_trip::<EnqueueRequest>("queue/add_request.json");
}

#[test]
fn aucune_fixture_oubliee() {
    let mut found = BTreeSet::new();
    let root = fixtures_dir();
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "json") {
                let rel = p
                    .strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                if rel != "routes.json" {
                    found.insert(rel);
                }
            }
        }
    }
    let covered: BTreeSet<String> = COVERED.iter().map(|s| s.to_string()).collect();
    let missing: Vec<_> = found.difference(&covered).collect();
    assert!(missing.is_empty(), "fixtures sans test Rust : {missing:?}");
}

#[tokio::test]
async fn chaque_route_de_routes_json_existe() {
    let app = TestApp::new();
    let routes = fixture("routes.json");
    for r in routes["routes"].as_array().unwrap() {
        let method: Method = r["method"].as_str().unwrap().parse().unwrap();
        let path = r["path"].as_str().unwrap().replace("{id}", "1");
        let body = r["request"].as_str().map(fixture);
        let (status, res) = app.call(method.clone(), &path, body, &Who::Tv).await;
        assert_ne!(status, StatusCode::METHOD_NOT_ALLOWED, "{method} {path}");
        assert_ne!(
            res["error"]["code"], "route_not_found",
            "{method} {path} absente du routeur"
        );
    }
}

#[tokio::test]
async fn erreurs_au_format_commun() {
    let app = TestApp::new();
    let (status, body) = app.get("/api/profiles", &Who::Stranger).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    common::assert_same_shape(&body, &fixture("errors/error.json"), "$");
    assert_eq!(body["error"]["code"], "unauthorized");
    let (status, body) = app.get("/api/nimporte/quoi", &Who::Tv).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "route_not_found");
}
