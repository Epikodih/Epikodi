//! État partagé de l'API.
//!
//! - La base (`Store`) est derrière un mutex (un seul écrivain) et appelée
//!   depuis `spawn_blocking` ; tant qu'elle n'est pas ouverte, l'API répond
//!   « pas prêt » (503).
//! - L'appairage, le profil actif, les tentatives de PIN et l'état du lecteur
//!   sont en mémoire.
//! - Les événements temps réel passent par un canal `broadcast` (WebSocket) ;
//!   l'app TV suit son état par un compteur `watch` (SSE).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use chrono::{DateTime, Utc};
use tokio::sync::{broadcast, watch};

use epikodi_domain::ports::{AppStateRepo, MetadataProvider, ProfileRepo};
use epikodi_domain::profile::{ActiveProfile, Profile};
use epikodi_domain::protocol::{Event, ScanStatus};
use epikodi_domain::{DomainResult, Id};
use epikodi_storage::cache::FsImageCache;
use epikodi_storage::Store;

use crate::auth::pairing::{PairingConfig, PairingManager};
use crate::error::{ApiError, ApiResult};
use crate::http::player::PlayerRuntime;

/// Réglages de l'API.
#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub version: String,
    /// IP annoncée aux télécommandes (QR code). `None` : 127.0.0.1.
    pub lan_host: Option<String>,
    pub port: u16,
    /// Dossier de données (base, cache, extensions installées).
    pub data_dir: PathBuf,
    /// Catalogue local d'extensions proposées à l'installation.
    pub extensions_catalog: Option<PathBuf>,
    /// Dossier des thèmes (un sous-dossier par thème).
    pub themes_dir: Option<PathBuf>,
    pub pairing: PairingConfig,
    /// Délai ajouté après chaque échec (code, PIN), croissant.
    pub failure_delay: Duration,
    /// Durée de vie d'un jeton d'appareil.
    pub token_ttl: Duration,
    pub pin_max_attempts: u32,
    pub pin_lockout: Duration,
}

impl ApiConfig {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            lan_host: None,
            port: 7878,
            data_dir: data_dir.into(),
            extensions_catalog: None,
            themes_dir: None,
            pairing: PairingConfig::default(),
            failure_delay: Duration::from_millis(250),
            token_ttl: Duration::from_secs(30 * 24 * 3600),
            pin_max_attempts: 5,
            pin_lockout: Duration::from_secs(60),
        }
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.data_dir.join("cache")
    }

    pub fn extensions_dir(&self) -> PathBuf {
        self.data_dir.join("extensions")
    }

    pub fn cast_dir(&self) -> PathBuf {
        self.data_dir.join("cast")
    }
}

/// Ouvre une connexion à la base (une pour l'API, une par tâche de fond).
pub type StoreFactory = Arc<dyn Fn() -> DomainResult<Store> + Send + Sync>;
/// Horloge injectable (tests d'expiration).
pub type Clock = Arc<dyn Fn() -> DateTime<Utc> + Send + Sync>;

/// Profil choisi pour la session (commun à la TV et aux télécommandes :
/// c'est le profil affiché sur la télévision).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Selection {
    #[default]
    None,
    Guest,
    Account(Id),
}

#[derive(Debug, Clone, Default)]
pub struct PinAttempts {
    pub failures: u32,
    pub locked_until: Option<DateTime<Utc>>,
}

pub struct Inner {
    pub config: ApiConfig,
    store: Mutex<Option<Store>>,
    open_store: StoreFactory,
    pub pairing: Mutex<PairingManager>,
    pub selection: Mutex<Selection>,
    pub pin_attempts: Mutex<HashMap<Id, PinAttempts>>,
    pub events: broadcast::Sender<Event>,
    pub tv_changed: watch::Sender<u64>,
    pub player: Mutex<PlayerRuntime>,
    pub scan: Mutex<ScanStatus>,
    /// Un scan a été demandé pendant qu'un autre tournait : il sera relancé
    /// (toutes les sources) à la fin du scan en cours.
    pub scan_pending: std::sync::atomic::AtomicBool,
    pub providers: Vec<Arc<dyn MetadataProvider>>,
    pub images: Arc<FsImageCache>,
    pub clock: Clock,
}

#[derive(Clone)]
pub struct AppState(pub Arc<Inner>);

impl std::ops::Deref for AppState {
    type Target = Inner;

    fn deref(&self) -> &Inner {
        &self.0
    }
}

