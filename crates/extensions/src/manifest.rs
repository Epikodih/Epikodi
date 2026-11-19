//! Lecture du manifeste d'une extension.
//!
//! ```toml
//! id = "weather.widget"
//! name = "Météo"
//! version = "1.0.0"
//! author = "Équipe"
//! description = "La météo sur l'écran d'accueil"
//! type = "widget"                     # source | metadata | widget
//! permissions = ["network", "ui.home"]
//! entry = "extension.wasm"
//!
//! [settings]
//! city = "Paris"
//! ```

use std::fs;
use std::path::Path;

use epikodi_domain::extension::Manifest;

use crate::ExtensionError;

pub const MANIFEST_TOML: &str = "manifest.toml";
pub const MANIFEST_JSON: &str = "manifest.json";

/// Lit et valide le manifeste d'un dossier d'extension.
pub fn load_manifest(dir: &Path) -> Result<Manifest, ExtensionError> {
    let manifest = if let Ok(text) = fs::read_to_string(dir.join(MANIFEST_TOML)) {
        toml::from_str::<Manifest>(&text)
            .map_err(|e| ExtensionError::InvalidManifest(e.to_string()))?
    } else if let Ok(text) = fs::read_to_string(dir.join(MANIFEST_JSON)) {
        serde_json::from_str::<Manifest>(&text)
            .map_err(|e| ExtensionError::InvalidManifest(e.to_string()))?
    } else {
        return Err(ExtensionError::InvalidManifest(format!(
            "{MANIFEST_TOML} ou {MANIFEST_JSON} absent de {}",
            dir.display()
        )));
    };
    manifest
        .validate()
        .map_err(|e| ExtensionError::InvalidManifest(e.to_string()))?;
    if !dir.join(&manifest.entry).is_file() {
        return Err(ExtensionError::InvalidManifest(format!(
            "fichier de code « {} » absent",
            manifest.entry
        )));
    }
    Ok(manifest)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn write_extension(dir: &Path, id: &str, version: &str, permissions: &[&str]) {
        fs::create_dir_all(dir).unwrap();
        let perms = permissions
            .iter()
            .map(|p| format!("\"{p}\""))
            .collect::<Vec<_>>()
            .join(", ");
        fs::write(
            dir.join(MANIFEST_TOML),
            format!(
                "id = \"{id}\"\nname = \"Météo\"\nversion = \"{version}\"\nauthor = \"Équipe\"\n\
                 type = \"widget\"\npermissions = [{perms}]\n\n[settings]\ncity = \"Paris\"\nunits = 1\n"
            ),
        )
        .unwrap();
        fs::write(dir.join("extension.wasm"), b"\0asm\x01\0\0\0").unwrap();
    }

    #[test]
    fn manifeste_toml() {
        let d = tempfile::tempdir().unwrap();
        write_extension(d.path(), "weather", "1.0.0", &["network", "ui.home"]);
        let m = load_manifest(d.path()).unwrap();
        assert_eq!(m.id, "weather");
        assert_eq!(m.permissions.len(), 2);
        assert_eq!(m.settings["city"], "Paris");
    }

    #[test]
    fn manifeste_absent_ou_invalide() {
        let d = tempfile::tempdir().unwrap();
        assert!(load_manifest(d.path()).is_err());
        fs::write(d.path().join(MANIFEST_JSON), r#"{"id":"x"}"#).unwrap();
        assert!(load_manifest(d.path()).is_err());
    }

    #[test]
    fn permission_inconnue_refusee() {
        let d = tempfile::tempdir().unwrap();
        write_extension(d.path(), "weather", "1.0.0", &["filesystem.all"]);
        assert!(load_manifest(d.path()).is_err());
    }

    #[test]
    fn code_absent() {
        let d = tempfile::tempdir().unwrap();
        write_extension(d.path(), "weather", "1.0.0", &[]);
        fs::remove_file(d.path().join("extension.wasm")).unwrap();
        assert!(load_manifest(d.path()).is_err());
    }
}
