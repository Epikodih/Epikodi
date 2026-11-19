//! Scan des sources : complet ou ciblé (un dossier), toujours incrémental.
//!
//! - Identité d'un fichier = `media.uri` : un re-scan met à jour, ne duplique pas.
//! - Fichier inchangé (même taille, même date) : rien n'est écrit.
//! - Fichier disparu : `status = missing`, la ligne est gardée (favoris,
//!   playlists et progression survivent à un NAS débranché).
//! - Fichier déplacé : retrouvé par son empreinte, on met à jour son `uri`.
//! - Source injoignable : on ne marque RIEN comme disparu, la médiathèque
//!   reste consultable et la source passe `offline`.
//! - Une transaction par fichier : un crash ne laisse pas une série à moitié écrite.

use std::collections::{HashMap, HashSet};
use std::time::Instant;

use epikodi_domain::media::{MatchStatus, Media, MediaKind, MediaStatus, NewMedia};
use epikodi_domain::ports::{FileSource, LibraryRepo, SourceError};
use epikodi_domain::source::{FileEntry, SourceStatus};
use epikodi_domain::{DomainError, DomainResult, Id};

use crate::recognition::{recognize, Recognized};

/// Bilan d'un scan.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScanReport {
    pub added: u64,
    pub updated: u64,
    pub moved: u64,
    pub missing: u64,
    pub unchanged: u64,
    pub skipped: u64,
    pub errors: Vec<String>,
    /// Source injoignable (rien n'a été marqué disparu).
    pub source_error: Option<String>,
    pub duration_ms: u64,
}

pub struct Scanner<'a, R: LibraryRepo + ?Sized> {
    repo: &'a R,
    source: &'a dyn FileSource,
    source_id: Id,
}

impl<'a, R: LibraryRepo + ?Sized> Scanner<'a, R> {
    pub fn new(repo: &'a R, source: &'a dyn FileSource, source_id: Id) -> Self {
        Self {
            repo,
            source,
            source_id,
        }
    }

    /// Scan complet de la source.
    pub fn scan(&self) -> DomainResult<ScanReport> {
        self.run(None)
    }

    /// Scan ciblé d'un dossier (URI du dossier, comme `media.parent_dir`).
    pub fn scan_dir(&self, dir_uri: &str) -> DomainResult<ScanReport> {
        self.run(Some(dir_uri))
    }

    fn run(&self, dir: Option<&str>) -> DomainResult<ScanReport> {
        let started = Instant::now();
        let mut report = ScanReport::default();
        let now = now_iso();

        let listing = match dir {
            Some(d) => self.source.list_dir(d),
            None => self.source.list(),
        };
        let entries = match listing {
            Ok(e) => e,
            Err(e) => {
                let status = match e {
                    SourceError::Unreachable(_) | SourceError::Timeout => SourceStatus::Offline,
                    _ => SourceStatus::Error,
                };
                tracing::warn!(source_id = self.source_id, "scan impossible : {e}");
                self.repo.set_source_scanned(self.source_id, status, &now)?;
                report.source_error = Some(e.to_string());
                report.duration_ms = started.elapsed().as_millis() as u64;
                return Ok(report);
            }
        };

        let known: HashMap<String, Media> = self
            .repo
            .known_files(self.source_id)?
            .into_iter()
            .filter(|m| dir.is_none_or(|d| m.parent_dir.as_deref() == Some(d)))
            .filter_map(|m| m.uri.clone().map(|u| (u, m)))
            .collect();
        let seen: HashSet<&str> = entries.iter().map(|e| e.uri.as_str()).collect();

        // D'abord les disparus : ils restent en base, marqués `missing`. Les
        // marquer avant permet de retrouver un fichier déplacé dès ce scan.
        let mut vanished: HashSet<Id> = HashSet::new();
        for (uri, m) in &known {
            if !seen.contains(uri.as_str()) && m.status == MediaStatus::Available {
                self.repo.set_media_status(m.id, MediaStatus::Missing)?;
                vanished.insert(m.id);
            }
        }

        let mut moved_from: HashSet<Id> = HashSet::new();
        for entry in &entries {
            let mut outcome = Outcome::Skipped;
            let res = self.repo.atomically(&mut || {
                outcome = self.process(entry, known.get(&entry.uri))?;
                Ok(())
            });
            match res {
                Ok(()) => match outcome {
                    Outcome::Added => report.added += 1,
                    Outcome::Updated => report.updated += 1,
                    Outcome::Moved(id) => {
                        report.moved += 1;
                        moved_from.insert(id);
                    }
                    Outcome::Unchanged => report.unchanged += 1,
                    Outcome::Skipped => report.skipped += 1,
                },
                Err(e) => {
                    tracing::warn!(uri = %entry.uri, "fichier ignoré : {e}");
                    report.errors.push(format!("{} : {e}", entry.relative_path));
                }
            }
        }
        report.missing = vanished.difference(&moved_from).count() as u64;

        self.repo
            .set_source_scanned(self.source_id, SourceStatus::Online, &now)?;
        report.duration_ms = started.elapsed().as_millis() as u64;
        tracing::info!(source_id = self.source_id, ?report, "scan terminé");
        Ok(report)
    }

