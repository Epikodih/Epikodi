//! Cycle de vie : installer (depuis un dossier, un `.zip` ou le catalogue
//! local), configurer, mettre à jour, désactiver, désinstaller. Le tout sans
//! redémarrer ni recompiler le produit.

use std::fs;
use std::io::{self, Read as _};
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use epikodi_domain::extension::{
    parse_version, Extension, ExtensionStatus, Grants, Manifest, Permission,
};
use epikodi_domain::ports::ExtensionRepo;

use crate::manifest::load_manifest;
use crate::ExtensionError;

/// Taille maximale d'un paquet décompressé.
const MAX_UNPACKED_BYTES: u64 = 200 * 1024 * 1024;
const MAX_FILES: usize = 2000;

pub struct ExtensionManager<'a, R: ExtensionRepo + ?Sized> {
    repo: &'a R,
    /// Dossier où les extensions sont installées (`<data>/extensions`).
    install_dir: PathBuf,
}

/// Un paquet prêt à être installé (dossier temporaire nettoyé à la fin).
pub struct Package {
    pub manifest: Manifest,
    pub dir: PathBuf,
    temp: Option<PathBuf>,
}

impl Drop for Package {
    fn drop(&mut self) {
        if let Some(t) = &self.temp {
            let _ = fs::remove_dir_all(t);
        }
    }
}

impl<'a, R: ExtensionRepo + ?Sized> ExtensionManager<'a, R> {
    pub fn new(repo: &'a R, install_dir: impl Into<PathBuf>) -> Self {
        Self {
            repo,
            install_dir: install_dir.into(),
        }
    }

