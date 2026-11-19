//! Découverte automatique des partages SMB annoncés en mDNS / Zeroconf
//! (`_smb._tcp`) : NAS, box, ordinateurs du réseau local.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use mdns_sd::{ServiceDaemon, ServiceEvent};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredShare {
    /// Nom annoncé (« NAS du salon »).
    pub name: String,
    pub host: String,
    /// `smb://hôte/` : l'utilisateur choisit ensuite le partage.
    pub uri: String,
}

pub const SMB_SERVICE: &str = "_smb._tcp.local.";

/// Écoute le réseau pendant `wait` et renvoie les serveurs trouvés.
/// Une erreur réseau donne une liste vide (la saisie manuelle reste possible).
pub fn discover_smb(wait: Duration) -> Vec<DiscoveredShare> {
    let daemon = match ServiceDaemon::new() {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!("mDNS indisponible : {e}");
            return Vec::new();
        }
    };
    let rx = match daemon.browse(SMB_SERVICE) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("recherche mDNS impossible : {e}");
            let _ = daemon.shutdown();
            return Vec::new();
        }
    };
    let deadline = Instant::now() + wait;
    let mut found: BTreeMap<String, DiscoveredShare> = BTreeMap::new();
    while let Some(left) = deadline.checked_duration_since(Instant::now()) {
        match rx.recv_timeout(left) {
            Ok(ServiceEvent::ServiceResolved(info)) => {
                let host = info.get_hostname().trim_end_matches('.').to_string();
                let name = info
                    .get_fullname()
                    .strip_suffix(&format!(".{SMB_SERVICE}"))
                    .unwrap_or(info.get_fullname())
                    .to_string();
                found.insert(
                    host.clone(),
                    DiscoveredShare {
                        uri: format!("smb://{host}/"),
                        name,
                        host,
                    },
                );
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }
    let _ = daemon.shutdown();
    found.into_values().collect()
}