/// Verrou qui survit à un fil d'exécution qui aurait paniqué.
pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl AppState {
    /// L'état démarre « pas prêt » : appeler [`AppState::open_db`].
    pub fn new(
        config: ApiConfig,
        open_store: StoreFactory,
        providers: Vec<Arc<dyn MetadataProvider>>,
    ) -> Self {
        let (events, _) = broadcast::channel(256);
        let (tv_changed, _) = watch::channel(0);
        let images = Arc::new(FsImageCache::new(config.cache_dir()));
        Self(Arc::new(Inner {
            pairing: Mutex::new(PairingManager::new(config.pairing)),
            config,
            store: Mutex::new(None),
            open_store,
            selection: Mutex::new(Selection::None),
            pin_attempts: Mutex::new(HashMap::new()),
            events,
            tv_changed,
            player: Mutex::new(PlayerRuntime::default()),
            scan: Mutex::new(ScanStatus::default()),
            scan_pending: std::sync::atomic::AtomicBool::new(false),
            providers,
            images,
            clock: Arc::new(Utc::now),
        }))
    }

    /// Comme [`AppState::new`], avec une horloge choisie (tests).
    pub fn with_clock(self, clock: Clock) -> Self {
        let inner =
            Arc::try_unwrap(self.0).unwrap_or_else(|_| panic!("with_clock avant tout partage"));
        Self(Arc::new(Inner { clock, ..inner }))
    }

    /// Ouvre la base. Au démarrage, aucun profil n'est actif : un compte non
    /// restreint redemandera son PIN.
    pub fn open_db(&self) -> DomainResult<()> {
        let store = (self.open_store)()?;
        store.set_active_user(None)?;
        let app = store.app_state()?;
        {
            let mut p = lock(&self.player);
            p.state.volume = app.volume;
            p.state.shuffle = app.shuffle;
            p.state.repeat = app.repeat_mode;
        }
        *lock(&self.store) = Some(store);
        Ok(())
    }

    pub fn is_ready(&self) -> bool {
        lock(&self.store).is_some()
    }

    /// Nouvelle connexion à la base (tâches de fond : scan, enrichissement).
    pub fn open_background_store(&self) -> DomainResult<Store> {
        (self.open_store)()
    }

    pub fn now(&self) -> DateTime<Utc> {
        (self.clock)()
    }

    pub fn now_iso(&self) -> String {
        iso(self.now())
    }

    /// Exécute `f` avec la base, hors du fil async.
    pub async fn db<T, F>(&self, f: F) -> ApiResult<T>
    where
        T: Send + 'static,
        F: FnOnce(&Store) -> ApiResult<T> + Send + 'static,
    {
        let state = self.clone();
        tokio::task::spawn_blocking(move || {
            let guard = lock(&state.store);
            let store = guard.as_ref().ok_or_else(ApiError::starting)?;
            f(store)
        })
        .await
        .map_err(ApiError::internal)?
    }

    pub fn emit(&self, event: Event) {
        let _ = self.events.send(event);
    }

    /// Signale à l'app TV que son état a changé (flux SSE).
    pub fn notify_tv(&self) {
        self.tv_changed.send_modify(|v| *v = v.wrapping_add(1));
    }

    pub fn selection(&self) -> Selection {
        *lock(&self.selection)
    }

    pub fn set_selection(&self, s: Selection) {
        *lock(&self.selection) = s;
        self.notify_tv();
    }

    /// Profil actif, relu en base (ses changements s'appliquent aussitôt).
    pub async fn active_profile(&self) -> ApiResult<Option<ActiveProfile>> {
        match self.selection() {
            Selection::None => Ok(None),
            Selection::Guest => Ok(Some(ActiveProfile::Guest)),
            Selection::Account(id) => {
                let profile: Option<Profile> = self.db(move |s| Ok(s.get_profile(id)?)).await?;
                match profile {
                    Some(p) => Ok(Some(ActiveProfile::Account(p))),
                    None => {
                        self.set_selection(Selection::None);
                        Ok(None)
                    }
                }
            }
        }
    }

    /// Pause croissante après un échec (code d'appairage, PIN).
    pub async fn failure_pause(&self, failures: u32) {
        let d = self.config.failure_delay * failures.min(8);
        if !d.is_zero() {
            tokio::time::sleep(d).await;
        }
    }
}

pub fn iso(t: DateTime<Utc>) -> String {
    t.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

impl ApiError {
    /// Base pas encore prête.
    pub fn starting() -> Self {
        Self::new(
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "starting",
            "Le serveur démarre, réessayez dans un instant.",
        )
    }
}
