//! Extensions : manifeste, types, permissions.
//!
//! Une extension n'est pas digne de confiance par défaut : elle déclare ses
//! permissions, l'utilisateur les accepte, tout le reste est refusé.

use std::collections::BTreeMap;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{DomainError, DomainResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionType {
    /// Ajoute une source de médias.
    Source,
    /// Fournit des informations sur les médias.
    Metadata,
    /// Élément sur l'écran d'accueil ou service.
    Widget,
}

impl ExtensionType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Metadata => "metadata",
            Self::Widget => "widget",
        }
    }
}

impl FromStr for ExtensionType {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "source" => Ok(Self::Source),
            "metadata" => Ok(Self::Metadata),
            "widget" => Ok(Self::Widget),
            _ => Err(DomainError::validation(
                "type",
                "source, metadata ou widget",
            )),
        }
    }
}

/// Permissions qu'une extension peut demander.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Permission {
    /// Accès à internet.
    #[serde(rename = "network")]
    Network,
    /// Lecture de la médiathèque.
    #[serde(rename = "library.read")]
    LibraryRead,
    /// Ajout de médias (extensions « source »).
    #[serde(rename = "library.write")]
    LibraryWrite,
    /// Contrôle de la lecture.
    #[serde(rename = "playback.control")]
    PlaybackControl,
    /// Réception des événements de lecture.
    #[serde(rename = "playback.events")]
    PlaybackEvents,
    /// Stockage de données propres à l'extension.
    #[serde(rename = "storage")]
    Storage,
    /// Affichage d'un élément sur l'écran d'accueil.
    #[serde(rename = "ui.home")]
    UiHome,
}

impl Permission {
    pub const ALL: [Permission; 7] = [
        Self::Network,
        Self::LibraryRead,
        Self::LibraryWrite,
        Self::PlaybackControl,
        Self::PlaybackEvents,
        Self::Storage,
        Self::UiHome,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Network => "network",
            Self::LibraryRead => "library.read",
            Self::LibraryWrite => "library.write",
            Self::PlaybackControl => "playback.control",
            Self::PlaybackEvents => "playback.events",
            Self::Storage => "storage",
            Self::UiHome => "ui.home",
        }
    }

    /// Texte montré à l'utilisateur avant l'installation.
    pub fn description(self) -> &'static str {
        match self {
            Self::Network => "Accéder à internet",
            Self::LibraryRead => "Lire votre médiathèque",
            Self::LibraryWrite => "Ajouter des médias à votre médiathèque",
            Self::PlaybackControl => "Contrôler la lecture",
            Self::PlaybackEvents => "Savoir ce qui est en cours de lecture",
            Self::Storage => "Enregistrer ses propres données",
            Self::UiHome => "Afficher un élément sur l'écran d'accueil",
        }
    }
}

impl FromStr for Permission {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|p| p.as_str() == s)
            .ok_or_else(|| {
                DomainError::validation("permissions", format!("permission inconnue : {s}"))
            })
    }
}

/// Permissions accordées : `{"network": true, "library.read": false}`.
/// Une permission absente est refusée.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Grants(pub BTreeMap<Permission, bool>);

impl Grants {
    pub fn allows(&self, p: Permission) -> bool {
        self.0.get(&p).copied().unwrap_or(false)
    }

    /// Accorde exactement `accepted` parmi les permissions `requested`.
    /// Accepter une permission non demandée est une erreur.
    pub fn from_acceptance(
        requested: &[Permission],
        accepted: &[Permission],
    ) -> DomainResult<Self> {
        if let Some(p) = accepted.iter().find(|p| !requested.contains(p)) {
            return Err(DomainError::validation(
                "permissions",
                format!("« {} » n'est pas demandée par l'extension", p.as_str()),
            ));
        }
        Ok(Self(
            requested
                .iter()
                .map(|p| (*p, accepted.contains(p)))
                .collect(),
        ))
    }

    /// Toutes les permissions demandées ont-elles été accordées ?
    pub fn covers(&self, requested: &[Permission]) -> bool {
        requested.iter().all(|p| self.allows(*p))
    }
}

/// Manifeste d'une extension (`manifest.toml` ou `manifest.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "type")]
    pub kind: ExtensionType,
    #[serde(default)]
    pub permissions: Vec<Permission>,
    /// Fichier de code (module WASM), relatif au dossier de l'extension.
    #[serde(default = "default_entry")]
    pub entry: String,
    /// Réglages proposés à l'utilisateur, avec leurs valeurs par défaut.
    #[serde(default)]
    pub settings: serde_json::Map<String, serde_json::Value>,
}

fn default_entry() -> String {
    "extension.wasm".into()
}

