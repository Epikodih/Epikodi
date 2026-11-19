//! Enrichissement automatique : affiche, résumé, casting, genres, note...
//!
//! - Un média `manual` n'est jamais écrasé.
//! - Fournisseur injoignable (hors connexion) : on s'arrête sans rien casser,
//!   le média reste `unmatched` et sera retenté au prochain scan.
//! - Les images sont téléchargées dans le cache local (consultable hors ligne).

use std::sync::Arc;

use epikodi_domain::media::{MatchStatus, Media, MediaKind, MediaPatch};
use epikodi_domain::ports::{
    ImageCache, LibraryRepo, MediaRepo, MetadataCandidate, MetadataError, MetadataProvider,
    MetadataQuery, MetadataRecord,
};
use epikodi_domain::{DomainError, DomainResult, Id};

/// Bilan d'un passage d'enrichissement.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnrichReport {
    pub enriched: u64,
    pub not_found: u64,
    /// Fournisseur indisponible : arrêt anticipé (mode hors ligne).
    pub offline: bool,
}

pub struct Enricher<'a, R: MediaRepo + LibraryRepo + ?Sized> {
    repo: &'a R,
    providers: &'a [Arc<dyn MetadataProvider>],
    images: &'a dyn ImageCache,
}

impl<'a, R: MediaRepo + LibraryRepo + ?Sized> Enricher<'a, R> {
    pub fn new(
        repo: &'a R,
        providers: &'a [Arc<dyn MetadataProvider>],
        images: &'a dyn ImageCache,
    ) -> Self {
        Self {
            repo,
            providers,
            images,
        }
    }

    /// Enrichit les médias non reconnus (films, séries, artistes, albums).
    pub fn enrich_unmatched(&self, limit: u32) -> DomainResult<EnrichReport> {
        let mut report = EnrichReport::default();
        for media in self.repo.unmatched(limit)? {
            match self.enrich(&media) {
                Ok(true) => report.enriched += 1,
                Ok(false) => report.not_found += 1,
                Err(EnrichError::Offline(e)) => {
                    tracing::info!("enrichissement interrompu (hors ligne ?) : {e}");
                    report.offline = true;
                    break;
                }
                Err(EnrichError::Domain(e)) => return Err(e),
            }
        }
        Ok(report)
    }

    /// Enrichit un média ; `Ok(false)` si aucun résultat.
    pub fn enrich(&self, media: &Media) -> Result<bool, EnrichError> {
        if !media.match_status.allows_auto_enrichment() {
            return Ok(false);
        }
        let query = query_for(self.repo, media)?;
        for provider in self.providers.iter().filter(|p| p.supports(media.kind)) {
            let candidates = match provider.search(&query) {
                Ok(c) => c,
                Err(MetadataError::Unavailable(e)) => return Err(EnrichError::Offline(e)),
                Err(e) => {
                    tracing::debug!(provider = provider.name(), "recherche : {e}");
                    continue;
                }
            };
            let Some(best) = best_candidate(&candidates, &query) else {
                continue;
            };
            let record = match provider.fetch(best.kind, &best.external_id) {
                Ok(r) => r,
                Err(MetadataError::Unavailable(e)) => return Err(EnrichError::Offline(e)),
                Err(_) => continue,
            };
            apply_record(
                self.repo,
                provider.as_ref(),
                self.images,
                media.id,
                record,
                MatchStatus::Auto,
            )?;
            return Ok(true);
        }
        Ok(false)
    }
}

#[derive(Debug)]
pub enum EnrichError {
    Offline(String),
    Domain(DomainError),
}

impl From<DomainError> for EnrichError {
    fn from(e: DomainError) -> Self {
        Self::Domain(e)
    }
}

