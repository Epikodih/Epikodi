//! Sources de médias : dossier local ou partage réseau.

use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{DomainError, DomainResult, Id};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    Local,
    Smb,
    Dlna,
    Nfs,
    Webdav,
}

impl Protocol {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Smb => "smb",
            Self::Dlna => "dlna",
            Self::Nfs => "nfs",
            Self::Webdav => "webdav",
        }
    }

    /// Schémas d'URI acceptés pour ce protocole.
    pub fn uri_schemes(self) -> &'static [&'static str] {
        match self {
            Self::Local => &["file://"],
            Self::Smb => &["smb://"],
            Self::Dlna => &["http://", "https://"],
            Self::Nfs => &["nfs://"],
            Self::Webdav => &["http://", "https://", "webdav://", "webdavs://"],
        }
    }
}

impl FromStr for Protocol {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "local" => Ok(Self::Local),
            "smb" => Ok(Self::Smb),
            "dlna" => Ok(Self::Dlna),
            "nfs" => Ok(Self::Nfs),
            "webdav" => Ok(Self::Webdav),
            _ => Err(DomainError::validation(
                "protocol",
                "local, smb, dlna, nfs ou webdav",
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceStatus {
    Online,
    Offline,
    Error,
    #[default]
    Unknown,
}

impl SourceStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Online => "online",
            Self::Offline => "offline",
            Self::Error => "error",
            Self::Unknown => "unknown",
        }
    }
}

impl FromStr for SourceStatus {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "online" => Ok(Self::Online),
            "offline" => Ok(Self::Offline),
            "error" => Ok(Self::Error),
            "unknown" => Ok(Self::Unknown),
            _ => Err(DomainError::validation("status", s.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Source {
    pub id: Id,
    pub name: String,
    pub protocol: Protocol,
    pub uri: String,
    /// Des identifiants sont enregistrés (jamais leur valeur).
    pub has_credentials: bool,
    pub status: SourceStatus,
    pub last_scan_at: Option<String>,
}

/// Identifiants d'un partage. Ne doit jamais apparaître dans les logs ni
/// dans le protocole : `Debug` est masqué.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub domain: Option<String>,
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Credentials(***)")
    }
}

/// Vérifie le couple protocole / URI.
pub fn check_source_uri(protocol: Protocol, uri: &str) -> DomainResult<()> {
    let uri = uri.trim();
    let ok = protocol
        .uri_schemes()
        .iter()
        .any(|s| uri.len() > s.len() && uri.to_ascii_lowercase().starts_with(s));
    if ok {
        Ok(())
    } else {
        Err(DomainError::validation(
            "uri",
            format!("doit commencer par {}", protocol.uri_schemes().join(" ou ")),
        ))
    }
}

/// Extensions de fichiers vidéo reconnues.
pub const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "webm", "avi", "mov", "m4v", "ts", "wmv", "mpg", "mpeg", "ogv",
];
/// Extensions de fichiers audio reconnues.
pub const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "flac", "ogg", "oga", "opus", "m4a", "aac", "wav", "wma",
];

/// Extension en minuscules (`None` pour un fichier caché ou sans extension).
pub fn media_extension(file_name: &str) -> Option<String> {
    if file_name.starts_with('.') {
        return None;
    }
    let ext = file_name.rsplit_once('.')?.1.to_ascii_lowercase();
    (VIDEO_EXTENSIONS.contains(&ext.as_str()) || AUDIO_EXTENSIONS.contains(&ext.as_str()))
        .then_some(ext)
}

/// Un fichier vu par une source (résultat d'un listage).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FileEntry {
    /// URI complète, identité du fichier en base (`media.uri`).
    pub uri: String,
    /// Chemin relatif à la racine de la source (sert à la reconnaissance).
    pub relative_path: String,
    /// Dossier parent (URI), pour le scan ciblé.
    pub parent_dir: String,
    pub size_bytes: u64,
    /// Date de modification ISO 8601.
    pub modified_at: Option<String>,
}

/// Tags lus dans un fichier audio.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AudioTags {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub track_number: Option<u32>,
    pub year: Option<i32>,
    pub duration_s: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uri_selon_protocole() {
        assert!(check_source_uri(Protocol::Local, "file:///home/films").is_ok());
        assert!(check_source_uri(Protocol::Smb, "smb://nas/films").is_ok());
        assert!(check_source_uri(Protocol::Smb, "file:///x").is_err());
        assert!(check_source_uri(Protocol::Local, "file://").is_err());
    }

    #[test]
    fn identifiants_masques_dans_debug() {
        let c = Credentials {
            username: "marc".into(),
            password: "secret".into(),
            domain: None,
        };
        assert!(!format!("{c:?}").contains("secret"));
    }
}
