//! Mode dégradé : partage lent, temporairement inaccessible ou supprimé.
//!
//! La médiathèque reste consultable : un listage qui échoue ou qui dépasse
//! le délai renvoie une erreur typée, le scan ne marque alors rien comme
//! disparu et la source passe `offline`.

use std::io;
use std::path::Path;
use std::sync::{mpsc, Arc};
use std::time::Duration;

use epikodi_domain::ports::{FileSource, SourceError};
use epikodi_domain::source::{AudioTags, FileEntry};

/// Erreur d'entrée/sortie → erreur de source compréhensible.
pub fn classify_io(e: &io::Error, path: &Path) -> SourceError {
    let p = path.display().to_string();
    match e.kind() {
        io::ErrorKind::NotFound => SourceError::NotFound(p),
        io::ErrorKind::PermissionDenied => SourceError::PermissionDenied(p),
        io::ErrorKind::TimedOut => SourceError::Timeout,
        _ => SourceError::Unreachable(format!("{p} : {e}")),
    }
}

/// Ajoute un délai maximal au listage d'une source (le travail continue en
/// arrière-plan, mais le scan n'attend pas indéfiniment un NAS qui rame).
pub struct TimeoutSource {
    inner: Arc<dyn FileSource>,
    timeout: Duration,
}

impl TimeoutSource {
    pub fn new(inner: Box<dyn FileSource>, timeout: Duration) -> Self {
        Self {
            inner: Arc::from(inner),
            timeout,
        }
    }

    fn with_timeout<T: Send + 'static>(
        &self,
        f: impl FnOnce(&dyn FileSource) -> Result<T, SourceError> + Send + 'static,
    ) -> Result<T, SourceError> {
        let (tx, rx) = mpsc::channel();
        let inner = Arc::clone(&self.inner);
        std::thread::spawn(move || {
            let _ = tx.send(f(inner.as_ref()));
        });
        rx.recv_timeout(self.timeout)
            .unwrap_or(Err(SourceError::Timeout))
    }
}

impl FileSource for TimeoutSource {
    fn list(&self) -> Result<Vec<FileEntry>, SourceError> {
        self.with_timeout(|s| s.list())
    }

    fn list_dir(&self, dir_uri: &str) -> Result<Vec<FileEntry>, SourceError> {
        let dir = dir_uri.to_string();
        self.with_timeout(move |s| s.list_dir(&dir))
    }

    fn audio_tags(&self, entry: &FileEntry) -> Option<AudioTags> {
        self.inner.audio_tags(entry)
    }

    fn fingerprint(&self, entry: &FileEntry) -> Option<String> {
        self.inner.fingerprint(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Slow;
    impl FileSource for Slow {
        fn list(&self) -> Result<Vec<FileEntry>, SourceError> {
            std::thread::sleep(Duration::from_millis(500));
            Ok(vec![])
        }
    }

    struct Fast;
    impl FileSource for Fast {
        fn list(&self) -> Result<Vec<FileEntry>, SourceError> {
            Ok(vec![FileEntry::default()])
        }
    }

    #[test]
    fn partage_lent_coupe_au_delai() {
        let s = TimeoutSource::new(Box::new(Slow), Duration::from_millis(50));
        assert_eq!(s.list(), Err(SourceError::Timeout));
    }

    #[test]
    fn partage_rapide_inchange() {
        let s = TimeoutSource::new(Box::new(Fast), Duration::from_secs(5));
        assert_eq!(s.list().unwrap().len(), 1);
    }

    #[test]
    fn erreurs_classees() {
        let p = Path::new("/x");
        assert!(matches!(
            classify_io(&io::Error::from(io::ErrorKind::NotFound), p),
            SourceError::NotFound(_)
        ));
        assert!(matches!(
            classify_io(&io::Error::from(io::ErrorKind::PermissionDenied), p),
            SourceError::PermissionDenied(_)
        ));
        assert!(matches!(
            classify_io(&io::Error::from(io::ErrorKind::ConnectionRefused), p),
            SourceError::Unreachable(_)
        ));
    }
}
