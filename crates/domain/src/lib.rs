//! Entités et règles métier du media center.
//!
//! Aucune I/O, aucun SQL : tout ce qui touche la base passe par les traits de
//! [`ports`], implémentés par `storage`. Les autres crates (library, api...)
//! s'appuient sur ces types.

pub mod app_state;
pub mod device;
pub mod error;
pub mod extension;
pub mod media;
pub mod parental;
pub mod playback;
pub mod playlist;
pub mod ports;
pub mod profile;
pub mod protocol;
pub mod queue;
pub mod source;

pub use error::{DomainError, DomainResult};

/// Identifiant d'une ligne en base.
pub type Id = i64;
