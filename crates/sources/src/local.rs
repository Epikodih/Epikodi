//! Dossier local de la machine (`file:///...`).

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use epikodi_domain::ports::{FileSource, SourceError};
use epikodi_domain::source::{media_extension, AudioTags, FileEntry};

use crate::degraded::classify_io;

/// Profondeur maximale parcourue (évite les boucles de liens).
const MAX_DEPTH: usize = 12;
/// Octets lus pour l'empreinte d'un fichier.
const FINGERPRINT_BYTES: usize = 64 * 1024;

/// Parcourt un dossier. `root_uri` est le préfixe des URI produites : il
/// permet de réutiliser ce code pour un partage monté (SMB).
pub struct LocalSource {
    root: PathBuf,
    root_uri: String,
}

/// `file:///home/x` → `/home/x` ; `file:///C:/x` → `C:/x`.
pub fn path_from_file_uri(uri: &str) -> Option<PathBuf> {
    let rest = uri
        .get(7..)
        .filter(|_| uri[..7].eq_ignore_ascii_case("file://"))?;
    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
    let bytes = rest.as_bytes();
    // « /C:/... » sous Windows
    if bytes.len() >= 3 && bytes[0] == b'/' && bytes[2] == b':' {
        return Some(PathBuf::from(&rest[1..]));
    }
    (!rest.is_empty()).then(|| PathBuf::from(rest))
}

/// Chemin → `file:///...` (séparateurs `/`).
pub fn file_uri(path: &Path) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    if s.starts_with('/') {
        format!("file://{s}")
    } else {
        format!("file:///{s}")
    }
}

impl LocalSource {
    pub fn from_uri(uri: &str) -> Result<Self, SourceError> {
        let root = path_from_file_uri(uri).ok_or_else(|| SourceError::NotFound(uri.to_string()))?;
        Ok(Self::new(root, uri.trim_end_matches('/').to_string()))
    }

    pub fn new(root: PathBuf, root_uri: String) -> Self {
        Self {
            root,
            root_uri: root_uri.trim_end_matches('/').to_string(),
        }
    }

    fn uri_for(&self, rel: &str) -> String {
        if rel.is_empty() {
            self.root_uri.clone()
        } else {
            format!("{}/{rel}", self.root_uri)
        }
    }

    /// Chemin réel d'une entrée (refuse de sortir de la racine).
    fn path_of(&self, entry: &FileEntry) -> Option<PathBuf> {
        let rel = Path::new(&entry.relative_path);
        rel.components()
            .all(|c| matches!(c, std::path::Component::Normal(_)))
            .then(|| self.root.join(rel))
    }

    fn walk(
        &self,
        dir: &Path,
        rel: &str,
        depth: usize,
        out: &mut Vec<FileEntry>,
    ) -> Result<(), SourceError> {
        let read = match fs::read_dir(dir) {
            Ok(r) => r,
            // un sous-dossier illisible n'arrête pas le scan
            Err(e) if depth > 0 => {
                tracing::warn!(dir = %dir.display(), "dossier ignoré : {e}");
                return Ok(());
            }
            Err(e) => return Err(classify_io(&e, dir)),
        };
        for item in read.flatten() {
            let name = item.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let child_rel = if rel.is_empty() {
                name.clone()
            } else {
                format!("{rel}/{name}")
            };
            let Ok(meta) = item.metadata() else { continue };
            if meta.is_dir() {
                if depth < MAX_DEPTH {
                    self.walk(&item.path(), &child_rel, depth + 1, out)?;
                }
            } else if meta.is_file() && media_extension(&name).is_some() {
                out.push(FileEntry {
                    uri: self.uri_for(&child_rel),
                    relative_path: child_rel,
                    parent_dir: self.uri_for(rel),
                    size_bytes: meta.len(),
                    modified_at: meta.modified().ok().map(|t| {
                        chrono::DateTime::<chrono::Utc>::from(t)
                            .format("%Y-%m-%dT%H:%M:%SZ")
                            .to_string()
                    }),
                });
            }
        }
        Ok(())
    }
}

impl FileSource for LocalSource {
    fn list(&self) -> Result<Vec<FileEntry>, SourceError> {
        if !self.root.is_dir() {
            return Err(SourceError::NotFound(self.root.display().to_string()));
        }
        let mut out = Vec::new();
        self.walk(&self.root, "", 0, &mut out)?;
        out.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        Ok(out)
    }

    fn list_dir(&self, dir_uri: &str) -> Result<Vec<FileEntry>, SourceError> {
        let rel = dir_uri
            .strip_prefix(&self.root_uri)
            .map(|r| r.trim_start_matches('/').to_string())
            .ok_or_else(|| SourceError::NotFound(dir_uri.to_string()))?;
        let mut out = Vec::new();
        let dir = self.root.join(&rel);
        if !dir.is_dir() {
            return Ok(out); // dossier supprimé : ses fichiers seront « missing »
        }
        self.walk(&dir, &rel, MAX_DEPTH, &mut out)?; // pas de sous-dossiers
        Ok(out)
    }

