//! Supervision : chaque extension tourne dans son propre fil d'exécution.
//! Un plantage (panique, erreur), un blocage (délai dépassé) ou un abus de
//! ressources arrête l'extension sans toucher au reste du media center, et
//! l'utilisateur est prévenu (statut `crashed` + événement).

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc;
use std::thread;

use serde_json::Value;

use epikodi_domain::extension::{ExtensionStatus, Grants};

use crate::permissions::{check, HostCall};
use crate::runtime::{ExtensionInstance, HostBridge, Limits, RuntimeError};

/// Échecs consécutifs avant l'arrêt définitif (jusqu'à réactivation).
pub const MAX_FAILURES: u32 = 3;

type Job = (String, Value, mpsc::Sender<Result<Value, RuntimeError>>);

/// Pont vers l'hôte qui vérifie les permissions avant chaque appel.
pub struct GuardedHost<H: HostBridge> {
    grants: Grants,
    inner: H,
}

impl<H: HostBridge> GuardedHost<H> {
    pub fn new(grants: Grants, inner: H) -> Self {
        Self { grants, inner }
    }
}

impl<H: HostBridge> HostBridge for GuardedHost<H> {
    fn call(&mut self, call: HostCall) -> Result<Value, RuntimeError> {
        check(&self.grants, &call).map_err(|e| RuntimeError::PermissionDenied(e.to_string()))?;
        if let HostCall::HttpGet { url } = &call {
            crate::permissions::check_outbound_url(url)
                .map_err(|e| RuntimeError::PermissionDenied(e.to_string()))?;
        }
        self.inner.call(call)
    }
}

/// Changement d'état signalé à l'application (pour prévenir l'utilisateur).
#[derive(Debug, Clone, PartialEq)]
pub struct StatusChange {
    pub extension_id: String,
    pub status: ExtensionStatus,
    pub message: Option<String>,
}

pub struct Supervisor {
    extension_id: String,
    limits: Limits,
    jobs: Option<mpsc::Sender<Job>>,
    failures: u32,
    status: ExtensionStatus,
    on_status: Box<dyn Fn(StatusChange) + Send + Sync>,
}

impl Supervisor {
    /// Démarre l'extension dans son fil d'exécution.
    pub fn start<H: HostBridge + 'static>(
        extension_id: impl Into<String>,
        mut instance: Box<dyn ExtensionInstance>,
        host: H,
        limits: Limits,
        on_status: impl Fn(StatusChange) + Send + Sync + 'static,
    ) -> Self {
        let (tx, rx) = mpsc::channel::<Job>();
        let id: String = extension_id.into();
        let thread_name = format!("extension-{id}");
        let mut host = host;
        let spawned = thread::Builder::new().name(thread_name).spawn(move || {
            for (function, input, reply) in rx {
                let result = catch_unwind(AssertUnwindSafe(|| {
                    instance.call(&function, &input, &mut host)
                }))
                .unwrap_or_else(|panic| {
                    let msg = panic
                        .downcast_ref::<&str>()
                        .map(|s| s.to_string())
                        .or_else(|| panic.downcast_ref::<String>().cloned())
                        .unwrap_or_else(|| "panique".into());
                    Err(RuntimeError::Trap(msg))
                });
                let fatal = matches!(&result, Err(RuntimeError::Trap(_)));
                let _ = reply.send(result);
                if fatal {
                    break; // l'instance n'est plus fiable : on la jette
                }
            }
        });
        Self {
            extension_id: id,
            limits,
            jobs: spawned.ok().map(|_| tx),
            failures: 0,
            status: ExtensionStatus::Ok,
            on_status: Box::new(on_status),
        }
    }

    pub fn status(&self) -> ExtensionStatus {
        self.status
    }

    /// Appelle une fonction de l'extension, avec délai maximal.
    pub fn call(&mut self, function: &str, input: Value) -> Result<Value, RuntimeError> {
        if self.status != ExtensionStatus::Ok {
            return Err(RuntimeError::Load("extension arrêtée".into()));
        }
        let Some(jobs) = &self.jobs else {
            return self.fail(RuntimeError::Load("fil d'exécution indisponible".into()));
        };
        let (reply_tx, reply_rx) = mpsc::channel();
        if jobs.send((function.to_string(), input, reply_tx)).is_err() {
            return self.fail(RuntimeError::Trap("l'extension ne répond plus".into()));
        }
        match reply_rx.recv_timeout(self.limits.call_timeout) {
            Ok(Ok(v)) => {
                self.failures = 0;
                Ok(v)
            }
            Ok(Err(e @ RuntimeError::MissingFunction(_)))
            | Ok(Err(e @ RuntimeError::PermissionDenied(_))) => Err(e),
            Ok(Err(e @ (RuntimeError::Trap(_) | RuntimeError::OutOfMemory))) => {
                // instance jetée : arrêt immédiat
                self.jobs = None;
                self.stop(ExtensionStatus::Crashed, e.to_string());
                Err(e)
            }
            Ok(Err(e)) => self.fail(e),
            Err(_) => {
                // Bloquée : on abandonne son fil (le moteur WASM l'interrompt
                // via son carburant) et on l'arrête.
                self.jobs = None;
                self.stop(ExtensionStatus::Crashed, "bloquée (délai dépassé)".into());
                Err(RuntimeError::Timeout)
            }
        }
    }

    /// Arrêt demandé (désactivation, désinstallation).
    pub fn shutdown(&mut self) {
        self.jobs = None;
        if self.status == ExtensionStatus::Ok {
            self.status = ExtensionStatus::Stopped;
        }
    }

    fn fail(&mut self, e: RuntimeError) -> Result<Value, RuntimeError> {
        self.failures += 1;
        if self.failures >= MAX_FAILURES {
            self.jobs = None;
            self.stop(
                ExtensionStatus::Crashed,
                format!("{MAX_FAILURES} échecs consécutifs : {e}"),
            );
        }
        Err(e)
    }

    fn stop(&mut self, status: ExtensionStatus, message: String) {
        self.status = status;
        tracing::warn!(extension = %self.extension_id, "extension arrêtée : {message}");
        (self.on_status)(StatusChange {
            extension_id: self.extension_id.clone(),
            status,
            message: Some(message),
        });
    }
}

