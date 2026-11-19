//! Erreurs du domaine, indépendantes du transport (HTTP) et de la base.

use thiserror::Error;

pub type DomainResult<T> = Result<T, DomainError>;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DomainError {
    /// Un champ ne respecte pas les règles (nom vide, PIN mal formé...).
    #[error("champ invalide « {field} » : {reason}")]
    Validation { field: String, reason: String },

    /// L'objet demandé n'existe pas.
    #[error("{entity} introuvable")]
    NotFound { entity: &'static str },

    /// Une contrainte d'unicité est violée (nom de profil déjà pris...).
    #[error("{entity} existe déjà")]
    Conflict { entity: &'static str },

    /// Une règle métier refuse l'opération.
    #[error("{0}")]
    Rule(String),

    /// Erreur technique remontée par un port (base, disque, réseau).
    #[error("erreur interne : {0}")]
    Internal(String),
}

impl DomainError {
    pub fn validation(field: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Validation {
            field: field.into(),
            reason: reason.into(),
        }
    }

    pub fn internal(e: impl std::fmt::Display) -> Self {
        Self::Internal(e.to_string())
    }
}
