//! Authentification des appels.
//!
//! - Télécommande : `Authorization: Bearer <jeton>` (ou `?token=` pour le
//!   WebSocket). Seul le hash du jeton est en base ; jeton expiré ou révoqué
//!   = 401, la télécommande retourne à l'écran d'appairage.
//! - App TV : elle tourne sur la même machine que le cœur. Un appel venant
//!   de la boucle locale SANS en-tête `Authorization` est traité comme la TV.
//!   Un en-tête présent est toujours vérifié, même en local.
//! - Routes propres à la TV (`/api/tv/*`, publication de l'état du lecteur) :
//!   accès local uniquement ([`LocalOnly`]).

pub mod pairing;

use std::net::{IpAddr, SocketAddr};

use axum::async_trait;
use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use rand::distributions::Alphanumeric;
use rand::Rng;
use serde_json::json;

use epikodi_domain::device::Device;
use epikodi_domain::parental::{check_pin_format, Visibility};
use epikodi_domain::ports::DeviceRepo;
use epikodi_domain::profile::{ActiveProfile, Profile};
use epikodi_domain::protocol::DeviceSummary;
use epikodi_storage::secrets::{hash_token, verify_pin};

use crate::error::{ApiError, ApiResult};
use crate::state::{lock, AppState};

pub const TOKEN_PREFIX: &str = "mc_";

/// Nouveau jeton de session, opaque et aléatoire.
pub fn new_token() -> String {
    let body: String = rand::rngs::OsRng
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    format!("{TOKEN_PREFIX}{body}")
}

fn is_loopback(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_loopback(),
        IpAddr::V6(v6) => {
            v6.is_loopback() || v6.to_ipv4_mapped().is_some_and(|v4| v4.is_loopback())
        }
    }
}

/// L'appel vient-il de la machine elle-même ? (Sans information de
/// connexion, on répond non : c'est le choix sûr.)
pub fn is_local(parts: &Parts) -> bool {
    parts
        .extensions
        .get::<ConnectInfo<SocketAddr>>()
        .is_some_and(|c| is_loopback(c.0.ip()))
}

#[derive(Debug, Clone)]
pub enum Caller {
    /// L'app TV, en local.
    Tv,
    /// Un téléphone appairé.
    Device(Device),
}

/// Appel authentifié (TV locale ou appareil appairé).
#[derive(Debug, Clone)]
pub struct Auth {
    pub caller: Caller,
    pub local: bool,
}

impl Auth {
    pub fn device(&self) -> Option<&Device> {
        match &self.caller {
            Caller::Device(d) => Some(d),
            Caller::Tv => None,
        }
    }

    pub fn is_tv(&self) -> bool {
        matches!(self.caller, Caller::Tv)
    }
}

fn bearer(parts: &Parts) -> Option<String> {
    if let Some(h) = parts.headers.get(AUTHORIZATION) {
        let v = h.to_str().ok()?.trim();
        return Some(
            v.strip_prefix("Bearer ")
                .or_else(|| v.strip_prefix("bearer "))
                .unwrap_or("")
                .trim()
                .to_string(),
        );
    }
    // WebSocket : les navigateurs ne peuvent pas poser d'en-tête.
    if parts.uri.path() == "/api/ws" {
        return parts.uri.query().and_then(|q| {
            q.split('&')
                .find_map(|kv| kv.strip_prefix("token="))
                .map(String::from)
        });
    }
    None
}

#[async_trait]
impl FromRequestParts<AppState> for Auth {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let local = is_local(parts);
        let Some(token) = bearer(parts) else {
            return if local {
                Ok(Auth {
                    caller: Caller::Tv,
                    local,
                })
            } else {
                Err(ApiError::unauthorized())
            };
        };
        if token.is_empty() {
            return Err(ApiError::unauthorized());
        }
        let hash = hash_token(&token);
        let now = state.now();
        let now_iso = state.now_iso();
        let device = state
            .db(move |s| {
                // Ni révoqué ni expiré à l'heure du serveur (vérifié en base).
                let Some(d) = s.find_device_by_token(&hash, &now_iso)? else {
                    return Ok(None);
                };
                // Dernière activité, au plus une écriture par minute.
                let stale = d
                    .last_seen_at
                    .as_deref()
                    .and_then(|t| t.parse::<chrono::DateTime<chrono::Utc>>().ok())
                    .is_none_or(|t| (now - t).num_seconds() > 60);
                if stale {
                    s.touch_device(d.id, &now_iso)?;
                }
                Ok(Some(d))
            })
            .await?
            .ok_or_else(ApiError::unauthorized)?;

        // Un téléphone déjà appairé qui se reconnecte fait avancer la TV.
        let attached = lock(&state.pairing).attach_device(DeviceSummary {
            id: device.id,
            name: device.name.clone(),
        });
        if attached {
            state.notify_tv();
        }
        Ok(Auth {
            caller: Caller::Device(device),
            local,
        })
    }
}

