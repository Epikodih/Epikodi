//! Banc de test de l'API : base SQLite temporaire (vraies migrations),
//! horloge contrôlée, appels « TV locale » ou « télécommande sur le réseau ».

#![allow(dead_code)]

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use chrono::{DateTime, Utc};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

use epikodi_api::{router, ApiConfig, AppState};
use epikodi_domain::media::MediaKind;
use epikodi_domain::Id;
use epikodi_metadata::DemoCatalog;
use epikodi_storage::{insert_test_media, SecretBox, Store};

/// Qui appelle.
#[derive(Clone, Debug)]
pub enum Who {
    /// L'app TV : boucle locale, sans jeton.
    Tv,
    /// Une télécommande sur le réseau, avec son jeton.
    Remote(String),
    /// Quelqu'un sur le réseau, sans jeton.
    Stranger,
}

pub struct TestApp {
    pub state: AppState,
    pub router: Router,
    pub dir: tempfile::TempDir,
    pub now: Arc<Mutex<DateTime<Utc>>>,
}

pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

pub fn fixture(rel: &str) -> Value {
    let text =
        std::fs::read_to_string(fixtures_dir().join(rel)).unwrap_or_else(|e| panic!("{rel} : {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{rel} : {e}"))
}

impl TestApp {
    pub fn new() -> Self {
        Self::build(true)
    }

    /// Base pas encore ouverte (serveur qui démarre).
    pub fn starting() -> Self {
        Self::build(false)
    }

    fn build(open: bool) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("test.db");
        let factory = Arc::new(move || {
            let conn = epikodi_storage::init(&db)
                .map_err(|e| epikodi_domain::DomainError::Internal(e.to_string()))?;
            Ok(Store::new(conn, Some(SecretBox::from_key([1; 32]))))
        });
        let mut config = ApiConfig::new(dir.path());
        config.failure_delay = Duration::ZERO;
        config.lan_host = Some("192.168.1.42".into());
        config.extensions_catalog = Some(dir.path().join("catalog"));
        let now = Arc::new(Mutex::new(
            "2026-11-07T18:30:00Z".parse::<DateTime<Utc>>().unwrap(),
        ));
        let clock_now = now.clone();
        let state = AppState::new(config, factory, vec![Arc::new(DemoCatalog)])
            .with_clock(Arc::new(move || *clock_now.lock().unwrap()));
        if open {
            state.open_db().unwrap();
        }
        let router = router(state.clone());
        Self {
            state,
            router,
            dir,
            now,
        }
    }

    pub fn advance(&self, secs: i64) {
        *self.now.lock().unwrap() += chrono::Duration::seconds(secs);
    }

    pub async fn call(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
        who: &Who,
    ) -> (StatusCode, Value) {
        let mut req = Request::builder().method(method).uri(path);
        let ip: SocketAddr = match who {
            Who::Tv => "127.0.0.1:50000".parse().unwrap(),
            _ => "192.168.1.50:50000".parse().unwrap(),
        };
        if let Who::Remote(t) = who {
            req = req.header("Authorization", format!("Bearer {t}"));
        }
        let body = match body {
            Some(b) => {
                req = req.header("Content-Type", "application/json");
                Body::from(b.to_string())
            }
            None => Body::empty(),
        };
        let mut req = req.body(body).unwrap();
        req.extensions_mut().insert(ConnectInfo(ip));
        let res = self.router.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let json = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes)
                .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into()))
        };
        (status, json)
    }

    pub async fn get(&self, path: &str, who: &Who) -> (StatusCode, Value) {
        self.call(Method::GET, path, None, who).await
    }

    pub async fn post(&self, path: &str, body: Value, who: &Who) -> (StatusCode, Value) {
        self.call(Method::POST, path, Some(body), who).await
    }

    pub async fn patch(&self, path: &str, body: Value, who: &Who) -> (StatusCode, Value) {
        self.call(Method::PATCH, path, Some(body), who).await
    }

    pub async fn put(&self, path: &str, body: Value, who: &Who) -> (StatusCode, Value) {
        self.call(Method::PUT, path, Some(body), who).await
    }

    pub async fn delete(&self, path: &str, who: &Who) -> (StatusCode, Value) {
        self.call(Method::DELETE, path, None, who).await
    }

    /// La TV se connecte : (session, code affiché).
    pub async fn tv_connect(&self) -> (String, String) {
        let (s, b) = self
            .post(
                "/api/tv/connect",
                json!({"model": "Desktop linux", "platform": "linux", "app_version": "0.1.0"}),
                &Who::Tv,
            )
            .await;
        assert_eq!(s, StatusCode::CREATED, "{b}");
        (
            b["tv_session_id"].as_str().unwrap().into(),
            b["pairing"]["code"].as_str().unwrap().into(),
        )
    }

    /// Appairage complet d'une télécommande : renvoie son jeton.
    pub async fn pair(&self) -> Who {
        let (_, code) = self.tv_connect().await;
        let (s, b) = self
            .post(
                "/api/remote/pair",
                json!({"code": code, "device": {"name": "Pixel 7", "install_id": "inst-1", "platform": "android"}}),
                &Who::Stranger,
            )
            .await;
        assert_eq!(s, StatusCode::OK, "{b}");
        Who::Remote(b["token"].as_str().unwrap().into())
    }

    /// Crée un compte (il devient actif) et renvoie son id.
    pub async fn create_profile(&self, who: &Who, name: &str, age: i64, restricted: bool) -> Id {
        let mut body = json!({"name": name, "icon": "avatar_01", "language": "fr", "age": age, "is_restricted": restricted});
        if !restricted {
            body["pin"] = json!("1234");
        }
        let (s, b) = self.post("/api/profiles", body, who).await;
        assert_eq!(s, StatusCode::CREATED, "{b}");
        b["profile"]["id"].as_i64().unwrap()
    }

    pub async fn select(&self, who: &Who, id: Id, pin: Option<&str>) -> (StatusCode, Value) {
        let mut body = json!({"type": "account", "profile_id": id});
        if let Some(p) = pin {
            body["pin"] = json!(p);
        }
        self.post("/api/session/profile", body, who).await
    }

    pub async fn media(
        &self,
        parent: Option<Id>,
        kind: MediaKind,
        title: &str,
        rating: Option<u8>,
    ) -> Id {
        let title = title.to_string();
        self.state
            .db(move |s| Ok(insert_test_media(s, parent, kind, &title, rating)))
            .await
            .unwrap()
    }
}

