//! Secrets : identifiants des partages (chiffrés), PIN et jetons (hachés).
//!
//! - Identifiants : ChaCha20-Poly1305. La clé est dans un fichier à part
//!   (`secret.key` dans le dossier de données), jamais dans la base.
//! - PIN : argon2 (lent et salé : 4 chiffres = 10 000 combinaisons).
//! - Jetons : SHA-256 (le jeton est long et aléatoire, un hash rapide suffit).

use std::fs;
use std::io::Write;
use std::path::Path;

use argon2::password_hash::{
    rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
};
use argon2::Argon2;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use rand::RngCore;
use sha2::{Digest, Sha256};

use epikodi_domain::source::Credentials;
use epikodi_domain::{DomainError, DomainResult};

const NONCE_LEN: usize = 12;

/// Chiffre et déchiffre les identifiants des sources.
#[derive(Clone)]
pub struct SecretBox {
    cipher: ChaCha20Poly1305,
}

impl std::fmt::Debug for SecretBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretBox(***)")
    }
}

impl SecretBox {
    pub fn from_key(key: [u8; 32]) -> Self {
        Self {
            cipher: ChaCha20Poly1305::new(Key::from_slice(&key)),
        }
    }

    /// Lit la clé, ou la crée (droits 600 sous Unix) si elle n'existe pas.
    pub fn load_or_create(path: &Path) -> std::io::Result<Self> {
        if let Ok(bytes) = fs::read(path) {
            let key: [u8; 32] = bytes.try_into().map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "clé de chiffrement invalide (32 octets attendus)",
                )
            })?;
            return Ok(Self::from_key(key));
        }
        let mut key = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut key);
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options.open(path)?.write_all(&key)?;
        Ok(Self::from_key(key))
    }

    pub fn encrypt(&self, plain: &[u8]) -> DomainResult<Vec<u8>> {
        let mut nonce = [0u8; NONCE_LEN];
        rand::rngs::OsRng.fill_bytes(&mut nonce);
        let mut out = nonce.to_vec();
        out.extend(
            self.cipher
                .encrypt(Nonce::from_slice(&nonce), plain)
                .map_err(|_| DomainError::Internal("chiffrement impossible".into()))?,
        );
        Ok(out)
    }

    pub fn decrypt(&self, blob: &[u8]) -> DomainResult<Vec<u8>> {
        if blob.len() <= NONCE_LEN {
            return Err(DomainError::Internal("secret illisible".into()));
        }
        let (nonce, data) = blob.split_at(NONCE_LEN);
        self.cipher
            .decrypt(Nonce::from_slice(nonce), data)
            .map_err(|_| DomainError::Internal("secret illisible (clé changée ?)".into()))
    }

    pub fn encrypt_credentials(&self, c: &Credentials) -> DomainResult<Vec<u8>> {
        let json = serde_json::to_vec(c).map_err(DomainError::internal)?;
        self.encrypt(&json)
    }

    pub fn decrypt_credentials(&self, blob: &[u8]) -> DomainResult<Credentials> {
        serde_json::from_slice(&self.decrypt(blob)?).map_err(DomainError::internal)
    }
}

/// Hash d'un PIN (argon2, sel aléatoire).
pub fn hash_pin(pin: &str) -> DomainResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(pin.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| DomainError::Internal(format!("hash du PIN : {e}")))
}

pub fn verify_pin(pin: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .map(|h| {
            Argon2::default()
                .verify_password(pin.as_bytes(), &h)
                .is_ok()
        })
        .unwrap_or(false)
}

/// Hash d'un jeton de session (hexadécimal).
pub fn hash_token(token: &str) -> String {
    Sha256::digest(token.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chiffrement_aller_retour() {
        let b = SecretBox::from_key([7; 32]);
        let c = Credentials {
            username: "marc".into(),
            password: "motdepasse".into(),
            domain: Some("WORKGROUP".into()),
        };
        let blob = b.encrypt_credentials(&c).unwrap();
        assert!(
            !String::from_utf8_lossy(&blob).contains("motdepasse"),
            "jamais en clair"
        );
        assert_eq!(b.decrypt_credentials(&blob).unwrap(), c);
        // une autre clé ne déchiffre pas
        assert!(SecretBox::from_key([8; 32])
            .decrypt_credentials(&blob)
            .is_err());
    }

    #[test]
    fn cle_creee_puis_relue() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("secret.key");
        let a = SecretBox::load_or_create(&path).unwrap();
        let blob = a.encrypt(b"x").unwrap();
        let b = SecretBox::load_or_create(&path).unwrap();
        assert_eq!(b.decrypt(&blob).unwrap(), b"x");
    }

    #[test]
    fn pin_hache() {
        let h = hash_pin("1234").unwrap();
        assert!(!h.contains("1234"));
        assert!(verify_pin("1234", &h));
        assert!(!verify_pin("4321", &h));
        assert!(!verify_pin("1234", "pas-un-hash"));
    }

    #[test]
    fn hash_de_jeton_stable() {
        assert_eq!(hash_token("abc"), hash_token("abc"));
        assert_eq!(hash_token("abc").len(), 64);
        assert_ne!(hash_token("abc"), hash_token("abd"));
    }
}
