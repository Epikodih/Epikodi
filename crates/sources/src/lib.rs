//! Accès aux fichiers des sources : dossier local, partage SMB, mode dégradé
//! (partage lent ou injoignable) et découverte des partages sur le réseau.
//!
//! Chaque source implémente `domain::ports::FileSource`.

pub mod degraded;
pub mod discovery;
pub mod local;
pub mod smb;
pub mod source;

pub use source::{open_source, playback_path};