/// Mêmes clés et mêmes types que la fixture (récursivement) ; les valeurs
/// peuvent différer. Comme `expectSameShape` côté Dart.
pub fn assert_same_shape(actual: &Value, expected: &Value, path: &str) {
    match expected {
        Value::Null => {}
        // champ facultatif (null) : sa forme n'est vérifiée que s'il est présent
        _ if actual.is_null() && !expected.is_boolean() => {}
        Value::Object(e) => {
            let a = actual
                .as_object()
                .unwrap_or_else(|| panic!("{path} doit être un objet : {actual}"));
            let mut ak: Vec<_> = a.keys().collect();
            let mut ek: Vec<_> = e.keys().collect();
            ak.sort();
            ek.sort();
            assert_eq!(ak, ek, "clés de {path}");
            for (k, v) in e {
                assert_same_shape(&a[k], v, &format!("{path}.{k}"));
            }
        }
        Value::Array(e) => {
            let a = actual
                .as_array()
                .unwrap_or_else(|| panic!("{path} doit être une liste"));
            if let Some(first) = e.first() {
                for item in a {
                    assert_same_shape(item, first, &format!("{path}[]"));
                }
            }
        }
        Value::Number(_) => assert!(
            actual.is_number() || actual.is_null(),
            "{path} doit être un nombre : {actual}"
        ),
        Value::String(_) => assert!(
            actual.is_string() || actual.is_null(),
            "{path} doit être un texte : {actual}"
        ),
        Value::Bool(_) => assert!(
            actual.is_boolean(),
            "{path} doit être un booléen : {actual}"
        ),
    }
}
