//! Profils (comptes) et préférences.

use serde::{Deserialize, Serialize};

use crate::parental::{check_pin_format, Visibility};
use crate::{DomainError, DomainResult, Id};

/// Un compte stocké en base (table `user`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub id: Id,
    pub name: String,
    pub icon: Option<String>,
    pub language: String,
    pub age: u8,
    pub is_restricted: bool,
    /// Hash du PIN (comptes non restreints). Jamais exposé par l'API.
    #[serde(skip)]
    pub pin_hash: Option<String>,
    pub preferences: Preferences,
}

impl Profile {
    /// Un compte non restreint s'ouvre avec son PIN.
    pub fn requires_pin(&self) -> bool {
        !self.is_restricted
    }

    pub fn visibility(&self) -> Visibility {
        if self.is_restricted {
            Visibility::MaxAge(self.age)
        } else {
            Visibility::All
        }
    }
}

/// Données d'un nouveau compte, déjà validées.
#[derive(Debug, Clone, PartialEq)]
pub struct NewProfile {
    pub name: String,
    pub icon: Option<String>,
    pub language: String,
    pub age: u8,
    pub is_restricted: bool,
    pub pin_hash: Option<String>,
}

/// Le profil actif d'une session.
// Valeur de courte durée (le temps d'une requête) : pas besoin de `Box`.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum ActiveProfile {
    /// Invité : restreint, âge 0, n'existe pas en base.
    Guest,
    Account(Profile),
}

impl ActiveProfile {
    pub fn visibility(&self) -> Visibility {
        match self {
            Self::Guest => Visibility::MaxAge(0),
            Self::Account(p) => p.visibility(),
        }
    }

    pub fn is_guest(&self) -> bool {
        matches!(self, Self::Guest)
    }

    pub fn account(&self) -> Option<&Profile> {
        match self {
            Self::Guest => None,
            Self::Account(p) => Some(p),
        }
    }

    pub fn user_id(&self) -> Option<Id> {
        self.account().map(|p| p.id)
    }

    /// Compte non restreint : peut gérer sources, extensions, appareils.
    pub fn is_admin(&self) -> bool {
        matches!(self, Self::Account(p) if !p.is_restricted)
    }
}

pub const NAME_MAX_LEN: usize = 32;
pub const ICON_MAX_LEN: usize = 256;

pub fn normalize_name(name: &str) -> DomainResult<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(DomainError::validation("name", "obligatoire"));
    }
    if name.chars().count() > NAME_MAX_LEN {
        return Err(DomainError::validation(
            "name",
            format!("{NAME_MAX_LEN} caractères maximum"),
        ));
    }
    if name.chars().any(char::is_control) {
        return Err(DomainError::validation("name", "caractères invalides"));
    }
    Ok(name.to_string())
}

pub fn check_icon(icon: Option<&str>) -> DomainResult<()> {
    match icon {
        Some(i) if i.is_empty() || i.len() > ICON_MAX_LEN => {
            Err(DomainError::validation("icon", "invalide"))
        }
        _ => Ok(()),
    }
}

/// Code de langue simple : `fr`, `en`, `pt-BR`...
pub fn check_language(lang: &str) -> DomainResult<()> {
    let ok = (2..=8).contains(&lang.len())
        && lang
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if ok {
        Ok(())
    } else {
        Err(DomainError::validation(
            "language",
            "code de langue invalide",
        ))
    }
}

pub fn check_age(age: i64) -> DomainResult<u8> {
    if (0..=120).contains(&age) {
        Ok(age as u8)
    } else {
        Err(DomainError::validation("age", "entre 0 et 120"))
    }
}

/// Règle du PIN à la création : obligatoire pour un compte non restreint,
/// absent pour un compte restreint.
pub fn check_pin_for_restriction(is_restricted: bool, pin: Option<&str>) -> DomainResult<()> {
    match (is_restricted, pin) {
        (false, None) => Err(DomainError::validation(
            "pin",
            "obligatoire pour un compte non restreint",
        )),
        (false, Some(p)) => check_pin_format(p),
        (true, Some(_)) => Err(DomainError::validation(
            "pin",
            "un compte restreint n'a pas de PIN",
        )),
        (true, None) => Ok(()),
    }
}

