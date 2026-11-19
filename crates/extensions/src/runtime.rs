//! Contrat d'exécution d'une extension.
//!
//! Le code d'une extension (module WASM) tourne isolé : il n'accède à rien
//! d'autre qu'aux appels [`HostCall`](crate::permissions::HostCall), filtrés
//! par ses permissions, et il est soumis à des [`Limits`]. Le moteur concret
//! (wasmtime, avec carburant et limite mémoire) implémente
//! [`ExtensionRuntime`] ; le [`Supervisor`](crate::supervisor::Supervisor)
//! se charge des délais et des plantages, quel que soit le moteur.

use std::path::Path;
use std::time::Duration;

use serde_json::Value;

use epikodi_domain::extension::Manifest;

use crate::permissions::HostCall;

/// Limites de ressources d'une extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Durée maximale d'un appel.
    pub call_timeout: Duration,
    /// Mémoire maximale du module.
    pub memory_bytes: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            call_timeout: Duration::from_secs(5),
            memory_bytes: 64 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RuntimeError {
    #[error("l'extension a planté : {0}")]
    Trap(String),
    #[error("délai dépassé")]
    Timeout,
    #[error("mémoire dépassée")]
    OutOfMemory,
    #[error("fonction « {0} » absente")]
    MissingFunction(String),
    #[error("chargement impossible : {0}")]
    Load(String),
    #[error("permission refusée : {0}")]
    PermissionDenied(String),
}

/// Ce que l'hôte met à disposition d'une extension pendant un appel.
/// Chaque appel est vérifié contre les permissions avant d'être servi.
pub trait HostBridge: Send {
    fn call(&mut self, call: HostCall) -> Result<Value, RuntimeError>;
}

/// Une extension chargée.
pub trait ExtensionInstance: Send {
    /// Appelle une fonction exportée (`on_start`, `home_widget`,
    /// `search`, `on_playback_event`...) avec une entrée JSON.
    fn call(
        &mut self,
        function: &str,
        input: &Value,
        host: &mut dyn HostBridge,
    ) -> Result<Value, RuntimeError>;
}

/// Moteur d'exécution.
pub trait ExtensionRuntime: Send + Sync {
    fn instantiate(
        &self,
        manifest: &Manifest,
        dir: &Path,
        limits: Limits,
    ) -> Result<Box<dyn ExtensionInstance>, RuntimeError>;
}
