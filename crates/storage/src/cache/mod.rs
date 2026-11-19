//! Cache local des images (affiches, fonds) : des fichiers, pas SQLite.
//! La médiathèque reste consultable hors connexion.

use std::fs;
use std::path::{Path, PathBuf};

use epikodi_domain::ports::ImageCache;
use epikodi_domain::{DomainError, DomainResult};

pub struct FsImageCache {
    root: PathBuf,
}

impl FsImageCache {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Chemin absolu d'une image du cache, ou `None` si le chemin sort du
    /// cache (`..`) ou si le fichier n'existe pas.
    pub fn resolve(&self, relative: &str) -> Option<PathBuf> {
        let rel = Path::new(relative);
        if rel.is_absolute()
            || rel
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return None;
        }
        let path = self.root.join(rel);
        path.is_file().then_some(path)
    }
}

/// Nom de fichier sûr à partir d'une clé (`tmdb-movie-123`).
fn sanitize(key: &str) -> String {
    let s: String = key
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(100)
        .collect();
    if s.is_empty() {
        "image".into()
    } else {
        s
    }
}

fn extension_for(bytes: &[u8]) -> &'static str {
    match bytes {
        [0x89, b'P', b'N', b'G', ..] => "png",
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => "webp",
        _ => "jpg",
    }
}

impl ImageCache for FsImageCache {
    fn store(&self, key: &str, bytes: &[u8]) -> DomainResult<String> {
        let dir = self.root.join("images");
        fs::create_dir_all(&dir).map_err(DomainError::internal)?;
        let name = format!("{}.{}", sanitize(key), extension_for(bytes));
        fs::write(dir.join(&name), bytes).map_err(DomainError::internal)?;
        Ok(format!("images/{name}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stocke_et_retrouve() {
        let dir = tempfile::tempdir().unwrap();
        let cache = FsImageCache::new(dir.path());
        let rel = cache
            .store("tmdb/movie:1", &[0x89, b'P', b'N', b'G', 1, 2])
            .unwrap();
        assert_eq!(rel, "images/tmdb_movie_1.png");
        assert!(cache.resolve(&rel).is_some());
    }

    #[test]
    fn refuse_de_sortir_du_cache() {
        let dir = tempfile::tempdir().unwrap();
        let cache = FsImageCache::new(dir.path());
        assert!(cache.resolve("../secret.key").is_none());
        assert!(cache.resolve("/etc/passwd").is_none());
        assert!(cache.resolve("images/absent.jpg").is_none());
    }
}