    /// Lit un paquet (dossier ou `.zip`) sans l'installer : sert à montrer
    /// les permissions demandées avant d'accepter.
    pub fn inspect(&self, path: &Path) -> Result<Package, ExtensionError> {
        if path.is_dir() {
            return Ok(Package {
                manifest: load_manifest(path)?,
                dir: path.to_path_buf(),
                temp: None,
            });
        }
        let is_zip = path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("zip"));
        if !path.is_file() || !is_zip {
            return Err(ExtensionError::InvalidPackage(format!(
                "{} : dossier ou fichier .zip attendu",
                path.display()
            )));
        }
        let staging = self.install_dir.join(".staging").join(unique_name());
        fs::create_dir_all(&staging)?;
        // Supprimé si quelque chose échoue avant la fin.
        let guard = Cleanup(Some(staging.clone()));
        unzip(path, &staging)?;
        // Un zip qui contient un seul dossier racine : on descend dedans.
        let root = single_subdir(&staging).unwrap_or_else(|| staging.clone());
        let manifest = load_manifest(&root)?;
        Ok(Package {
            manifest,
            dir: root,
            temp: guard.disarm(),
        })
    }

    /// Installe une extension en accordant `accepted` parmi ses permissions.
    pub fn install(
        &self,
        path: &Path,
        accepted: &[Permission],
    ) -> Result<Extension, ExtensionError> {
        let pkg = self.inspect(path)?;
        if self.repo.get_extension(&pkg.manifest.id)?.is_some() {
            return Err(ExtensionError::AlreadyInstalled(pkg.manifest.id.clone()));
        }
        let grants = Grants::from_acceptance(&pkg.manifest.permissions, accepted)?;
        let staged = self.copy_in(&pkg)?;
        let ext = Extension {
            id: pkg.manifest.id.clone(),
            name: pkg.manifest.name.clone(),
            version: pkg.manifest.version.clone(),
            kind: pkg.manifest.kind,
            install_path: staged.target.to_string_lossy().into_owned(),
            enabled: true,
            status: ExtensionStatus::Ok,
            last_error: None,
            permissions: grants,
            settings: pkg.manifest.settings.clone(),
        };
        // Base d'abord ; si elle échoue, les fichiers copiés sont retirés.
        self.repo.save_extension(&ext)?;
        staged.commit();
        tracing::info!(id = %ext.id, version = %ext.version, "extension installée");
        Ok(ext)
    }

    /// Met à jour vers une version plus récente. Les réglages existants sont
    /// gardés ; les permissions sont celles acceptées pour la nouvelle version.
    pub fn update(
        &self,
        id: &str,
        path: &Path,
        accepted: &[Permission],
    ) -> Result<Extension, ExtensionError> {
        let current = self
            .repo
            .get_extension(id)?
            .ok_or_else(|| ExtensionError::NotFound(id.into()))?;
        let pkg = self.inspect(path)?;
        if pkg.manifest.id != id {
            return Err(ExtensionError::InvalidPackage(format!(
                "le paquet contient « {} », pas « {id} »",
                pkg.manifest.id
            )));
        }
        if parse_version(&pkg.manifest.version) <= parse_version(&current.version) {
            return Err(ExtensionError::NotNewer {
                current: current.version,
                new: pkg.manifest.version.clone(),
            });
        }
        let grants = Grants::from_acceptance(&pkg.manifest.permissions, accepted)?;
        let staged = self.copy_in(&pkg)?;
        // Réglages : défauts de la nouvelle version, valeurs de l'utilisateur
        // gardées pour les clés qui existent toujours.
        let mut settings = pkg.manifest.settings.clone();
        for (k, v) in current.settings {
            if settings.contains_key(&k) {
                settings.insert(k, v);
            }
        }
        let ext = Extension {
            name: pkg.manifest.name.clone(),
            version: pkg.manifest.version.clone(),
            kind: pkg.manifest.kind,
            install_path: staged.target.to_string_lossy().into_owned(),
            status: ExtensionStatus::Ok,
            last_error: None,
            permissions: grants,
            settings,
            ..current
        };
        // Si la base échoue, l'ancienne version est remise en place.
        self.repo.save_extension(&ext)?;
        staged.commit();
        tracing::info!(id, version = %ext.version, "extension mise à jour");
        Ok(ext)
    }

    pub fn uninstall(&self, id: &str) -> Result<(), ExtensionError> {
        let ext = self
            .repo
            .get_extension(id)?
            .ok_or_else(|| ExtensionError::NotFound(id.into()))?;
        // La base d'abord : si elle échoue, l'extension reste installée et
        // complète. Des fichiers restants après coup sont seulement signalés.
        self.repo.delete_extension(id)?;
        let path = PathBuf::from(&ext.install_path);
        // On ne supprime que ce qui est dans notre dossier d'installation.
        if path.starts_with(&self.install_dir) && path.is_dir() {
            if let Err(e) = fs::remove_dir_all(&path) {
                tracing::warn!(path = %path.display(), "fichiers de l'extension non supprimés : {e}");
            }
        }
        tracing::info!(id, "extension désinstallée");
        Ok(())
    }

    /// Activer remet aussi à zéro une extension arrêtée après un plantage.
    pub fn set_enabled(&self, id: &str, enabled: bool) -> Result<Extension, ExtensionError> {
        let mut ext = self
            .repo
            .get_extension(id)?
            .ok_or_else(|| ExtensionError::NotFound(id.into()))?;
        ext.enabled = enabled;
        ext.status = if enabled {
            ExtensionStatus::Ok
        } else {
            ExtensionStatus::Stopped
        };
        if enabled {
            ext.last_error = None;
        }
        self.repo.save_extension(&ext)?;
        Ok(ext)
    }

    /// Change des réglages. Seules les clés déclarées dans le manifeste
    /// sont acceptées, avec le même type JSON que la valeur par défaut.
    pub fn set_settings(
        &self,
        id: &str,
        changes: &Map<String, Value>,
    ) -> Result<Extension, ExtensionError> {
        let mut ext = self
            .repo
            .get_extension(id)?
            .ok_or_else(|| ExtensionError::NotFound(id.into()))?;
        let declared = load_manifest(Path::new(&ext.install_path))
            .map(|m| m.settings)
            .unwrap_or_else(|_| ext.settings.clone());
        for (k, v) in changes {
            let Some(default) = declared.get(k) else {
                return Err(ExtensionError::InvalidSetting(format!(
                    "« {k} » n'existe pas"
                )));
            };
            if std::mem::discriminant(default) != std::mem::discriminant(v) && !default.is_null() {
                return Err(ExtensionError::InvalidSetting(format!(
                    "« {k} » : mauvais type"
                )));
            }
            ext.settings.insert(k.clone(), v.clone());
        }
        self.repo.save_extension(&ext)?;
        Ok(ext)
    }

    /// Paquets disponibles dans un catalogue local (un dossier contenant des
    /// dossiers d'extensions ou des `.zip`). Les paquets invalides sont ignorés.
    pub fn catalog(&self, catalog_dir: &Path) -> Vec<(PathBuf, Manifest)> {
        let Ok(read) = fs::read_dir(catalog_dir) else {
            return Vec::new();
        };
        let mut out: Vec<(PathBuf, Manifest)> = read
            .flatten()
            .map(|e| e.path())
            .filter_map(|p| match self.inspect(&p) {
                Ok(pkg) => Some((p, pkg.manifest.clone())),
                Err(e) => {
                    tracing::debug!(path = %p.display(), "paquet ignoré : {e}");
                    None
                }
            })
            .collect();
        out.sort_by(|a, b| a.1.name.cmp(&b.1.name));
        out
    }

    /// Copie le paquet dans `<install_dir>/<id>`. L'ancienne version est
    /// mise de côté, et supprimée seulement par [`Staged::commit`] ; sans
    /// `commit` (erreur, panique), tout est remis comme avant.
    fn copy_in(&self, pkg: &Package) -> Result<Staged, ExtensionError> {
        fs::create_dir_all(&self.install_dir)?;
        let target = self.install_dir.join(&pkg.manifest.id);
        let suffix = unique_name();
        let tmp = self
            .install_dir
            .join(format!(".{}-new-{suffix}", pkg.manifest.id));
        let mut staged = Staged {
            target: target.clone(),
            tmp: Some(tmp.clone()),
            backup: None,
            swapped: false,
            committed: false,
        };
        copy_dir(&pkg.dir, &tmp)?;
        if target.exists() {
            let backup = self
                .install_dir
                .join(format!(".{}-old-{suffix}", pkg.manifest.id));
            fs::rename(&target, &backup)?;
            staged.backup = Some(backup);
        }
        fs::rename(&tmp, &target)?;
        staged.tmp = None;
        staged.swapped = true;
        Ok(staged)
    }
}