/// Préférences d'un profil (`user.preferences`), toujours typées.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub theme: String,
    pub text_scale: f64,
    pub reduce_motion: bool,
    pub high_contrast: bool,
    pub subtitles: SubtitlePreferences,
    pub audio_lang: Option<String>,
    pub subtitle_lang: Option<String>,
    /// Synthèse vocale des éléments sélectionnés.
    pub speech: bool,
    pub autoplay_next: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme: "default".into(),
            text_scale: 1.0,
            reduce_motion: false,
            high_contrast: false,
            subtitles: SubtitlePreferences::default(),
            audio_lang: None,
            subtitle_lang: None,
            speech: false,
            autoplay_next: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SubtitlePreferences {
    pub size: f64,
    pub color: String,
    pub background: String,
    pub delay_ms: i64,
}

impl Default for SubtitlePreferences {
    fn default() -> Self {
        Self {
            size: 1.0,
            color: "#FFFFFF".into(),
            background: "#000000".into(),
            delay_ms: 0,
        }
    }
}

impl Preferences {
    pub fn validate(&self) -> DomainResult<()> {
        if !(0.5..=3.0).contains(&self.text_scale) {
            return Err(DomainError::validation("text_scale", "entre 0.5 et 3.0"));
        }
        if !(0.5..=3.0).contains(&self.subtitles.size) {
            return Err(DomainError::validation(
                "subtitles.size",
                "entre 0.5 et 3.0",
            ));
        }
        if self.subtitles.delay_ms.abs() > 60_000 {
            return Err(DomainError::validation(
                "subtitles.delay_ms",
                "entre -60000 et 60000",
            ));
        }
        for (field, color) in [
            ("subtitles.color", &self.subtitles.color),
            ("subtitles.background", &self.subtitles.background),
        ] {
            if !is_hex_color(color) {
                return Err(DomainError::validation(
                    field,
                    "couleur #RRGGBB ou #RRGGBBAA",
                ));
            }
        }
        if self.theme.is_empty() || self.theme.len() > 64 {
            return Err(DomainError::validation("theme", "invalide"));
        }
        Ok(())
    }
}

fn is_hex_color(s: &str) -> bool {
    s.strip_prefix('#').is_some_and(|hex| {
        (hex.len() == 6 || hex.len() == 8) && hex.chars().all(|c| c.is_ascii_hexdigit())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nom() {
        assert_eq!(normalize_name("  Léa ").unwrap(), "Léa");
        assert!(normalize_name("   ").is_err());
        assert!(normalize_name(&"x".repeat(33)).is_err());
    }

    #[test]
    fn pin_selon_restriction() {
        assert!(check_pin_for_restriction(false, Some("1234")).is_ok());
        assert!(check_pin_for_restriction(false, None).is_err());
        assert!(check_pin_for_restriction(false, Some("12a4")).is_err());
        assert!(check_pin_for_restriction(true, None).is_ok());
        assert!(check_pin_for_restriction(true, Some("1234")).is_err());
    }

    #[test]
    fn preferences_par_defaut_si_cle_absente() {
        let p: Preferences = serde_json::from_str(r#"{"theme":"alternative"}"#).unwrap();
        assert_eq!(p.theme, "alternative");
        assert_eq!(p.text_scale, 1.0);
        assert_eq!(p.subtitles.color, "#FFFFFF");
        let empty: Preferences = serde_json::from_str("{}").unwrap();
        assert_eq!(empty, Preferences::default());
        assert!(empty.validate().is_ok());
    }

    #[test]
    fn preferences_invalides() {
        let mut p = Preferences {
            text_scale: 10.0,
            ..Default::default()
        };
        assert!(p.validate().is_err());
        p.text_scale = 1.0;
        p.subtitles.color = "blanc".into();
        assert!(p.validate().is_err());
    }

    #[test]
    fn langue_et_age() {
        assert!(check_language("fr").is_ok());
        assert!(check_language("pt-BR").is_ok());
        assert!(check_language("f").is_err());
        assert!(check_age(121).is_err());
        assert_eq!(check_age(35).unwrap(), 35);
    }

    #[test]
    fn invite_est_restreint_age_zero() {
        assert_eq!(ActiveProfile::Guest.visibility(), Visibility::MaxAge(0));
        assert!(!ActiveProfile::Guest.is_admin());
    }
}
