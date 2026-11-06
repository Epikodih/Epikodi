use std::{env, fs, io::ErrorKind, net::IpAddr, path::PathBuf};

use serde::Deserialize;

/// Fichier lu par défaut (relatif au dossier depuis lequel on lance le serveur).
const DEFAULT_PATH: &str = "epikodi.toml";

/// Variable d'environnement pour utiliser un autre fichier.
const PATH_ENV: &str = "EPIKODI_CONFIG";

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub server: ServerConfig,
    pub storage: StorageConfig,
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ServerConfig {
    /// Adresse d'écoute. `0.0.0.0` = accessible depuis le réseau local.
    pub bind: IpAddr,
    pub port: u16,
    /// IP annoncée aux remotes (dans le QR code). Détectée si absente.
    pub host: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: IpAddr::from([0, 0, 0, 0]),
            port: 7878,
            host: None,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StorageConfig {
    /// Dossier de données, qui contient `epikodi.db`.
    pub data_dir: PathBuf,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("data"),
        }
    }
}

impl Config {
    /// Charge la configuration. Un fichier absent donne les valeurs par défaut ;
    /// un fichier illisible ou invalide est une erreur.
    pub fn load() -> Result<Self, String> {
        let path = env::var_os(PATH_ENV)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_PATH));

        match fs::read_to_string(&path) {
            Ok(text) => {
                tracing::info!(path = %path.display(), "configuration chargée");
                toml::from_str(&text).map_err(|e| format!("{} : {e}", path.display()))
            }
            Err(e) if e.kind() == ErrorKind::NotFound => {
                tracing::warn!(
                    path = %path.display(),
                    "fichier de configuration introuvable, valeurs par défaut"
                );
                Ok(Self::default())
            }
            Err(e) => Err(format!("{} : {e}", path.display())),
        }
    }
}