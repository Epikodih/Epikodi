//! Système d'extensions.
//!
//! - [`manifest`] : lecture et validation de `manifest.toml` / `manifest.json` ;
//! - [`permissions`] : ce qu'une extension a le droit de demander à l'hôte ;
//! - [`lifecycle`] : installer, mettre à jour, activer, configurer, désinstaller,
//!   sans redémarrer le produit ;
//! - [`runtime`] : le contrat d'exécution d'une extension (isolée) ;
//! - [`supervisor`] : une extension qui plante, se bloque ou abuse est arrêtée
//!   sans faire planter le media center.

pub mod lifecycle;
pub mod manifest;
pub mod permissions;
pub mod runtime;
pub mod supervisor;

use epikodi_domain::extension::Permission;
use epikodi_domain::DomainError;

#[derive(Debug, thiserror::Error)]
pub enum ExtensionError {
    #[error("manifeste invalide : {0}")]
    InvalidManifest(String),
    #[error("paquet invalide : {0}")]
    InvalidPackage(String),
    #[error("extension « {0} » déjà installée")]
    AlreadyInstalled(String),
    #[error("extension « {0} » introuvable")]
    NotFound(String),
    #[error("version {new} pas plus récente que {current}")]
    NotNewer { current: String, new: String },
    #[error("permission refusée : {}", .0.as_str())]
    PermissionDenied(Permission),
    #[error("réglage invalide : {0}")]
    InvalidSetting(String),
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Domain(#[from] DomainError),
}
