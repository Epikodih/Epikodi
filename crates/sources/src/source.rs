//! Ouverture d'une source selon son protocole.

use std::path::PathBuf;
use std::time::Duration;

use epikodi_domain::ports::{FileSource, SourceError};
use epikodi_domain::source::{Credentials, Protocol};

use crate::degraded::TimeoutSource;
use crate::local::{path_from_file_uri, LocalSource};
use crate::smb::{resolve_smb, SmbSource};

/// Délai maximal d'un listage (partage lent).
pub const LIST_TIMEOUT: Duration = Duration::from_secs(120);

/// Ouvre une source. Les protocoles non pris en charge renvoient une erreur
/// claire (la source reste enregistrée, en statut `error`).
pub fn open_source(
    protocol: Protocol,
    uri: &str,
    credentials: Option<Credentials>,
) -> Result<Box<dyn FileSource>, SourceError> {
    let inner: Box<dyn FileSource> = match protocol {
        Protocol::Local => Box::new(LocalSource::from_uri(uri)?),
        Protocol::Smb => Box::new(SmbSource::open(uri, credentials)?),
        other => {
            return Err(SourceError::Unsupported(format!(
                "{} : pas encore pris en charge (utiliser un dossier local ou SMB)",
                other.as_str()
            )))
        }
    };
    Ok(Box::new(TimeoutSource::new(inner, LIST_TIMEOUT)))
}

/// Chemin que le lecteur peut ouvrir pour un `media.uri`, si le fichier est
/// accessible depuis cette machine (dossier local ou partage monté).
pub fn playback_path(uri: &str) -> Option<PathBuf> {
    let lower = uri.to_ascii_lowercase();
    if lower.starts_with("file://") {
        path_from_file_uri(uri)
    } else if lower.starts_with("smb://") {
        resolve_smb(uri).ok()
    } else {
        None
    }
}
