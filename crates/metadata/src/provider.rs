//! Accès HTTP commun aux fournisseurs.

use std::io::Read;
use std::time::Duration;

use serde_json::Value;

use epikodi_domain::ports::MetadataError;

/// Client HTTP minimal (remplacé par un faux dans les tests).
pub trait Http: Send + Sync {
    fn get_json(&self, url: &str, headers: &[(&str, &str)]) -> Result<Value, MetadataError>;
    fn get_bytes(&self, url: &str) -> Result<Vec<u8>, MetadataError>;
}

/// Taille maximale d'une image téléchargée.
const MAX_IMAGE_BYTES: u64 = 10 * 1024 * 1024;

pub struct UreqHttp {
    agent: ureq::Agent,
}

impl UreqHttp {
    pub fn new(user_agent: &str) -> Self {
        Self {
            agent: ureq::AgentBuilder::new()
                .timeout(Duration::from_secs(10))
                .user_agent(user_agent)
                .build(),
        }
    }
}

fn map_err(e: ureq::Error) -> MetadataError {
    match e {
        ureq::Error::Status(404, _) => MetadataError::NotFound,
        ureq::Error::Status(code, _) if code == 429 || code >= 500 => {
            MetadataError::Unavailable(format!("HTTP {code}"))
        }
        ureq::Error::Status(code, _) => MetadataError::Invalid(format!("HTTP {code}")),
        ureq::Error::Transport(t) => MetadataError::Unavailable(t.to_string()),
    }
}

impl Http for UreqHttp {
    fn get_json(&self, url: &str, headers: &[(&str, &str)]) -> Result<Value, MetadataError> {
        let mut req = self.agent.get(url).set("Accept", "application/json");
        for (k, v) in headers {
            req = req.set(k, v);
        }
        req.call()
            .map_err(map_err)?
            .into_json()
            .map_err(|e| MetadataError::Invalid(e.to_string()))
    }

    fn get_bytes(&self, url: &str) -> Result<Vec<u8>, MetadataError> {
        let resp = self.agent.get(url).call().map_err(map_err)?;
        let mut out = Vec::new();
        resp.into_reader()
            .take(MAX_IMAGE_BYTES)
            .read_to_end(&mut out)
            .map_err(|e| MetadataError::Unavailable(e.to_string()))?;
        Ok(out)
    }
}

/// Encode une valeur pour une URL (RFC 3986).
pub fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// Année au début d'une date (`2010-09-27` → 2010).
pub fn year_of(date: Option<&str>) -> Option<i32> {
    date.and_then(|d| d.get(..4)).and_then(|y| y.parse().ok())
}

pub fn str_field(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
}

#[cfg(test)]
pub(crate) mod fake {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Répond selon un morceau d'URL ; enregistre les URL demandées.
    #[derive(Default)]
    pub struct FakeHttp {
        pub routes: HashMap<&'static str, Value>,
        pub offline: bool,
        pub calls: Mutex<Vec<String>>,
    }

    impl Http for FakeHttp {
        fn get_json(&self, url: &str, _: &[(&str, &str)]) -> Result<Value, MetadataError> {
            self.calls.lock().unwrap().push(url.to_string());
            if self.offline {
                return Err(MetadataError::Unavailable("hors ligne".into()));
            }
            self.routes
                .iter()
                .filter(|(k, _)| url.contains(*k))
                .max_by_key(|(k, _)| k.len())
                .map(|(_, v)| v.clone())
                .ok_or(MetadataError::NotFound)
        }

        fn get_bytes(&self, _: &str) -> Result<Vec<u8>, MetadataError> {
            if self.offline {
                return Err(MetadataError::Unavailable("hors ligne".into()));
            }
            Ok(vec![0xFF, 0xD8])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodage_url() {
        assert_eq!(
            encode("Série Démo & co"),
            "S%C3%A9rie%20D%C3%A9mo%20%26%20co"
        );
    }

    #[test]
    fn annee() {
        assert_eq!(year_of(Some("2010-09-27")), Some(2010));
        assert_eq!(year_of(Some("")), None);
        assert_eq!(year_of(None), None);
    }
}