impl Drop for Supervisor {
    fn drop(&mut self) {
        self.jobs = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use epikodi_domain::extension::Permission;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    /// Fausse extension pilotée par le nom de la fonction appelée.
    struct Fake;
    impl ExtensionInstance for Fake {
        fn call(
            &mut self,
            function: &str,
            input: &Value,
            host: &mut dyn HostBridge,
        ) -> Result<Value, RuntimeError> {
            match function {
                "echo" => Ok(input.clone()),
                "panic" => panic!("bug dans l'extension"),
                "loop" => {
                    std::thread::sleep(Duration::from_secs(2));
                    Ok(Value::Null)
                }
                "error" => Err(RuntimeError::Load("erreur passagère".into())),
                "read_library" => host.call(HostCall::LibrarySearch { query: "x".into() }),
                "fetch_local" => host.call(HostCall::HttpGet {
                    url: "http://192.168.1.1".into(),
                }),
                f => Err(RuntimeError::MissingFunction(f.into())),
            }
        }
    }

    struct Host;
    impl HostBridge for Host {
        fn call(&mut self, _: HostCall) -> Result<Value, RuntimeError> {
            Ok(Value::Bool(true))
        }
    }

    fn start(grants: &[Permission]) -> (Supervisor, Arc<Mutex<Vec<StatusChange>>>) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let ev = events.clone();
        let host = GuardedHost::new(Grants::from_acceptance(grants, grants).unwrap(), Host);
        let s = Supervisor::start(
            "weather",
            Box::new(Fake),
            host,
            Limits {
                call_timeout: Duration::from_millis(200),
                ..Default::default()
            },
            move |c| ev.lock().unwrap().push(c),
        );
        (s, events)
    }

    #[test]
    fn appel_normal() {
        let (mut s, _) = start(&[]);
        assert_eq!(s.call("echo", Value::from(42)).unwrap(), Value::from(42));
        assert_eq!(s.status(), ExtensionStatus::Ok);
    }

    #[test]
    fn plantage_isole_et_signale() {
        let (mut s, events) = start(&[]);
        assert!(matches!(
            s.call("panic", Value::Null),
            Err(RuntimeError::Trap(_))
        ));
        assert_eq!(s.status(), ExtensionStatus::Crashed);
        assert_eq!(events.lock().unwrap()[0].status, ExtensionStatus::Crashed);
        assert!(s.call("echo", Value::Null).is_err(), "arrêtée");
    }

    #[test]
    fn blocage_coupe_au_delai() {
        let (mut s, events) = start(&[]);
        assert_eq!(s.call("loop", Value::Null), Err(RuntimeError::Timeout));
        assert_eq!(s.status(), ExtensionStatus::Crashed);
        assert!(events.lock().unwrap()[0]
            .message
            .as_deref()
            .unwrap()
            .contains("délai"));
    }

    #[test]
    fn echecs_repetes() {
        let (mut s, _) = start(&[]);
        for _ in 0..MAX_FAILURES - 1 {
            assert!(s.call("error", Value::Null).is_err());
            assert_eq!(s.status(), ExtensionStatus::Ok);
        }
        assert!(s.call("error", Value::Null).is_err());
        assert_eq!(s.status(), ExtensionStatus::Crashed);
    }

    #[test]
    fn permissions_verifiees_par_l_hote() {
        let (mut s, _) = start(&[Permission::Network]);
        assert!(matches!(
            s.call("read_library", Value::Null),
            Err(RuntimeError::PermissionDenied(_))
        ));
        assert!(matches!(
            s.call("fetch_local", Value::Null),
            Err(RuntimeError::PermissionDenied(_))
        ));
        assert_eq!(
            s.status(),
            ExtensionStatus::Ok,
            "un refus n'arrête pas l'extension"
        );
        let (mut s, _) = start(&[Permission::LibraryRead]);
        assert_eq!(
            s.call("read_library", Value::Null).unwrap(),
            Value::Bool(true)
        );
    }
}
