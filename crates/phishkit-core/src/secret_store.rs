//! At-rest sealing for SMTP passwords and API keys (H1).
//!
//! Secrets are sealed with ChaCha20-Poly1305 under a data-encryption key stored
//! next to the DB at `$PHISHKIT_DATA/master.key` with mode 0600. Ciphertext is
//! prefixed with `enc:v1:` so legacy plaintext rows stay readable and get
//! re-sealed on the next write. The key never leaves the data directory; with
//! the H2 0700/0600 hardening that directory is already unreadable to other
//! local users.

use std::fs;
use std::path::{Path, PathBuf};

use base64::Engine;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use rand::RngCore;

use crate::error::{AppError, AppResult};
use crate::setup;

const MAGIC: &str = "enc:v1:";
const KEY_BYTES: usize = 32;
const NONCE_BYTES: usize = 12;

fn key_path() -> AppResult<PathBuf> {
    Ok(setup::data_dir()?.join("master.key"))
}

#[cfg(unix)]
fn write_key_file(path: &Path, bytes: &[u8]) -> AppResult<()> {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
        let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
    }
    // Exclusive create so we never clobber an existing key.
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
    {
        Ok(mut f) => {
            use std::io::Write;
            f.write_all(bytes)?;
            Ok(())
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            // Another writer won the race; read whatever is there.
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

#[cfg(not(unix))]
fn write_key_file(path: &Path, bytes: &[u8]) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if path.exists() {
        return Ok(());
    }
    fs::write(path, bytes)?;
    Ok(())
}

fn load_or_create_key() -> AppResult<[u8; KEY_BYTES]> {
    let path = key_path()?;
    if path.exists() {
        let raw = fs::read(&path)?;
        if raw.len() != KEY_BYTES {
            return Err(AppError::msg(format!(
                "master.key has unexpected length {} (want {KEY_BYTES})",
                raw.len()
            )));
        }
        let mut key = [0u8; KEY_BYTES];
        key.copy_from_slice(&raw);
        return Ok(key);
    }
    let mut key = [0u8; KEY_BYTES];
    rand::thread_rng().fill_bytes(&mut key);
    write_key_file(&path, &key)?;
    // Re-read in case another process created first.
    let raw = fs::read(&path)?;
    if raw.len() != KEY_BYTES {
        return Err(AppError::msg("master.key unreadable after create"));
    }
    let mut out = [0u8; KEY_BYTES];
    out.copy_from_slice(&raw);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }
    Ok(out)
}

/// True when `stored` looks like a sealed blob from this module.
pub fn is_sealed(stored: &str) -> bool {
    stored.starts_with(MAGIC)
}

/// Seal a plaintext secret for persistence. Empty input stays empty.
pub fn seal(plaintext: &str) -> AppResult<String> {
    if plaintext.is_empty() {
        return Ok(String::new());
    }
    // Already sealed (e.g. read-modify-write of a row we just loaded sealed).
    if is_sealed(plaintext) {
        return Ok(plaintext.to_string());
    }
    let key_bytes = load_or_create_key()?;
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key_bytes));
    let mut nonce_bytes = [0u8; NONCE_BYTES];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|_| AppError::msg("failed to seal secret"))?;
    let mut packed = Vec::with_capacity(NONCE_BYTES + ciphertext.len());
    packed.extend_from_slice(&nonce_bytes);
    packed.extend_from_slice(&ciphertext);
    Ok(format!(
        "{MAGIC}{}",
        base64::engine::general_purpose::STANDARD.encode(packed)
    ))
}

/// Open a stored secret. Legacy plaintext (no magic prefix) is returned as-is
/// so existing DBs keep working until the next write re-seals them.
pub fn open(stored: &str) -> AppResult<String> {
    if stored.is_empty() || !is_sealed(stored) {
        return Ok(stored.to_string());
    }
    let b64 = &stored[MAGIC.len()..];
    let packed = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|_| AppError::msg("sealed secret is corrupt (base64)"))?;
    if packed.len() <= NONCE_BYTES {
        return Err(AppError::msg("sealed secret is corrupt (too short)"));
    }
    let (nonce_bytes, ciphertext) = packed.split_at(NONCE_BYTES);
    let key_bytes = load_or_create_key()?;
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key_bytes));
    let nonce = Nonce::from_slice(nonce_bytes);
    let plain = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| AppError::msg("failed to open sealed secret (wrong key or corrupt)"))?;
    String::from_utf8(plain).map_err(|_| AppError::msg("sealed secret is not valid UTF-8"))
}

/// Re-seal a value that may still be legacy plaintext. Used by migration.
pub fn reseal_if_plaintext(stored: &str) -> AppResult<(String, bool)> {
    if stored.is_empty() || is_sealed(stored) {
        return Ok((stored.to_string(), false));
    }
    Ok((seal(stored)?, true))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_isolated_data<F: FnOnce()>(f: F) {
        let _g = crate::test_lock::env_guard();
        let tmp = tempfile::tempdir().unwrap();
        let data = tmp.path().join("data");
        fs::create_dir_all(&data).unwrap();
        std::env::set_var("PHISHKIT_DATA", &data);
        std::env::set_var("PHISHKIT_CONFIG", tmp.path().join("config"));
        f();
        std::env::remove_var("PHISHKIT_DATA");
        std::env::remove_var("PHISHKIT_CONFIG");
    }

    #[test]
    fn seal_open_roundtrip() {
        with_isolated_data(|| {
            let sealed = seal("s3cr3t-smtp-pass").unwrap();
            assert!(is_sealed(&sealed), "must carry enc:v1: prefix");
            assert!(!sealed.contains("s3cr3t-smtp-pass"));
            assert_eq!(open(&sealed).unwrap(), "s3cr3t-smtp-pass");
        });
    }

    #[test]
    fn open_passes_through_legacy_plaintext() {
        with_isolated_data(|| {
            assert_eq!(open("legacy-plain").unwrap(), "legacy-plain");
            assert!(!is_sealed("legacy-plain"));
        });
    }

    #[test]
    fn empty_stays_empty() {
        with_isolated_data(|| {
            assert_eq!(seal("").unwrap(), "");
            assert_eq!(open("").unwrap(), "");
        });
    }

    #[test]
    fn reseal_upgrades_plaintext_once() {
        with_isolated_data(|| {
            let (first, changed) = reseal_if_plaintext("plain-pass").unwrap();
            assert!(changed);
            assert!(is_sealed(&first));
            let (second, changed2) = reseal_if_plaintext(&first).unwrap();
            assert!(!changed2);
            assert_eq!(first, second);
            assert_eq!(open(&first).unwrap(), "plain-pass");
        });
    }

    #[test]
    fn master_key_file_is_0600_on_unix() {
        with_isolated_data(|| {
            let _ = seal("x").unwrap();
            let path = key_path().unwrap();
            assert!(path.exists());
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
                assert_eq!(mode, 0o600, "master.key must be 0600, got {mode:o}");
            }
        });
    }

    #[test]
    fn wrong_key_fails_closed() {
        with_isolated_data(|| {
            let sealed = seal("top-secret").unwrap();
            // Clobber the key.
            let path = key_path().unwrap();
            fs::write(&path, [9u8; KEY_BYTES]).unwrap();
            let err = open(&sealed).unwrap_err().to_string();
            assert!(
                err.contains("failed to open") || err.contains("corrupt"),
                "expected decrypt failure, got: {err}"
            );
        });
    }
}
