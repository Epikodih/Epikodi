//! Téléphones appairés.

use serde::{Deserialize, Serialize};

use crate::Id;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Device {
    pub id: Id,
    pub name: String,
    pub install_id: Option<String>,
    pub platform: Option<String>,
    pub paired_at: String,
    pub expires_at: Option<String>,
    pub last_seen_at: Option<String>,
    pub revoked_at: Option<String>,
}

/// Données d'un nouvel appairage. Seul le hash du jeton est stocké.
#[derive(Debug, Clone, PartialEq)]
pub struct NewDevice {
    pub name: String,
    pub install_id: Option<String>,
    pub platform: Option<String>,
    pub token_hash: String,
    /// Date d\'appairage (horloge du serveur).
    pub paired_at: String,
    pub expires_at: Option<String>,
}
