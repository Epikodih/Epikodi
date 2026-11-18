//! Listes de lecture (par profil).

use serde::{Deserialize, Serialize};

use crate::{DomainError, DomainResult, Id};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Playlist {
    pub id: Id,
    pub user_id: Id,
    pub name: String,
    pub item_count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaylistItem {
    pub position: i64,
    pub media_id: Id,
}

pub fn normalize_playlist_name(name: &str) -> DomainResult<String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 64 {
        return Err(DomainError::validation("name", "entre 1 et 64 caractères"));
    }
    Ok(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nom_de_liste() {
        assert_eq!(normalize_playlist_name(" Soirée ").unwrap(), "Soirée");
        assert!(normalize_playlist_name("").is_err());
    }
}