/// Route réservée à l'app TV (boucle locale).
pub struct LocalOnly;

#[async_trait]
impl FromRequestParts<AppState> for LocalOnly {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _: &AppState) -> Result<Self, Self::Rejection> {
        if is_local(parts) {
            Ok(LocalOnly)
        } else {
            Err(ApiError::localhost_only())
        }
    }
}

/// Appel authentifié + profil actif (s'il y en a un).
#[derive(Debug, Clone)]
pub struct Ctx {
    pub auth: Auth,
    pub profile: Option<ActiveProfile>,
}

#[async_trait]
impl FromRequestParts<AppState> for Ctx {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth = Auth::from_request_parts(parts, state).await?;
        let profile = state.active_profile().await?;
        Ok(Ctx { auth, profile })
    }
}

impl Ctx {
    /// Un profil (invité ou compte) doit être choisi.
    pub fn profile(&self) -> ApiResult<&ActiveProfile> {
        self.profile.as_ref().ok_or_else(ApiError::profile_required)
    }

    pub fn visibility(&self) -> ApiResult<Visibility> {
        Ok(self.profile()?.visibility())
    }

    /// Un compte (pas l'invité).
    pub fn account(&self) -> ApiResult<&Profile> {
        self.profile()?
            .account()
            .ok_or_else(ApiError::guest_forbidden)
    }

    /// Un compte non restreint.
    pub fn admin(&self) -> ApiResult<&Profile> {
        let p = self.account()?;
        if p.is_restricted {
            Err(ApiError::admin_required())
        } else {
            Ok(p)
        }
    }
}

/// Vérifie le PIN d'un compte, avec tentatives limitées et verrouillage
/// temporaire (4 chiffres = 10 000 combinaisons).
pub async fn check_pin(state: &AppState, profile: &Profile, pin: Option<&str>) -> ApiResult<()> {
    let Some(hash) = profile.pin_hash.clone() else {
        return Ok(()); // compte sans PIN
    };
    let pin = pin.ok_or_else(|| {
        ApiError::new(
            axum::http::StatusCode::FORBIDDEN,
            "pin_required",
            "Ce compte est protégé par un code PIN.",
        )
    })?;
    let now = state.now();
    {
        let attempts = lock(&state.pin_attempts);
        if let Some(until) = attempts.get(&profile.id).and_then(|a| a.locked_until) {
            if now < until {
                let retry = (until - now).num_seconds().max(1);
                return Err(pin_locked(retry));
            }
        }
    }
    let ok = check_pin_format(pin).is_ok() && {
        let pin = pin.to_string();
        tokio::task::spawn_blocking(move || verify_pin(&pin, &hash))
            .await
            .unwrap_or(false)
    };
    if ok {
        lock(&state.pin_attempts).remove(&profile.id);
        return Ok(());
    }
    let max = state.config.pin_max_attempts;
    let (failures, locked) = {
        let mut attempts = lock(&state.pin_attempts);
        let a = attempts.entry(profile.id).or_default();
        if a.locked_until.is_some_and(|u| now >= u) {
            *a = Default::default();
        }
        a.failures += 1;
        if a.failures >= max {
            a.locked_until = Some(
                now + chrono::Duration::from_std(state.config.pin_lockout).unwrap_or_default(),
            );
            a.failures = 0;
            (max, true)
        } else {
            (a.failures, false)
        }
    };
    state.failure_pause(failures).await;
    if locked {
        return Err(pin_locked(state.config.pin_lockout.as_secs().max(1) as i64));
    }
    Err(ApiError::new(
        axum::http::StatusCode::FORBIDDEN,
        "pin_invalid",
        "Code PIN incorrect.",
    )
    .with_details(json!({ "attempts_left": max - failures })))
}

fn pin_locked(retry_after_s: i64) -> ApiError {
    ApiError::new(
        axum::http::StatusCode::TOO_MANY_REQUESTS,
        "pin_locked",
        "Trop d'essais : réessayez plus tard.",
    )
    .with_details(json!({ "retry_after_s": retry_after_s }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jeton_opaque() {
        let t = new_token();
        assert!(t.starts_with(TOKEN_PREFIX));
        assert_eq!(t.len(), TOKEN_PREFIX.len() + 32);
        assert_ne!(t, new_token());
    }

    #[test]
    fn boucle_locale() {
        assert!(is_loopback("127.0.0.1".parse().unwrap()));
        assert!(is_loopback("::1".parse().unwrap()));
        assert!(is_loopback("::ffff:127.0.0.1".parse().unwrap()));
        assert!(!is_loopback("192.168.1.20".parse().unwrap()));
    }
}
