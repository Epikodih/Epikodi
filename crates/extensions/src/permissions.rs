//! Permissions : ce qu'une extension peut demander à l'hôte.
//!
//! Le principe est celui d'un smartphone : l'extension déclare ses besoins,
//! l'utilisateur les accepte à l'installation, l'hôte refuse tout le reste.
//! Chaque appel de l'extension vers l'hôte passe par [`check`].

use serde::{Deserialize, Serialize};

use epikodi_domain::extension::{Grants, Permission};

use crate::ExtensionError;

/// Appels qu'une extension peut faire vers le media center.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "call", rename_all = "snake_case")]
pub enum HostCall {
    /// Requête HTTP(S) sortante.
    HttpGet { url: String },
    /// Recherche dans la médiathèque.
    LibrarySearch { query: String },
    /// Lecture d'une fiche.
    LibraryGet { media_id: i64 },
    /// Ajout d'un média (extension « source »).
    LibraryAdd {
        title: String,
        uri: String,
        kind: String,
    },
    /// Commande de lecture (lecture, pause...).
    PlaybackCommand { action: String },
    /// Abonnement aux événements de lecture.
    SubscribePlayback,
    /// Stockage clé/valeur propre à l'extension.
    StorageGet { key: String },
    StorageSet {
        key: String,
        value: serde_json::Value,
    },
    /// Élément sur l'écran d'accueil.
    HomeWidget { title: String, body: String },
    /// Journal (toujours autorisé).
    Log { message: String },
}

impl HostCall {
    /// Permission nécessaire (`None` = toujours autorisé).
    pub fn required_permission(&self) -> Option<Permission> {
        match self {
            Self::HttpGet { .. } => Some(Permission::Network),
            Self::LibrarySearch { .. } | Self::LibraryGet { .. } => Some(Permission::LibraryRead),
            Self::LibraryAdd { .. } => Some(Permission::LibraryWrite),
            Self::PlaybackCommand { .. } => Some(Permission::PlaybackControl),
            Self::SubscribePlayback => Some(Permission::PlaybackEvents),
            Self::StorageGet { .. } | Self::StorageSet { .. } => Some(Permission::Storage),
            Self::HomeWidget { .. } => Some(Permission::UiHome),
            Self::Log { .. } => None,
        }
    }
}

/// Vérifie qu'un appel est autorisé par les permissions accordées.
pub fn check(grants: &Grants, call: &HostCall) -> Result<(), ExtensionError> {
    match call.required_permission() {
        Some(p) if !grants.allows(p) => Err(ExtensionError::PermissionDenied(p)),
        _ => Ok(()),
    }
}

/// Une URL sortante doit être en HTTP(S), et jamais vers le réseau local
/// (l'extension ne doit pas pouvoir sonder la box ou le NAS).
pub fn check_outbound_url(url: &str) -> Result<(), ExtensionError> {
    let lower = url.to_ascii_lowercase();
    let rest = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
        .ok_or(ExtensionError::PermissionDenied(Permission::Network))?;
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = host.rsplit_once('@').map_or(host, |(_, h)| h);
    let host = host.split(':').next().unwrap_or("");
    let private = host == "localhost"
        || host.ends_with(".local")
        || host.starts_with("127.")
        || host.starts_with("10.")
        || host.starts_with("192.168.")
        || host.starts_with("169.254.")
        || host.starts_with('[')
        || (host.starts_with("172.")
            && host
                .split('.')
                .nth(1)
                .and_then(|o| o.parse::<u8>().ok())
                .is_some_and(|o| (16..=31).contains(&o)))
        || host == "0.0.0.0"
        || host.is_empty();
    if private {
        Err(ExtensionError::PermissionDenied(Permission::Network))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grants(p: &[Permission]) -> Grants {
        Grants::from_acceptance(p, p).unwrap()
    }

    #[test]
    fn meteo_ne_lit_pas_la_mediatheque() {
        let g = grants(&[Permission::Network, Permission::UiHome]);
        assert!(check(
            &g,
            &HostCall::HttpGet {
                url: "https://api".into()
            }
        )
        .is_ok());
        assert!(check(
            &g,
            &HostCall::HomeWidget {
                title: "x".into(),
                body: "y".into()
            }
        )
        .is_ok());
        assert!(matches!(
            check(&g, &HostCall::LibrarySearch { query: "x".into() }),
            Err(ExtensionError::PermissionDenied(Permission::LibraryRead))
        ));
        assert!(check(
            &g,
            &HostCall::LibraryAdd {
                title: "x".into(),
                uri: "y".into(),
                kind: "movie".into()
            }
        )
        .is_err());
        assert!(check(
            &g,
            &HostCall::Log {
                message: "ok".into()
            }
        )
        .is_ok());
    }

    #[test]
    fn permission_refusee_par_l_utilisateur() {
        let g = Grants::from_acceptance(&[Permission::Network], &[]).unwrap();
        assert!(check(
            &g,
            &HostCall::HttpGet {
                url: "https://x".into()
            }
        )
        .is_err());
    }

    #[test]
    fn reseau_local_interdit() {
        assert!(check_outbound_url("https://api.open-meteo.com/v1").is_ok());
        for bad in [
            "http://192.168.1.1/admin",
            "http://localhost:7878/api",
            "http://127.0.0.1",
            "http://nas.local/",
            "http://172.20.0.1",
            "http://user@10.0.0.1/",
            "file:///etc/passwd",
            "http://[::1]/",
        ] {
            assert!(check_outbound_url(bad).is_err(), "{bad}");
        }
        assert!(check_outbound_url("http://172.32.0.1").is_ok());
    }
}