    fn audio_tags(&self, entry: &FileEntry) -> Option<AudioTags> {
        use lofty::file::{AudioFile, TaggedFileExt};
        use lofty::tag::Accessor;

        let ext = media_extension(&entry.relative_path)?;
        if !epikodi_domain::source::AUDIO_EXTENSIONS.contains(&ext.as_str()) {
            return None;
        }
        let tagged = lofty::read_from_path(self.path_of(entry)?).ok()?;
        let duration = tagged.properties().duration().as_secs();
        let tag = tagged.primary_tag().or_else(|| tagged.first_tag());
        Some(AudioTags {
            title: tag.and_then(|t| t.title().map(|s| s.into_owned())),
            artist: tag.and_then(|t| t.artist().map(|s| s.into_owned())),
            album: tag.and_then(|t| t.album().map(|s| s.into_owned())),
            track_number: tag.and_then(|t| t.track()),
            year: tag.and_then(|t| t.year()).map(|y| y as i32),
            duration_s: (duration > 0).then_some(duration as i64),
        })
    }

    /// Taille + SHA-256 des 64 premiers Kio : retrouve un fichier déplacé ou
    /// renommé sans tout relire.
    fn fingerprint(&self, entry: &FileEntry) -> Option<String> {
        let mut f = fs::File::open(self.path_of(entry)?).ok()?;
        let mut buf = vec![0u8; FINGERPRINT_BYTES];
        let mut n = 0;
        while n < buf.len() {
            match f.read(&mut buf[n..]) {
                Ok(0) => break,
                Ok(k) => n += k,
                Err(_) => return None,
            }
        }
        let hash = Sha256::digest(&buf[..n]);
        let hex: String = hash.iter().take(12).map(|b| format!("{b:02x}")).collect();
        Some(format!("{}-{hex}", entry.size_bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        let p = d.path();
        fs::create_dir_all(p.join("Films")).unwrap();
        fs::create_dir_all(p.join("Séries/Série Démo/Saison 1")).unwrap();
        fs::create_dir_all(p.join(".cache")).unwrap();
        fs::write(p.join("Films/Sintel (2010).mkv"), b"video-sintel").unwrap();
        fs::write(p.join("Films/Sintel (2010).srt"), b"1").unwrap();
        fs::write(p.join("Séries/Série Démo/Saison 1/S01E01.mkv"), b"ep1").unwrap();
        fs::write(p.join(".cache/x.mkv"), b"cache").unwrap();
        d
    }

    #[test]
    fn liste_les_medias_seulement() {
        let d = tree();
        let src = LocalSource::from_uri(&file_uri(d.path())).unwrap();
        let files = src.list().unwrap();
        let rel: Vec<_> = files.iter().map(|f| f.relative_path.as_str()).collect();
        assert_eq!(
            rel,
            [
                "Films/Sintel (2010).mkv",
                "Séries/Série Démo/Saison 1/S01E01.mkv"
            ]
        );
        let f = &files[0];
        assert!(f.uri.starts_with("file://") && f.uri.ends_with("Films/Sintel (2010).mkv"));
        assert!(f.parent_dir.ends_with("/Films"));
        assert_eq!(f.size_bytes, 12);
        assert!(f.modified_at.as_deref().unwrap().ends_with('Z'));
    }

    #[test]
    fn scan_cible_d_un_dossier() {
        let d = tree();
        let src = LocalSource::from_uri(&file_uri(d.path())).unwrap();
        let films = format!("{}/Films", file_uri(d.path()));
        assert_eq!(src.list_dir(&films).unwrap().len(), 1);
        let gone = format!("{}/Supprimé", file_uri(d.path()));
        assert!(src.list_dir(&gone).unwrap().is_empty());
    }

    #[test]
    fn dossier_racine_absent() {
        let src = LocalSource::from_uri("file:///n/existe/pas").unwrap();
        assert!(matches!(src.list(), Err(SourceError::NotFound(_))));
    }

    #[test]
    fn empreinte_suit_le_contenu_pas_le_nom() {
        let d = tree();
        let src = LocalSource::from_uri(&file_uri(d.path())).unwrap();
        let files = src.list().unwrap();
        let fp = src.fingerprint(&files[0]).unwrap();
        fs::rename(
            d.path().join("Films/Sintel (2010).mkv"),
            d.path().join("Films/renomme.mkv"),
        )
        .unwrap();
        let moved = src
            .list()
            .unwrap()
            .into_iter()
            .find(|f| f.relative_path.ends_with("renomme.mkv"))
            .unwrap();
        assert_eq!(src.fingerprint(&moved).unwrap(), fp);
        assert_ne!(src.fingerprint(&files[1]), Some(fp));
    }

    #[test]
    fn uri_fichier() {
        assert_eq!(
            path_from_file_uri("file:///home/x").unwrap(),
            PathBuf::from("/home/x")
        );
        assert_eq!(
            path_from_file_uri("file:///C:/Films").unwrap(),
            PathBuf::from("C:/Films")
        );
        assert!(path_from_file_uri("smb://nas/x").is_none());
        assert_eq!(file_uri(Path::new("/a/b")), "file:///a/b");
    }
}
