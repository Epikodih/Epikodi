mod config;

use std::{fs, net::SocketAddr, process, sync::Arc};

use epikodi_api::{router, AppState};

use config::Config;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = Config::load().unwrap_or_else(|e| {
        tracing::error!("configuration invalide : {e}");
        process::exit(1);
    });

    // --- Base de données ---
    let data_dir = &config.storage.data_dir;
    fs::create_dir_all(data_dir).unwrap_or_else(|e| {
        tracing::error!(dir = %data_dir.display(), "impossible de créer le dossier de données : {e}");
        process::exit(1);
    });
    let db_path = data_dir.join("epikodi.db");
    let _db = epikodi_storage::init(&db_path).unwrap_or_else(|e| {
        tracing::error!(path = %db_path.display(), "impossible d'ouvrir la base : {e}");
        process::exit(1);
    });
    tracing::info!(path = %db_path.display(), "base de données prête");

    // --- Réseau ---
    let host = config.server.host.clone().or_else(|| {
        local_ip_address::local_ip().ok().map(|ip| ip.to_string())
    });
    tracing::info!(?host, "adresse annoncée aux remotes");

    let state = Arc::new(AppState::new(host));

    let addr = SocketAddr::new(config.server.bind, config.server.port);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|e| {
            tracing::error!(%addr, "impossible d'écouter : {e}");
            process::exit(1);
        });
    tracing::info!(%addr, "serveur à l'écoute");

    axum::serve(
        listener,
        router(state).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("erreur du serveur");
}