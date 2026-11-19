//! Partage SMB (Windows, NAS, box) : `smb://serveur/partage/dossier`.
//!
//! Plutôt que d'embarquer un client SMB, on passe par le système, qui gère
//! déjà l'authentification et le cache :
//! - Windows : chemin UNC `\\serveur\partage\dossier` (lu directement) ;
//! - Linux : partage monté par GVFS (`gio mount smb://...`, ou en l'ouvrant
//!   dans le gestionnaire de fichiers), sous `$XDG_RUNTIME_DIR/gvfs/`, ou un
//!   point de montage indiqué par `SMB_MOUNT_ROOT` (`<racine>/<serveur>/<partage>`).
//!
//! Les URI enregistrées restent `smb://...` : le média garde son identité
//! même si le point de montage change.

use std::path::PathBuf;

use epikodi_domain::ports::{FileSource, SourceError};
use epikodi_domain::source::{AudioTags, Credentials, FileEntry};

use crate::local::LocalSource;

/// Variable d'environnement : racine des partages montés à la main.
pub const MOUNT_ROOT_ENV: &str = "SMB_MOUNT_ROOT";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmbPath {
    pub host: String,
    pub share: String,
    /// Chemin dans le partage, sans `/` initial.
    pub path: String,
}

pub fn parse_smb_uri(uri: &str) -> Result<SmbPath, SourceError> {
    let rest = uri
        .get(6..)
        .filter(|_| uri[..6].eq_ignore_ascii_case("smb://"))
        .ok_or_else(|| SourceError::NotFound(uri.into()))?;
    // on ignore « user@ » éventuel : les identifiants ne vont pas dans l'URI
    let rest = rest.rsplit_once('@').map_or(rest, |(_, r)| r);
    let mut parts = rest.trim_end_matches('/').splitn(3, '/');
    let host = parts.next().filter(|h| !h.is_empty());
    let share = parts.next().filter(|s| !s.is_empty());
    match (host, share) {
        (Some(h), Some(s)) => Ok(SmbPath {
            host: h.to_string(),
            share: s.to_string(),
            path: parts.next().unwrap_or("").to_string(),
        }),
        _ => Err(SourceError::NotFound(format!(
            "{uri} : forme attendue smb://serveur/partage"
        ))),
    }
}

/// Chemin local par lequel le système expose le partage.
pub fn resolve_smb(uri: &str) -> Result<PathBuf, SourceError> {
    let p = parse_smb_uri(uri)?;
    let mut candidates: Vec<PathBuf> = Vec::new();

    if cfg!(windows) {
        let mut unc = format!(r"\\{}\{}", p.host, p.share);
        if !p.path.is_empty() {
            unc.push('\\');
            unc.push_str(&p.path.replace('/', "\\"));
        }
        candidates.push(PathBuf::from(unc));
    }
    if let Some(root) = std::env::var_os(MOUNT_ROOT_ENV) {
        candidates.push(
            PathBuf::from(root)
                .join(&p.host)
                .join(&p.share)
                .join(&p.path),
        );
    }
    if let Some(run) = std::env::var_os("XDG_RUNTIME_DIR") {
        let gvfs = PathBuf::from(run).join("gvfs");
        let name = format!(
            "smb-share:server={},share={}",
            p.host.to_lowercase(),
            p.share.to_lowercase()
        );
        candidates.push(gvfs.join(name).join(&p.path));
    }

    candidates.into_iter().find(|c| c.is_dir()).ok_or_else(|| {
        SourceError::Unreachable(format!(
            "partage {uri} injoignable ou non monté (Linux : `gio mount smb://{}/{}`)",
            p.host, p.share
        ))
    })
}

pub struct SmbSource {
    uri: String,
}

impl SmbSource {
    /// Les identifiants servent au montage par le système ; on ne les garde
    /// pas en mémoire plus que nécessaire.
    pub fn open(uri: &str, _credentials: Option<Credentials>) -> Result<Self, SourceError> {
        parse_smb_uri(uri)?;
        Ok(Self {
            uri: uri.trim_end_matches('/').to_string(),
        })
    }

    /// Résolu à chaque appel : le partage peut être monté ou démonté entre
    /// deux scans.
    fn local(&self) -> Result<LocalSource, SourceError> {
        Ok(LocalSource::new(resolve_smb(&self.uri)?, self.uri.clone()))
    }
}

impl FileSource for SmbSource {
    fn list(&self) -> Result<Vec<FileEntry>, SourceError> {
        self.local()?.list()
    }

    fn list_dir(&self, dir_uri: &str) -> Result<Vec<FileEntry>, SourceError> {
        self.local()?.list_dir(dir_uri)
    }

    fn audio_tags(&self, entry: &FileEntry) -> Option<AudioTags> {
        self.local().ok()?.audio_tags(entry)
    }

    fn fingerprint(&self, entry: &FileEntry) -> Option<String> {
        self.local().ok()?.fingerprint(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyse_de_l_uri() {
        assert_eq!(
            parse_smb_uri("smb://nas/films/action").unwrap(),
            SmbPath {
                host: "nas".into(),
                share: "films".into(),
                path: "action".into()
            }
        );
        assert_eq!(
            parse_smb_uri("SMB://marc@nas/films/").unwrap().share,
            "films"
        );
        assert!(parse_smb_uri("smb://nas").is_err());
        assert!(parse_smb_uri("file:///x").is_err());
    }

    #[test]
    fn partage_monte_via_la_racine_configuree() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("nas/films")).unwrap();
        std::fs::write(d.path().join("nas/films/Sintel.mkv"), b"x").unwrap();
        std::env::set_var(MOUNT_ROOT_ENV, d.path());
        let src = SmbSource::open("smb://nas/films", None).unwrap();
        let files = src.list().unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(
            files[0].uri, "smb://nas/films/Sintel.mkv",
            "l'URI reste smb://"
        );
        assert!(SmbSource::open("smb://nas/absent", None)
            .unwrap()
            .list()
            .is_err());
    }
}