/// Requête de recherche pour un média (l'artiste d'un album vient du parent).
pub fn query_for<R: MediaRepo + ?Sized>(repo: &R, media: &Media) -> DomainResult<MetadataQuery> {
    let artist = match (media.kind, media.parent_id) {
        (MediaKind::Album, Some(pid)) => repo.get_media(pid)?.map(|a| a.title),
        _ => None,
    };
    Ok(MetadataQuery {
        kind: Some(media.kind),
        title: media.title.clone(),
        year: media.year,
        artist,
        season: None,
        episode: None,
    })
}

/// Le premier résultat du bon type, en préférant la même année.
pub fn best_candidate<'c>(
    candidates: &'c [MetadataCandidate],
    q: &MetadataQuery,
) -> Option<&'c MetadataCandidate> {
    let same_kind = |c: &&MetadataCandidate| q.kind.is_none_or(|k| c.kind == k);
    candidates
        .iter()
        .filter(same_kind)
        .find(|c| q.year.is_some() && c.year == q.year)
        .or_else(|| candidates.iter().find(same_kind))
}

/// Écrit une fiche dans la base (et l'affiche dans le cache).
pub fn apply_record<R: MediaRepo + LibraryRepo + ?Sized>(
    repo: &R,
    provider: &dyn MetadataProvider,
    images: &dyn ImageCache,
    media_id: Id,
    record: MetadataRecord,
    status: MatchStatus,
) -> DomainResult<()> {
    let poster_path =
        record
            .poster_url
            .as_deref()
            .and_then(|url| match provider.fetch_image(url) {
                Ok(bytes) => images
                    .store(&format!("{}-{media_id}-poster", provider.name()), &bytes)
                    .map_err(|e| tracing::warn!("image non enregistrée : {e}"))
                    .ok(),
                Err(e) => {
                    tracing::debug!("image non téléchargée : {e}");
                    None
                }
            });
    let patch = MediaPatch {
        title: record.title.filter(|t| !t.trim().is_empty()),
        year: record.year.map(Some),
        age_rating: record.age_rating.map(Some),
        duration_s: record.duration_s.map(Some),
        poster_path: poster_path.map(Some),
        match_status: Some(status),
        ..Default::default()
    };
    let mut details = record.details;
    details.provider = Some(provider.name().to_string());
    // Champs et fiche écrits ensemble (jamais l'un sans l'autre).
    repo.atomically(&mut || {
        repo.patch_media(media_id, &patch)?;
        repo.set_details(media_id, &details)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use epikodi_domain::media::MediaDetails;
    use epikodi_storage::{cache::FsImageCache, insert_test_media, Store};
    use std::sync::atomic::{AtomicBool, Ordering};

    struct Fake {
        offline: AtomicBool,
    }

    impl MetadataProvider for Fake {
        fn name(&self) -> &str {
            "fake"
        }
        fn supports(&self, kind: MediaKind) -> bool {
            kind == MediaKind::Movie
        }
        fn search(&self, q: &MetadataQuery) -> Result<Vec<MetadataCandidate>, MetadataError> {
            if self.offline.load(Ordering::SeqCst) {
                return Err(MetadataError::Unavailable("pas de réseau".into()));
            }
            if q.title.to_lowercase().contains("sintel") {
                Ok(vec![
                    MetadataCandidate {
                        provider: "fake".into(),
                        external_id: "1".into(),
                        kind: MediaKind::Movie,
                        title: "Sintel (remake)".into(),
                        year: Some(2030),
                        overview: None,
                        poster_url: None,
                    },
                    MetadataCandidate {
                        provider: "fake".into(),
                        external_id: "2".into(),
                        kind: MediaKind::Movie,
                        title: "Sintel".into(),
                        year: Some(2010),
                        overview: None,
                        poster_url: None,
                    },
                ])
            } else {
                Ok(vec![])
            }
        }
        fn fetch(&self, _: MediaKind, id: &str) -> Result<MetadataRecord, MetadataError> {
            Ok(MetadataRecord {
                title: Some(
                    if id == "2" {
                        "Sintel"
                    } else {
                        "Sintel (remake)"
                    }
                    .into(),
                ),
                year: Some(if id == "2" { 2010 } else { 2030 }),
                age_rating: Some(10),
                duration_s: Some(888),
                poster_url: Some("http://img/sintel.jpg".into()),
                details: MediaDetails {
                    overview: Some("Une jeune fille et un dragon.".into()),
                    genres: vec!["Animation".into()],
                    ..Default::default()
                },
            })
        }
        fn fetch_image(&self, _: &str) -> Result<Vec<u8>, MetadataError> {
            Ok(vec![0xFF, 0xD8, 0xFF])
        }
    }

    fn setup() -> (Store, tempfile::TempDir, Vec<Arc<dyn MetadataProvider>>) {
        let store = Store::in_memory();
        let providers: Vec<Arc<dyn MetadataProvider>> = vec![Arc::new(Fake {
            offline: AtomicBool::new(false),
        })];
        (store, tempfile::tempdir().unwrap(), providers)
    }

    #[test]
    fn enrichit_avec_le_resultat_de_la_bonne_annee() {
        let (store, dir, providers) = setup();
        let cache = FsImageCache::new(dir.path());
        let id = insert_test_media(&store, None, MediaKind::Movie, "sintel", None);
        store
            .patch_media(
                id,
                &MediaPatch {
                    year: Some(Some(2010)),
                    match_status: Some(MatchStatus::Unmatched),
                    ..Default::default()
                },
            )
            .unwrap();
        let r = Enricher::new(&store, &providers, &cache)
            .enrich_unmatched(10)
            .unwrap();
        assert_eq!(r.enriched, 1);
        let m = store.get_media(id).unwrap().unwrap();
        assert_eq!(m.title, "Sintel");
        assert_eq!(m.year, Some(2010));
        assert_eq!(m.age_rating, Some(10));
        assert_eq!(m.match_status, MatchStatus::Auto);
        assert!(m.poster_path.is_some(), "affiche dans le cache");
        let d = store.get_details(id).unwrap().unwrap();
        assert_eq!(d.provider.as_deref(), Some("fake"));
        assert_eq!(d.genres, ["Animation"]);
    }

    #[test]
    fn manuel_jamais_ecrase() {
        let (store, dir, providers) = setup();
        let cache = FsImageCache::new(dir.path());
        let id = insert_test_media(&store, None, MediaKind::Movie, "Sintel ma version", None);
        store
            .patch_media(
                id,
                &MediaPatch {
                    match_status: Some(MatchStatus::Manual),
                    ..Default::default()
                },
            )
            .unwrap();
        let m = store.get_media(id).unwrap().unwrap();
        assert!(!Enricher::new(&store, &providers, &cache)
            .enrich(&m)
            .unwrap());
        assert_eq!(
            store.get_media(id).unwrap().unwrap().title,
            "Sintel ma version"
        );
    }

    #[test]
    fn hors_connexion_rien_ne_casse() {
        let store = Store::in_memory();
        let dir = tempfile::tempdir().unwrap();
        let cache = FsImageCache::new(dir.path());
        let providers: Vec<Arc<dyn MetadataProvider>> = vec![Arc::new(Fake {
            offline: AtomicBool::new(true),
        })];
        let id = insert_test_media(&store, None, MediaKind::Movie, "Sintel", None);
        store
            .patch_media(
                id,
                &MediaPatch {
                    match_status: Some(MatchStatus::Unmatched),
                    ..Default::default()
                },
            )
            .unwrap();
        let r = Enricher::new(&store, &providers, &cache)
            .enrich_unmatched(10)
            .unwrap();
        assert!(r.offline);
        assert_eq!(r.enriched, 0);
        assert_eq!(
            store.get_media(id).unwrap().unwrap().match_status,
            MatchStatus::Unmatched
        );
    }
}
