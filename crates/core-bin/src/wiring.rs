//! Câblage : ouvre la base, choisit les fournisseurs, construit l'état de l'API.

use std::path::Path;
use std::sync::Arc;

use epikodi_api::{ApiConfig, AppState, StoreFactory};
use epikodi_domain::ports::MetadataProvider;
use epikodi_domain::DomainError;
use epikodi_metadata::{DemoCatalog, MusicBrainz, Tmdb, UreqHttp};
use epikodi_storage::{SecretBox, Store};

use crate::config::Config;

/// Variable d'environnement de la clé TMDB (jamais dans le dépôt).
pub const TMDB_KEY_ENV: &str = "TMDB_API_KEY";
const USER_AGENT: &str = concat!(
    "MediaCenterCore/",
    env!("CARGO_PKG_VERSION"),
    " (projet étudiant)"
);

/// Ouvre une connexion : crée la base si besoin, applique les migrations.
pub fn store_factory(data_dir: &Path) -> StoreFactory {
    let db_path = data_dir.join("epikodi.db");
    let key_path = data_dir.join("secret.key");
    Arc::new(move || {
        let conn = epikodi_storage::init(&db_path)
            .map_err(|e| DomainError::Internal(format!("base {} : {e}", db_path.display())))?;
        let secrets = SecretBox::load_or_create(&key_path)
            .map_err(|e| DomainError::Internal(format!("clé {} : {e}", key_path.display())))?;
        Ok(Store::new(conn, Some(secrets)))
    })
}

pub fn providers(config: &Config) -> Vec<Arc<dyn MetadataProvider>> {
    let http = Arc::new(UreqHttp::new(USER_AGENT));
    let mut list: Vec<Arc<dyn MetadataProvider>> = Vec::new();
    match std::env::var(TMDB_KEY_ENV) {
        Ok(key) if !key.trim().is_empty() => {
            tracing::info!("fournisseur TMDB activé");
            list.push(Arc::new(Tmdb::new(
                http.clone(),
                key.trim(),
                &config.metadata.language,
            )));
        }
        _ => tracing::info!("{TMDB_KEY_ENV} absente : pas d'enrichissement TMDB"),
    }
    if config.metadata.musicbrainz {
        list.push(Arc::new(MusicBrainz::new(http)));
    }
    if config.metadata.demo_catalog {
        list.push(Arc::new(DemoCatalog));
    }
    list
}

pub fn app_state(config: &Config, host: Option<String>) -> AppState {
    let data_dir = config.storage.data_dir.clone();
    let mut api = ApiConfig::new(&data_dir);
    api.lan_host = host;
    api.port = config.server.port;
    api.themes_dir = config.library.themes_dir.clone();
    api.extensions_catalog = config.library.extensions_catalog.clone();
    AppState::new(api, store_factory(&data_dir), providers(config))
}
