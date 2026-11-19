//! Annonce du media center sur le réseau local (mDNS / Zeroconf), pour que
//! la télécommande le trouve sans saisir d'adresse.
//!
//! Service `_mediacenter._tcp.local.`, TXT : `version`, `api=/api`.

use std::collections::HashMap;

use mdns_sd::{ServiceDaemon, ServiceInfo};

pub const SERVICE_TYPE: &str = "_mediacenter._tcp.local.";

/// Garde le service annoncé tant qu'elle vit.
pub struct Announcement {
    daemon: ServiceDaemon,
    fullname: String,
}

impl Drop for Announcement {
    fn drop(&mut self) {
        let _ = self.daemon.unregister(&self.fullname);
        let _ = self.daemon.shutdown();
    }
}

pub fn announce(name: &str, host_ip: &str, port: u16) -> Result<Announcement, String> {
    let daemon = ServiceDaemon::new().map_err(|e| e.to_string())?;
    let hostname = format!("{}.local.", sanitize(name));
    let props: HashMap<String, String> = [
        ("version".to_string(), env!("CARGO_PKG_VERSION").to_string()),
        ("api".to_string(), "/api".to_string()),
    ]
    .into_iter()
    .collect();
    let info = ServiceInfo::new(SERVICE_TYPE, name, &hostname, host_ip, port, props)
        .map_err(|e| e.to_string())?;
    let fullname = info.get_fullname().to_string();
    daemon.register(info).map_err(|e| e.to_string())?;
    Ok(Announcement { daemon, fullname })
}

fn sanitize(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let s = s.trim_matches('-').to_string();
    if s.is_empty() {
        "media-center".into()
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nom_d_hote_valide() {
        assert_eq!(sanitize("Media center du salon"), "media-center-du-salon");
        assert_eq!(sanitize("!!!"), "media-center");
    }
}
