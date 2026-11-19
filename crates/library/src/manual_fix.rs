//! Correction manuelle : l'utilisateur corrige une reconnaissance erronée,
//! soit en modifiant les champs, soit en choisissant parmi plusieurs résultats.
//! Le média passe `manual` : plus aucun enrichissement automatique ne le touche.

use std::sync::Arc;

use epikodi_domain::media::{check_age_rating, MatchStatus, MediaPatch};
use epikodi_domain::ports::{
    ImageCache, LibraryRepo, MediaRepo, MetadataCandidate, MetadataError, MetadataProvider,
    MetadataQuery,
};
use epikodi_domain::{DomainError, DomainResult, Id};

use crate::enrichment::{apply_record, query_for};

/// Corrige des champs à la main.
pub fn edit<R: MediaRepo + ?Sized>(
    repo: &R,
    media_id: Id,
    mut patch: MediaPatch,
) -> DomainResult<()> {
    if repo.get_media(media_id)?.is_none() {
        return Err(DomainError::NotFound { entity: "média" });
    }
    if let Some(t) = &patch.title {
        let t = t.trim();
        if t.is_empty() || t.chars().count() > 200 {
            return Err(DomainError::validation(
                "title",
                "entre 1 et 200 caractères",
            ));
        }
        patch.title = Some(t.to_string());
    }
    if let Some(Some(y)) = patch.year {
        if !(1870..=2100).contains(&y) {
            return Err(DomainError::validation("year", "année invalide"));
        }
    }
    if let Some(r) = patch.age_rating {
        check_age_rating(r)?;
    }
    patch.match_status = Some(MatchStatus::Manual);
    repo.patch_media(media_id, &patch)
}

/// Résultats proposés chez tous les fournisseurs (recherche libre possible).
pub fn candidates<R: MediaRepo + ?Sized>(
    repo: &R,
    providers: &[Arc<dyn MetadataProvider>],
    media_id: Id,
    search: Option<&str>,
    year: Option<i32>,
) -> DomainResult<Vec<MetadataCandidate>> {
    let media = repo
        .get_media(media_id)?
        .ok_or(DomainError::NotFound { entity: "média" })?;
    let mut q: MetadataQuery = query_for(repo, &media)?;
    if let Some(s) = search.map(str::trim).filter(|s| !s.is_empty()) {
        q.title = s.to_string();
        q.year = year;
    } else if year.is_some() {
        q.year = year;
    }
    let mut out = Vec::new();
    let mut offline = None;
    for p in providers.iter().filter(|p| p.supports(media.kind)) {
        match p.search(&q) {
            Ok(c) => out.extend(c),
            Err(MetadataError::Unavailable(e)) => offline = Some(e),
            Err(e) => tracing::debug!(provider = p.name(), "recherche : {e}"),
        }
    }
    if out.is_empty() {
        if let Some(e) = offline {
            return Err(DomainError::Rule(format!("fournisseur injoignable : {e}")));
        }
    }
    Ok(out)
}

/// Applique le résultat choisi par l'utilisateur.
pub fn apply<R: MediaRepo + LibraryRepo + ?Sized>(
    repo: &R,
    providers: &[Arc<dyn MetadataProvider>],
    images: &dyn ImageCache,
    media_id: Id,
    provider_name: &str,
    external_id: &str,
) -> DomainResult<()> {
    let media = repo
        .get_media(media_id)?
        .ok_or(DomainError::NotFound { entity: "média" })?;
    let provider = providers
        .iter()
        .find(|p| p.name() == provider_name)
        .ok_or_else(|| DomainError::validation("provider", "fournisseur inconnu"))?;
    let record = provider
        .fetch(media.kind, external_id)
        .map_err(|e| match e {
            MetadataError::NotFound => {
                DomainError::validation("external_id", "introuvable chez ce fournisseur")
            }
            e => DomainError::Rule(e.to_string()),
        })?;
    apply_record(
        repo,
        provider.as_ref(),
        images,
        media_id,
        record,
        MatchStatus::Manual,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use epikodi_domain::media::MediaKind;
    use epikodi_storage::{insert_test_media, Store};

    #[test]
    fn correction_manuelle_passe_en_manual() {
        let store = Store::in_memory();
        let id = insert_test_media(&store, None, MediaKind::Movie, "mon_film_2019_final", None);
        edit(
            &store,
            id,
            MediaPatch {
                title: Some("  Mon Film  ".into()),
                year: Some(Some(2019)),
                age_rating: Some(Some(12)),
                ..Default::default()
            },
        )
        .unwrap();
        let m = store.get_media(id).unwrap().unwrap();
        assert_eq!(m.title, "Mon Film");
        assert_eq!(m.match_status, MatchStatus::Manual);
    }

    #[test]
    fn correction_invalide_refusee() {
        let store = Store::in_memory();
        let id = insert_test_media(&store, None, MediaKind::Movie, "x", None);
        let bad = |p: MediaPatch| edit(&store, id, p).is_err();
        assert!(bad(MediaPatch {
            title: Some("  ".into()),
            ..Default::default()
        }));
        assert!(bad(MediaPatch {
            age_rating: Some(Some(13)),
            ..Default::default()
        }));
        assert!(bad(MediaPatch {
            year: Some(Some(3000)),
            ..Default::default()
        }));
        assert!(edit(&store, 999, MediaPatch::default()).is_err());
    }
}