    fn process(&self, entry: &FileEntry, known: Option<&Media>) -> DomainResult<Outcome> {
        if let Some(m) = known {
            let same = m.size_bytes == Some(entry.size_bytes as i64)
                && m.modified_at == entry.modified_at
                && m.status == MediaStatus::Available;
            if same {
                return Ok(Outcome::Unchanged);
            }
            self.repo
                .update_file(m.id, &self.file_fields(entry, None))?;
            return Ok(Outcome::Updated);
        }

        let fingerprint = self.source.fingerprint(entry);
        if let Some(fp) = &fingerprint {
            if let Some(old) = self.repo.find_missing_by_fingerprint(fp)? {
                self.repo
                    .update_file(old.id, &self.file_fields(entry, fingerprint.clone()))?;
                return Ok(Outcome::Moved(old.id));
            }
        }

        let tags = self.source.audio_tags(entry);
        let Some(rec) = recognize(&entry.relative_path, tags.as_ref()) else {
            return Ok(Outcome::Skipped);
        };
        let mut new = self.file_fields(entry, fingerprint);
        match rec {
            Recognized::Movie { title, year } => {
                new.kind = Some(MediaKind::Movie);
                new.title = title;
                new.year = year;
            }
            Recognized::Episode {
                series,
                series_year,
                season,
                episode,
                title,
            } => {
                let series_id =
                    self.container(MediaKind::Series, None, &series, None, series_year)?;
                let season_id = self.container(
                    MediaKind::Season,
                    Some(series_id),
                    &format!("Saison {season}"),
                    Some(season),
                    None,
                )?;
                new.kind = Some(MediaKind::Episode);
                new.parent_id = Some(season_id);
                new.sort_number = Some(episode);
                new.title = title.unwrap_or_else(|| format!("Épisode {episode}"));
            }
            Recognized::Track {
                artist,
                album,
                title,
                track,
                year,
                duration_s,
            } => {
                let artist_id = self.container(MediaKind::Artist, None, &artist, None, None)?;
                let album_id =
                    self.container(MediaKind::Album, Some(artist_id), &album, None, year)?;
                new.kind = Some(MediaKind::Track);
                new.parent_id = Some(album_id);
                new.sort_number = track.map(i64::from);
                new.title = title;
                new.year = year;
                new.duration_s = duration_s;
            }
        }
        self.repo.insert_media(&new)?;
        Ok(Outcome::Added)
    }

    /// Série, saison, artiste ou album : créé à la demande puis réutilisé.
    fn container(
        &self,
        kind: MediaKind,
        parent_id: Option<Id>,
        title: &str,
        sort_number: Option<i64>,
        year: Option<i32>,
    ) -> DomainResult<Id> {
        if let Some(id) = self
            .repo
            .find_container(kind, parent_id, title, sort_number)?
        {
            return Ok(id);
        }
        self.repo.insert_media(&NewMedia {
            kind: Some(kind),
            parent_id,
            title: title.to_string(),
            sort_number,
            year,
            match_status: Some(MatchStatus::Unmatched),
            ..Default::default()
        })
    }

    fn file_fields(&self, entry: &FileEntry, fingerprint: Option<String>) -> NewMedia {
        NewMedia {
            source_id: Some(self.source_id),
            uri: Some(entry.uri.clone()),
            parent_dir: Some(entry.parent_dir.clone()),
            size_bytes: Some(entry.size_bytes as i64),
            modified_at: entry.modified_at.clone(),
            fingerprint: fingerprint.or_else(|| self.source.fingerprint(entry)),
            match_status: Some(MatchStatus::Unmatched),
            ..Default::default()
        }
    }
}

