//! Fournisseurs d'informations sur les médias.
//!
//! Chaque fournisseur implémente `domain::ports::MetadataProvider`. Les
//! appels HTTP passent par le trait [`provider::Http`] : les tests utilisent
//! des réponses enregistrées et passent sans connexion.
//!
//! Attribution demandée par les services (à afficher dans l'app) :
//! - TMDB : « This product uses the TMDB API but is not endorsed or certified by TMDB. »
//! - MusicBrainz : données sous licence CC0, pochettes du Cover Art Archive.

pub mod mock;
pub mod musicbrainz;
pub mod provider;
pub mod tmdb;

pub use mock::DemoCatalog;
pub use musicbrainz::MusicBrainz;
pub use provider::{Http, UreqHttp};
pub use tmdb::Tmdb;
