//! Appairage : sessions TV et code à 6 chiffres, gardés EN MÉMOIRE (rien en
//! base : seul le résultat, l'appareil appairé, y est écrit).
//!
//! - Code aléatoire sûr (OsRng), à usage unique, valable quelques minutes.
//! - Tentatives limitées par code ; au-delà, code invalidé, nouveau code, et
//!   blocage temporaire. 6 chiffres = 1 000 000 de combinaisons : c'est la
//!   limitation des tentatives qui protège.

use std::collections::HashMap;
use std::time::Duration;

use chrono::{DateTime, Utc};
use rand::Rng;

use epikodi_domain::protocol::DeviceSummary;

#[derive(Debug, Clone, PartialEq)]
pub struct PairingCode {
    pub code: String,
    pub expires_at: DateTime<Utc>,
    pub attempts_left: u32,
    pub session_id: String,
}

#[derive(Debug, Clone)]
pub struct TvSession {
    pub id: String,
    pub model: String,
    pub platform: String,
    pub app_version: String,
    /// Téléphone appairé (ou reconnecté) pendant cette session.
    pub device: Option<DeviceSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairFailure {
    Invalid { attempts_left: u32 },
    Expired,
    TooManyAttempts { retry_after_s: u64 },
}

#[derive(Debug, Clone, Copy)]
pub struct PairingConfig {
    pub code_ttl: Duration,
    pub max_attempts: u32,
    pub lockout: Duration,
}

impl Default for PairingConfig {
    fn default() -> Self {
        Self {
            code_ttl: Duration::from_secs(300),
            max_attempts: 5,
            lockout: Duration::from_secs(30),
        }
    }
}

pub struct PairingManager {
    cfg: PairingConfig,
    sessions: HashMap<String, TvSession>,
    current: Option<PairingCode>,
    locked_until: Option<DateTime<Utc>>,
}

fn random_code() -> String {
    format!("{:06}", rand::rngs::OsRng.gen_range(0..1_000_000u32))
}

fn random_session_id() -> String {
    let mut rng = rand::rngs::OsRng;
    (0..16)
        .map(|_| format!("{:x}", rng.gen_range(0..16u8)))
        .collect()
}

/// Comparaison en temps constant (ne révèle pas le nombre de chiffres justes).
fn same_code(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (x, y)| acc | (x ^ y))
            == 0
}

impl PairingManager {
    pub fn new(cfg: PairingConfig) -> Self {
        Self {
            cfg,
            sessions: HashMap::new(),
            current: None,
            locked_until: None,
        }
    }

    fn new_code(&mut self, now: DateTime<Utc>, session_id: &str) -> PairingCode {
        let code = PairingCode {
            code: random_code(),
            expires_at: now + chrono::Duration::from_std(self.cfg.code_ttl).unwrap_or_default(),
            attempts_left: self.cfg.max_attempts,
            session_id: session_id.to_string(),
        };
        self.current = Some(code.clone());
        code
    }

    /// Nouvelle session TV (au démarrage de l'app TV) et son premier code.
    pub fn connect(
        &mut self,
        now: DateTime<Utc>,
        model: &str,
        platform: &str,
        app_version: &str,
    ) -> (TvSession, PairingCode) {
        let id = random_session_id();
        // Un appareil déjà appairé dans une session précédente reste connu.
        let device = self.sessions.values().find_map(|s| s.device.clone());
        let session = TvSession {
            id: id.clone(),
            model: model.into(),
            platform: platform.into(),
            app_version: app_version.into(),
            device,
        };
        self.sessions.insert(id.clone(), session.clone());
        let code = self.new_code(now, &id);
        (session, code)
    }

    pub fn session(&self, id: &str) -> Option<&TvSession> {
        self.sessions.get(id)
    }

    /// Nouveau code pour une session existante (`None` : session inconnue).
    pub fn refresh(&mut self, now: DateTime<Utc>, session_id: &str) -> Option<PairingCode> {
        self.sessions
            .contains_key(session_id)
            .then(|| self.new_code(now, session_id))
    }

    /// Code valide affiché pour une session (renouvelé s'il a expiré).
    pub fn code_for(&mut self, now: DateTime<Utc>, session_id: &str) -> Option<PairingCode> {
        if !self.sessions.contains_key(session_id) {
            return None;
        }
        match &self.current {
            Some(c) if c.session_id == session_id && c.expires_at > now => Some(c.clone()),
            _ => Some(self.new_code(now, session_id)),
        }
    }