enum Outcome {
    Added,
    Updated,
    Moved(Id),
    Unchanged,
    Skipped,
}

fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Erreur « source introuvable » pour l'appelant.
pub fn source_not_found() -> DomainError {
    DomainError::NotFound { entity: "source" }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    use epikodi_domain::media::MediaQuery;
    use epikodi_domain::parental::Visibility;
    use epikodi_domain::ports::{MediaRepo, NewSource, SourceRepo};
    use epikodi_domain::source::Protocol;
    use epikodi_storage::Store;

    /// Fausse source : une liste de fichiers modifiable entre deux scans.
    struct FakeSource {
        files: Mutex<Result<Vec<FileEntry>, SourceError>>,
    }

    impl FakeSource {
        fn new(paths: &[&str]) -> Self {
            let s = Self {
                files: Mutex::new(Ok(Vec::new())),
            };
            s.set(paths);
            s
        }

        fn set(&self, paths: &[&str]) {
            *self.files.lock().unwrap() = Ok(paths
                .iter()
                .map(|p| entry(p, 1000, "2026-01-01T00:00:00Z"))
                .collect());
        }

        fn set_entries(&self, e: Vec<FileEntry>) {
            *self.files.lock().unwrap() = Ok(e);
        }

        fn fail(&self, e: SourceError) {
            *self.files.lock().unwrap() = Err(e);
        }
    }

    fn entry(path: &str, size: u64, modified: &str) -> FileEntry {
        let parent = path.rsplit_once('/').map_or("", |(d, _)| d);
        FileEntry {
            uri: format!("file:///media/{path}"),
            relative_path: path.into(),
            parent_dir: format!("file:///media/{parent}"),
            size_bytes: size,
            modified_at: Some(modified.into()),
        }
    }

    impl FileSource for FakeSource {
        fn list(&self) -> Result<Vec<FileEntry>, SourceError> {
            self.files.lock().unwrap().clone()
        }
    }

    fn setup() -> (Store, Id) {
        let store = Store::in_memory();
        let src = store
            .create_source(&NewSource {
                name: "Démo".into(),
                protocol: Protocol::Local,
                uri: "file:///media".into(),
                credentials: None,
            })
            .unwrap();
        (store, src.id)
    }

    fn count(store: &Store, kind: MediaKind) -> u64 {
        store
            .list_media(
                &MediaQuery {
                    kind: Some(kind),
                    limit: 500,
                    ..Default::default()
                },
                Visibility::All,
            )
            .unwrap()
            .total
    }

    const FILES: &[&str] = &[
        "Films/Sintel (2010).mkv",
        "Films/Big_Buck_Bunny_1080p.mp4",
        "Séries/Série Démo/Saison 1/Série Démo - S01E01.mkv",
        "Séries/Série Démo/Saison 1/Série Démo - S01E02.mkv",
        "Séries/Série Démo/Saison 2/Série Démo - S02E01.mkv",
        "Musique/Artiste/Album/01 - Un.mp3",
        "Musique/Artiste/Album/02 - Deux.mp3",
        "Films/Sintel (2010).srt",
    ];

    #[test]
    fn premier_scan_cree_la_hierarchie() {
        let (store, sid) = setup();
        let src = FakeSource::new(FILES);
        let r = Scanner::new(&store, &src, sid).scan().unwrap();
        assert_eq!(r.added, 7);
        assert_eq!(r.skipped, 1, "le .srt n'est pas un média");
        assert_eq!(count(&store, MediaKind::Movie), 2);
        assert_eq!(
            count(&store, MediaKind::Series),
            1,
            "série créée une seule fois"
        );
        assert_eq!(count(&store, MediaKind::Season), 2);
        assert_eq!(count(&store, MediaKind::Episode), 3);
        assert_eq!(count(&store, MediaKind::Artist), 1);
        assert_eq!(count(&store, MediaKind::Album), 1);
        assert_eq!(count(&store, MediaKind::Track), 2);
        let source = store.get_source(sid).unwrap().unwrap();
        assert_eq!(source.status, SourceStatus::Online);
        assert!(source.last_scan_at.is_some());
    }

    #[test]
    fn rescan_idempotent() {
        let (store, sid) = setup();
        let src = FakeSource::new(FILES);
        Scanner::new(&store, &src, sid).scan().unwrap();
        let r = Scanner::new(&store, &src, sid).scan().unwrap();
        assert_eq!((r.added, r.updated, r.unchanged), (0, 0, 7));
        assert_eq!(count(&store, MediaKind::Episode), 3, "pas de doublon");
    }

    #[test]
    fn scan_incremental_ajout_modification_suppression() {
        let (store, sid) = setup();
        let src = FakeSource::new(FILES);
        Scanner::new(&store, &src, sid).scan().unwrap();

        let mut e: Vec<FileEntry> = FILES[..6]
            .iter()
            .map(|p| entry(p, 1000, "2026-01-01T00:00:00Z"))
            .collect();
        e[0] = entry(FILES[0], 2000, "2026-02-01T00:00:00Z"); // modifié
        e.push(entry("Films/Spring (2019).mp4", 5, "2026-02-01T00:00:00Z")); // ajouté
        src.set_entries(e); // le 2e morceau a disparu
        let r = Scanner::new(&store, &src, sid).scan().unwrap();
        assert_eq!((r.added, r.updated, r.missing, r.unchanged), (1, 1, 1, 5));

        let tracks = store
            .list_media(
                &MediaQuery {
                    kind: Some(MediaKind::Track),
                    limit: 10,
                    ..Default::default()
                },
                Visibility::All,
            )
            .unwrap();
        let deux = tracks.items.iter().find(|m| m.title == "Deux").unwrap();
        assert_eq!(deux.status, MediaStatus::Missing, "gardé, marqué disparu");
    }

    #[test]
    fn fichier_deplace_garde_son_identifiant() {
        let (store, sid) = setup();
        let src = FakeSource::new(&["Films/Sintel (2010).mkv"]);
        Scanner::new(&store, &src, sid).scan().unwrap();
        let before = store
            .find_by_uri("file:///media/Films/Sintel (2010).mkv")
            .unwrap()
            .unwrap();

        // Déplacé : retrouvé par son empreinte dès le premier scan.
        src.set(&["Archives/Sintel (2010).mkv"]);
        let r = Scanner::new(&store, &src, sid).scan().unwrap();
        assert_eq!((r.added, r.moved, r.missing), (0, 1, 0));
        let after = store
            .find_by_uri("file:///media/Archives/Sintel (2010).mkv")
            .unwrap()
            .unwrap();
        assert_eq!(
            after.id, before.id,
            "même ligne : favoris et progression gardés"
        );
        assert_eq!(after.status, MediaStatus::Available);
        assert_eq!(count(&store, MediaKind::Movie), 1);
    }

    #[test]
    fn source_injoignable_ne_marque_rien_disparu() {
        let (store, sid) = setup();
        let src = FakeSource::new(FILES);
        Scanner::new(&store, &src, sid).scan().unwrap();
        src.fail(SourceError::Unreachable("NAS éteint".into()));
        let r = Scanner::new(&store, &src, sid).scan().unwrap();
        assert!(r.source_error.is_some());
        assert_eq!(r.missing, 0);
        assert_eq!(
            store.get_source(sid).unwrap().unwrap().status,
            SourceStatus::Offline
        );
        let movies = store
            .list_media(
                &MediaQuery {
                    kind: Some(MediaKind::Movie),
                    limit: 10,
                    ..Default::default()
                },
                Visibility::All,
            )
            .unwrap();
        assert!(movies
            .items
            .iter()
            .all(|m| m.status == MediaStatus::Available));
    }

    #[test]
    fn scan_cible_d_un_dossier() {
        let (store, sid) = setup();
        let src = FakeSource::new(FILES);
        Scanner::new(&store, &src, sid).scan().unwrap();
        // On retire un fichier hors du dossier ciblé : il n'est pas touché.
        src.set(&FILES[1..]);
        let r = Scanner::new(&store, &src, sid)
            .scan_dir("file:///media/Séries/Série Démo/Saison 1")
            .unwrap();
        assert_eq!((r.added, r.missing, r.unchanged), (0, 0, 2));
        let sintel = store
            .find_by_uri("file:///media/Films/Sintel (2010).mkv")
            .unwrap()
            .unwrap();
        assert_eq!(sintel.status, MediaStatus::Available);
    }
}
