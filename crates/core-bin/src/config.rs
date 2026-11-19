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
    pub library: LibraryConfig,
    pub metadata: MetadataConfig,
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ServerConfig {
    /// Adresse d'écoute. `0.0.0.0` = accessible depuis le réseau local.
    pub bind: IpAddr,
    pub port: u16,
    /// IP annoncée aux remotes (dans le QR code). Détectée si absente.
    pub host: Option<String>,
    /// Nom annoncé en mDNS (découverte par la télécommande).
    pub name: String,
    /// Annonce mDNS / Zeroconf activée.
    pub mdns: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: IpAddr::from([0, 0, 0, 0]),
            port: 7878,
            host: None,
            name: "Media center".into(),
            mdns: true,
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

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LibraryConfig {
    /// Thèmes proposés (un sous-dossier par thème).
    pub themes_dir: Option<PathBuf>,
    /// Catalogue local d'extensions installables.
    pub extensions_catalog: Option<PathBuf>,
}

impl Default for LibraryConfig {
    fn default() -> Self {
        Self {
            themes_dir: Some(PathBuf::from("themes")),
            extensions_catalog: Some(PathBuf::from("extensions")),
        }
    }
}

/// La clé TMDB n'est JAMAIS dans ce fichier : variable `TMDB_API_KEY`.
#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MetadataConfig {
    /// Langue des fiches (TMDB).
    pub language: String,
    /// Interroger MusicBrainz pour la musique.
    pub musicbrainz: bool,
    /// Catalogue démo intégré (films libres), utile hors connexion.
    pub demo_catalog: bool,
}

impl Default for MetadataConfig {
    fn default() -> Self {
        Self {
            language: "fr-FR".into(),
            musicbrainz: true,
            demo_catalog: true,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fichier_du_depot_valide() {
        let text = include_str!("../../../epikodi.toml");
        let c: Config = toml::from_str(text).unwrap();
        assert_eq!(c.server.port, 7878);
    }

    #[test]
    fn cle_inconnue_refusee() {
        assert!(toml::from_str::<Config>("[server]\nportt = 1").is_err());
    }

    #[test]
    fn valeurs_par_defaut() {
        let c: Config = toml::from_str("").unwrap();
        assert_eq!(c.server.port, 7878);
        assert!(c.metadata.demo_catalog);
    }
}