    /// Vérifie un code saisi sur la télécommande. En cas de succès, le code
    /// est consommé (un nouveau est généré) et l'identifiant de la session
    /// TV concernée est renvoyé.
    pub fn verify(&mut self, now: DateTime<Utc>, code: &str) -> Result<String, PairFailure> {
        if let Some(until) = self.locked_until {
            if now < until {
                return Err(PairFailure::TooManyAttempts {
                    retry_after_s: (until - now).num_seconds().max(1) as u64,
                });
            }
            self.locked_until = None;
        }
        let Some(current) = self.current.clone() else {
            return Err(PairFailure::Expired);
        };
        if now >= current.expires_at {
            self.new_code(now, &current.session_id);
            return Err(PairFailure::Expired);
        }
        if !same_code(code.trim(), &current.code) {
            let left = current.attempts_left.saturating_sub(1);
            if left == 0 {
                self.new_code(now, &current.session_id);
                self.locked_until =
                    Some(now + chrono::Duration::from_std(self.cfg.lockout).unwrap_or_default());
                return Err(PairFailure::TooManyAttempts {
                    retry_after_s: self.cfg.lockout.as_secs().max(1),
                });
            }
            if let Some(c) = self.current.as_mut() {
                c.attempts_left = left;
            }
            return Err(PairFailure::Invalid {
                attempts_left: left,
            });
        }
        self.new_code(now, &current.session_id);
        Ok(current.session_id)
    }

    /// Un appareil vient de s'appairer (ou de se reconnecter) : les sessions
    /// TV en attente passent à l'étape suivante.
    pub fn attach_device(&mut self, device: DeviceSummary) -> bool {
        let mut changed = false;
        for s in self.sessions.values_mut() {
            if s.device.is_none() {
                s.device = Some(device.clone());
                changed = true;
            }
        }
        changed
    }

    /// Un appareil révoqué n'est plus affiché par les sessions TV.
    pub fn detach_device(&mut self, device_id: i64) {
        for s in self.sessions.values_mut() {
            if s.device.as_ref().is_some_and(|d| d.id == device_id) {
                s.device = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mgr() -> PairingManager {
        PairingManager::new(PairingConfig::default())
    }

    fn t0() -> DateTime<Utc> {
        "2026-11-07T18:30:00Z".parse().unwrap()
    }

    #[test]
    fn code_six_chiffres_et_usage_unique() {
        let mut m = mgr();
        let (s, c) = m.connect(t0(), "Desktop", "linux", "0.1.0");
        assert_eq!(c.code.len(), 6);
        assert!(c.code.chars().all(|ch| ch.is_ascii_digit()));
        assert_eq!(c.attempts_left, 5);
        assert_eq!(m.verify(t0(), &c.code).unwrap(), s.id);
        assert!(m.verify(t0(), &c.code).is_err(), "usage unique");
    }

    #[test]
    fn tentatives_limitees_puis_blocage() {
        let mut m = mgr();
        let (_, c) = m.connect(t0(), "Desktop", "linux", "0.1.0");
        let wrong = if c.code == "000000" {
            "111111"
        } else {
            "000000"
        };
        for left in (1..5).rev() {
            assert_eq!(
                m.verify(t0(), wrong),
                Err(PairFailure::Invalid {
                    attempts_left: left
                })
            );
        }
        assert!(matches!(
            m.verify(t0(), wrong),
            Err(PairFailure::TooManyAttempts { .. })
        ));
        // l'ancien code est invalidé, et on est bloqué même avec le bon
        assert!(matches!(
            m.verify(t0(), &c.code),
            Err(PairFailure::TooManyAttempts { .. })
        ));
        let later = t0() + chrono::Duration::seconds(31);
        assert!(m.verify(later, &c.code).is_err(), "ancien code invalidé");
    }

    #[test]
    fn code_expire() {
        let mut m = mgr();
        let (s, c) = m.connect(t0(), "Desktop", "linux", "0.1.0");
        let later = t0() + chrono::Duration::seconds(301);
        assert_eq!(m.verify(later, &c.code), Err(PairFailure::Expired));
        let fresh = m.code_for(later, &s.id).unwrap();
        assert_ne!(fresh.code, c.code);
        assert!(fresh.expires_at > later);
    }

    #[test]
    fn rafraichir_une_session_inconnue() {
        let mut m = mgr();
        assert!(m.refresh(t0(), "inconnue").is_none());
        let (s, c) = m.connect(t0(), "Desktop", "linux", "0.1.0");
        let r = m.refresh(t0(), &s.id).unwrap();
        assert_eq!(r.session_id, s.id);
        assert!(
            m.verify(t0(), &c.code).is_err() || r.code == c.code,
            "l'ancien code ne vaut plus"
        );
    }

    #[test]
    fn appareil_attache_aux_sessions() {
        let mut m = mgr();
        let (s, _) = m.connect(t0(), "Desktop", "linux", "0.1.0");
        assert!(m.attach_device(DeviceSummary {
            id: 1,
            name: "Pixel".into()
        }));
        assert_eq!(m.session(&s.id).unwrap().device.as_ref().unwrap().id, 1);
        let (s2, _) = m.connect(t0(), "Desktop", "linux", "0.1.0");
        assert!(
            m.session(&s2.id).unwrap().device.is_some(),
            "connu après redémarrage de la TV"
        );
        m.detach_device(1);
        assert!(m.session(&s.id).unwrap().device.is_none());
    }
}