/// Une version copiée en place, en attente de validation.
struct Staged {
    target: PathBuf,
    tmp: Option<PathBuf>,
    backup: Option<PathBuf>,
    swapped: bool,
    committed: bool,
}

impl Staged {
    /// Valide : l'ancienne version est supprimée.
    fn commit(mut self) {
        self.committed = true;
        if let Some(b) = self.backup.take() {
            if let Err(e) = fs::remove_dir_all(&b) {
                tracing::warn!(path = %b.display(), "ancienne version non supprimée : {e}");
            }
        }
    }
}

impl Drop for Staged {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        if let Some(t) = &self.tmp {
            let _ = fs::remove_dir_all(t);
        }
        if self.swapped {
            let _ = fs::remove_dir_all(&self.target);
        }
        if let Some(b) = &self.backup {
            if let Err(e) = fs::rename(b, &self.target) {
                tracing::error!(path = %b.display(), "ancienne version non restaurée : {e}");
            }
        }
    }
}

/// Supprime un dossier temporaire à la sortie, sauf si on le garde.
struct Cleanup(Option<PathBuf>);

impl Cleanup {
    fn disarm(mut self) -> Option<PathBuf> {
        self.0.take()
    }
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        if let Some(p) = &self.0 {
            let _ = fs::remove_dir_all(p);
        }
    }
}

fn unique_name() -> String {
    format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    )
}

fn single_subdir(dir: &Path) -> Option<PathBuf> {
    let entries: Vec<_> = fs::read_dir(dir).ok()?.flatten().collect();
    match entries.as_slice() {
        [only] if only.path().is_dir() => Some(only.path()),
        _ => None,
    }
}

fn copy_dir(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest = to.join(entry.file_name());
        if ty.is_dir() {
            copy_dir(&entry.path(), &dest)?;
        } else if ty.is_file() {
            fs::copy(entry.path(), dest)?;
        } // les liens symboliques sont ignorés
    }
    Ok(())
}

