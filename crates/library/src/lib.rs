//! Médiathèque : scan des sources, reconnaissance des noms de fichiers,
//! enrichissement par des API publiques, correction manuelle.
//!
//! Ne dépend que de `domain` : la base, les fichiers et les fournisseurs
//! arrivent par les ports (`LibraryRepo`, `FileSource`, `MetadataProvider`).

pub mod enrichment;
pub mod manual_fix;
pub mod recognition;
pub mod scanner;
