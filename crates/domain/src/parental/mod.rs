//! Contrôle parental : règle de visibilité et format du PIN.
//!
//! Un profil restreint ne voit que les médias dont la classification est
//! connue et inférieure ou égale à son âge. Classification inconnue = masqué.
//! Un épisode ou un morceau sans classification hérite de celle de son
//! parent (saison, série, album, artiste) : c'est la classification
//! « effective ».

use crate::{DomainError, DomainResult};

/// Ce qu'un profil a le droit de voir. Obligatoire pour toute requête qui
/// liste le catalogue, pour qu'on ne puisse pas l'oublier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// Compte non restreint : tout.
    All,
    /// Profil restreint (ou invité, âge 0).
    MaxAge(u8),
}

impl Visibility {
    /// `ratings` : classification du média puis celles de ses ancêtres
    /// (parent, grand-parent). La première connue fait foi.
    pub fn allows(self, ratings: &[Option<u8>]) -> bool {
        match self {
            Self::All => true,
            Self::MaxAge(age) => effective_rating(ratings).is_some_and(|r| r <= age),
        }
    }
}

/// Première classification connue en remontant la hiérarchie.
pub fn effective_rating(ratings: &[Option<u8>]) -> Option<u8> {
    ratings.iter().find_map(|r| *r)
}

pub const PIN_LEN: usize = 4;

/// Un PIN fait exactement 4 chiffres.
pub fn check_pin_format(pin: &str) -> DomainResult<()> {
    if pin.len() == PIN_LEN && pin.chars().all(|c| c.is_ascii_digit()) {
        Ok(())
    } else {
        Err(DomainError::validation("pin", "4 chiffres"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_restreint_voit_tout() {
        assert!(Visibility::All.allows(&[None]));
        assert!(Visibility::All.allows(&[Some(18)]));
    }

    #[test]
    fn restreint_voit_seulement_classification_connue_et_inferieure() {
        let v = Visibility::MaxAge(10);
        assert!(v.allows(&[Some(0)]));
        assert!(v.allows(&[Some(10)]));
        assert!(!v.allows(&[Some(12)]));
        assert!(!v.allows(&[None]), "classification inconnue = masqué");
    }

    #[test]
    fn heritage_du_parent() {
        let v = Visibility::MaxAge(12);
        // épisode sans note, saison sans note, série 10 ans
        assert!(v.allows(&[None, None, Some(10)]));
        // la note propre du média prime sur celle du parent
        assert!(!v.allows(&[Some(16), Some(10)]));
        assert!(!v.allows(&[None, None, None]));
    }

    #[test]
    fn invite_age_zero() {
        let v = Visibility::MaxAge(0);
        assert!(v.allows(&[Some(0)]));
        assert!(!v.allows(&[Some(10)]));
    }

    #[test]
    fn format_pin() {
        assert!(check_pin_format("0000").is_ok());
        assert!(check_pin_format("123").is_err());
        assert!(check_pin_format("12345").is_err());
        assert!(check_pin_format("12a4").is_err());
        assert!(check_pin_format("١٢٣٤").is_err());
    }
}