/// Décompresse en refusant les chemins qui sortent du dossier (zip-slip) et
/// les paquets démesurés (bombe zip).
fn unzip(path: &Path, dest: &Path) -> Result<(), ExtensionError> {
    let file = fs::File::open(path)?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| ExtensionError::InvalidPackage(e.to_string()))?;
    if archive.len() > MAX_FILES {
        return Err(ExtensionError::InvalidPackage("trop de fichiers".into()));
    }
    let mut total: u64 = 0;
    for i in 0..archive.len() {
        let mut f = archive
            .by_index(i)
            .map_err(|e| ExtensionError::InvalidPackage(e.to_string()))?;
        let Some(rel) = f.enclosed_name() else {
            return Err(ExtensionError::InvalidPackage(format!(
                "chemin interdit : {}",
                f.name()
            )));
        };
        let out = dest.join(rel);
        if f.is_dir() {
            fs::create_dir_all(&out)?;
            continue;
        }
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut w = fs::File::create(&out)?;
        let copied = io::copy(&mut (&mut f).take(MAX_UNPACKED_BYTES - total + 1), &mut w)?;
        total += copied;
        if total > MAX_UNPACKED_BYTES {
            return Err(ExtensionError::InvalidPackage(
                "paquet trop volumineux".into(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::tests::write_extension;
    use epikodi_storage::Store;
    use std::io::Write;

    fn zip_extension(src: &Path, zip_path: &Path, prefix: &str) {
        let f = fs::File::create(zip_path).unwrap();
        let mut z = zip::ZipWriter::new(f);
        let opts = zip::write::SimpleFileOptions::default();
        for entry in fs::read_dir(src).unwrap().flatten() {
            z.start_file(
                format!("{prefix}{}", entry.file_name().to_string_lossy()),
                opts,
            )
            .unwrap();
            z.write_all(&fs::read(entry.path()).unwrap()).unwrap();
        }
        z.finish().unwrap();
    }

    #[test]
    fn installer_depuis_un_dossier() {
        let store = Store::in_memory();
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("src");
        write_extension(&src, "weather", "1.0.0", &["network", "ui.home"]);
        let mgr = ExtensionManager::new(&store, d.path().join("installed"));

        let ext = mgr.install(&src, &[Permission::Network]).unwrap();
        assert!(ext.enabled);
        assert!(ext.permissions.allows(Permission::Network));
        assert!(
            !ext.permissions.allows(Permission::UiHome),
            "non acceptée = refusée"
        );
        assert!(Path::new(&ext.install_path).join("manifest.toml").is_file());
        assert!(matches!(
            mgr.install(&src, &[]),
            Err(ExtensionError::AlreadyInstalled(_))
        ));
    }

    #[test]
    fn installer_depuis_un_zip() {
        let store = Store::in_memory();
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("src");
        write_extension(&src, "weather", "1.0.0", &[]);
        let zip_path = d.path().join("weather.zip");
        zip_extension(&src, &zip_path, "weather/");
        let mgr = ExtensionManager::new(&store, d.path().join("installed"));
        let ext = mgr.install(&zip_path, &[]).unwrap();
        assert_eq!(ext.id, "weather");
        assert!(
            !d.path()
                .join("installed/.staging")
                .read_dir()
                .unwrap()
                .any(|_| true),
            "dossier temporaire nettoyé"
        );
    }

    #[test]
    fn zip_slip_refuse() {
        let store = Store::in_memory();
        let d = tempfile::tempdir().unwrap();
        let zip_path = d.path().join("evil.zip");
        let mut z = zip::ZipWriter::new(fs::File::create(&zip_path).unwrap());
        z.start_file("../../evil.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        z.write_all(b"x").unwrap();
        z.finish().unwrap();
        let mgr = ExtensionManager::new(&store, d.path().join("installed"));
        assert!(matches!(
            mgr.install(&zip_path, &[]),
            Err(ExtensionError::InvalidPackage(_))
        ));
        assert!(!d.path().join("evil.txt").exists());
    }

    #[test]
    fn mise_a_jour_garde_les_reglages() {
        let store = Store::in_memory();
        let d = tempfile::tempdir().unwrap();
        let v1 = d.path().join("v1");
        let v2 = d.path().join("v2");
        write_extension(&v1, "weather", "1.0.0", &["network"]);
        write_extension(&v2, "weather", "1.1.0", &["network", "storage"]);
        let mgr = ExtensionManager::new(&store, d.path().join("installed"));
        mgr.install(&v1, &[Permission::Network]).unwrap();
        let mut changes = Map::new();
        changes.insert("city".into(), Value::from("Lyon"));
        mgr.set_settings("weather", &changes).unwrap();

        assert!(matches!(
            mgr.update("weather", &v1, &[]),
            Err(ExtensionError::NotNewer { .. })
        ));
        let ext = mgr
            .update("weather", &v2, &[Permission::Network, Permission::Storage])
            .unwrap();
        assert_eq!(ext.version, "1.1.0");
        assert_eq!(ext.settings["city"], "Lyon");
        assert!(ext.permissions.allows(Permission::Storage));
    }

    /// Dépôt qui échoue à l'écriture quand on le demande.
    struct FailingRepo {
        inner: Store,
        fail: std::cell::Cell<bool>,
    }

    impl ExtensionRepo for FailingRepo {
        fn list_extensions(&self) -> epikodi_domain::DomainResult<Vec<Extension>> {
            self.inner.list_extensions()
        }
        fn get_extension(&self, id: &str) -> epikodi_domain::DomainResult<Option<Extension>> {
            self.inner.get_extension(id)
        }
        fn save_extension(&self, e: &Extension) -> epikodi_domain::DomainResult<()> {
            if self.fail.get() {
                return Err(epikodi_domain::DomainError::Internal("disque plein".into()));
            }
            self.inner.save_extension(e)
        }
        fn set_extension_status(
            &self,
            id: &str,
            status: ExtensionStatus,
            error: Option<&str>,
        ) -> epikodi_domain::DomainResult<()> {
            self.inner.set_extension_status(id, status, error)
        }
        fn delete_extension(&self, id: &str) -> epikodi_domain::DomainResult<()> {
            if self.fail.get() {
                return Err(epikodi_domain::DomainError::Internal("disque plein".into()));
            }
            self.inner.delete_extension(id)
        }
    }

    fn installed_version(dir: &Path) -> String {
        crate::manifest::load_manifest(dir).unwrap().version
    }

    /// Régression : la mise à jour supprimait l'ancienne version avant que
    /// la base soit à jour.
    #[test]
    fn echec_en_base_garde_l_ancienne_version() {
        let repo = FailingRepo {
            inner: Store::in_memory(),
            fail: std::cell::Cell::new(false),
        };
        let d = tempfile::tempdir().unwrap();
        let (v1, v2) = (d.path().join("v1"), d.path().join("v2"));
        write_extension(&v1, "weather", "1.0.0", &[]);
        write_extension(&v2, "weather", "2.0.0", &[]);
        let installed = d.path().join("installed");
        let mgr = ExtensionManager::new(&repo, &installed);
        let ext = mgr.install(&v1, &[]).unwrap();

        repo.fail.set(true);
        assert!(mgr.update("weather", &v2, &[]).is_err());
        let dir = Path::new(&ext.install_path);
        assert_eq!(installed_version(dir), "1.0.0", "fichiers restaurés");
        assert_eq!(
            repo.get_extension("weather").unwrap().unwrap().version,
            "1.0.0"
        );
        let leftovers: Vec<_> = fs::read_dir(&installed)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n != "weather" && n != ".staging")
            .collect();
        assert!(leftovers.is_empty(), "rien ne traîne : {leftovers:?}");

        // désinstallation qui échoue en base : les fichiers restent
        assert!(mgr.uninstall("weather").is_err());
        assert_eq!(installed_version(dir), "1.0.0");

        // installation qui échoue en base : aucun fichier laissé
        let other = d.path().join("other");
        write_extension(&other, "other.ext", "1.0.0", &[]);
        assert!(mgr.install(&other, &[]).is_err());
        assert!(!installed.join("other.ext").exists());

        repo.fail.set(false);
        assert_eq!(mgr.update("weather", &v2, &[]).unwrap().version, "2.0.0");
        assert_eq!(installed_version(dir), "2.0.0");
    }

    #[test]
    fn reglages_controles() {
        let store = Store::in_memory();
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("src");
        write_extension(&src, "weather", "1.0.0", &[]);
        let mgr = ExtensionManager::new(&store, d.path().join("installed"));
        mgr.install(&src, &[]).unwrap();
        let mut bad = Map::new();
        bad.insert("inconnu".into(), Value::from(1));
        assert!(mgr.set_settings("weather", &bad).is_err());
        let mut bad = Map::new();
        bad.insert("units".into(), Value::from("texte"));
        assert!(mgr.set_settings("weather", &bad).is_err(), "mauvais type");
    }

    #[test]
    fn desactiver_puis_desinstaller() {
        let store = Store::in_memory();
        let d = tempfile::tempdir().unwrap();
        let src = d.path().join("src");
        write_extension(&src, "weather", "1.0.0", &[]);
        let mgr = ExtensionManager::new(&store, d.path().join("installed"));
        let ext = mgr.install(&src, &[]).unwrap();
        let off = mgr.set_enabled("weather", false).unwrap();
        assert!(!off.enabled);
        assert_eq!(off.status, ExtensionStatus::Stopped);
        mgr.uninstall("weather").unwrap();
        assert!(!Path::new(&ext.install_path).exists());
        assert!(store.get_extension("weather").unwrap().is_none());
        assert!(matches!(
            mgr.uninstall("weather"),
            Err(ExtensionError::NotFound(_))
        ));
    }

    #[test]
    fn catalogue_local() {
        let store = Store::in_memory();
        let d = tempfile::tempdir().unwrap();
        let cat = d.path().join("catalog");
        write_extension(&cat.join("a"), "weather", "1.0.0", &[]);
        fs::create_dir_all(cat.join("cassee")).unwrap();
        let mgr = ExtensionManager::new(&store, d.path().join("installed"));
        let list = mgr.catalog(&cat);
        assert_eq!(list.len(), 1, "le paquet invalide est ignoré");
        assert_eq!(list[0].1.id, "weather");
    }
}