impl Manifest {
    pub fn validate(&self) -> DomainResult<()> {
        let id_ok = (3..=64).contains(&self.id.len())
            && self.id.chars().all(|c| {
                c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '-' | '_')
            })
            && !self.id.starts_with('.')
            && !self.id.contains("..");
        if !id_ok {
            return Err(DomainError::validation(
                "id",
                "3 à 64 caractères : minuscules, chiffres, « . », « - », « _ »",
            ));
        }
        if self.name.trim().is_empty() {
            return Err(DomainError::validation("name", "obligatoire"));
        }
        if parse_version(&self.version).is_none() {
            return Err(DomainError::validation("version", "format x.y.z attendu"));
        }
        if self.author.trim().is_empty() {
            return Err(DomainError::validation("author", "obligatoire"));
        }
        let entry_ok = !self.entry.is_empty()
            && !self.entry.starts_with('/')
            && !self.entry.contains('\\')
            && !self.entry.split('/').any(|p| p == "..");
        if !entry_ok {
            return Err(DomainError::validation("entry", "chemin relatif attendu"));
        }
        let mut seen = self.permissions.clone();
        seen.sort();
        seen.dedup();
        if seen.len() != self.permissions.len() {
            return Err(DomainError::validation("permissions", "en double"));
        }
        Ok(())
    }
}

/// Version sémantique simplifiée `x.y.z`.
pub fn parse_version(v: &str) -> Option<(u64, u64, u64)> {
    let mut parts = v.split('.').map(|p| p.parse::<u64>().ok());
    let (a, b, c) = (parts.next()??, parts.next()??, parts.next()??);
    if parts.next().is_some() {
        return None;
    }
    Some((a, b, c))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionStatus {
    #[default]
    Ok,
    /// Arrêtée par le superviseur (plantage, blocage, ressources).
    Crashed,
    Stopped,
}

impl ExtensionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Crashed => "crashed",
            Self::Stopped => "stopped",
        }
    }
}

impl FromStr for ExtensionStatus {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ok" => Ok(Self::Ok),
            "crashed" => Ok(Self::Crashed),
            "stopped" => Ok(Self::Stopped),
            _ => Err(DomainError::validation("status", s.to_string())),
        }
    }
}

/// Une extension installée (table `extension`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Extension {
    pub id: String,
    pub name: String,
    pub version: String,
    pub kind: ExtensionType,
    pub install_path: String,
    pub enabled: bool,
    pub status: ExtensionStatus,
    pub last_error: Option<String>,
    pub permissions: Grants,
    pub settings: serde_json::Map<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> Manifest {
        Manifest {
            id: "weather.widget".into(),
            name: "Météo".into(),
            version: "1.0.0".into(),
            author: "Équipe".into(),
            description: String::new(),
            kind: ExtensionType::Widget,
            permissions: vec![Permission::Network, Permission::UiHome],
            entry: "extension.wasm".into(),
            settings: Default::default(),
        }
    }

    #[test]
    fn manifeste_valide() {
        assert!(manifest().validate().is_ok());
    }

    #[test]
    fn manifeste_invalide() {
        let mut m = manifest();
        m.id = "../evil".into();
        assert!(m.validate().is_err());
        let mut m = manifest();
        m.version = "1.0".into();
        assert!(m.validate().is_err());
        let mut m = manifest();
        m.entry = "../../etc/passwd".into();
        assert!(m.validate().is_err());
        let mut m = manifest();
        m.permissions = vec![Permission::Network, Permission::Network];
        assert!(m.validate().is_err());
    }

    #[test]
    fn manifeste_toml_et_permissions_texte() {
        let m: Manifest = serde_json::from_str(
            r#"{"id":"a.b","name":"A","version":"0.1.0","author":"x","type":"metadata",
                "permissions":["network","library.read"]}"#,
        )
        .unwrap();
        assert_eq!(
            m.permissions,
            vec![Permission::Network, Permission::LibraryRead]
        );
        assert_eq!(m.entry, "extension.wasm");
    }

    #[test]
    fn permissions_refusees_par_defaut() {
        let g = Grants::from_acceptance(
            &[Permission::Network, Permission::LibraryRead],
            &[Permission::Network],
        )
        .unwrap();
        assert!(g.allows(Permission::Network));
        assert!(!g.allows(Permission::LibraryRead));
        assert!(!g.allows(Permission::Storage), "non demandée = refusée");
        assert!(!g.covers(&[Permission::Network, Permission::LibraryRead]));
    }

    #[test]
    fn accepter_une_permission_non_demandee_est_refuse() {
        assert!(Grants::from_acceptance(&[Permission::Network], &[Permission::Storage]).is_err());
    }

    #[test]
    fn grants_json() {
        let g = Grants::from_acceptance(&[Permission::LibraryRead], &[Permission::LibraryRead])
            .unwrap();
        assert_eq!(
            serde_json::to_string(&g).unwrap(),
            r#"{"library.read":true}"#
        );
    }

    #[test]
    fn versions() {
        assert_eq!(parse_version("1.2.3"), Some((1, 2, 3)));
        assert!(parse_version("1.2.3") > parse_version("1.2.0"));
        assert_eq!(parse_version("1.2"), None);
        assert_eq!(parse_version("1.2.3.4"), None);
    }
}
