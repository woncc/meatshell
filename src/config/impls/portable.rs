//! Passphrase-protected portable exports (#26).
//!
//! On-disk layout, version 1 (little-endian integers):
//!
//! ```text
//! magic:        b"MSHX"          4
//! version:      u8 = 1           1
//! memory_kib:   u32              4
//! iterations:   u32              4
//! parallelism:  u32              4
//! salt:         [u8; 16]        16
//! nonce:        [u8; 24]        24
//! ciphertext:   XChaCha20-Poly1305(JSON) || tag
//! ```
//!
//! The header (everything before the ciphertext) is the AEAD additional data.
//! Version 1 always uses Argon2id; a crafted file cannot select Argon2i or
//! Argon2d. KDF parameters are stored so the file can be opened, and rejected
//! before Argon2 allocates if they exceed the bounds below.

use std::path::Path;

use anyhow::{anyhow, bail, Result};
use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, Payload},
    XChaCha20Poly1305,
};
use rand::rngs::OsRng;
use rand::RngCore;
use zeroize::Zeroizing;

use super::{ConfigStore, ExportFile};

pub const EXPORT_MAGIC: &[u8; 4] = b"MSHX";
pub const EXPORT_VERSION: u8 = 1;
pub const HEADER_LEN: usize = 57;
const TAG_LEN: usize = 16;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 24;

/// Same ceiling as JSON import. Checked before any KDF work.
pub(crate) const MAX_EXPORT_BYTES: usize = 16 * 1024 * 1024;

pub const PASSPHRASE_MIN_CHARS: usize = 8;
const PASSPHRASE_MAX_BYTES: usize = 1024;

/// OWASP interactive Argon2id baseline (19 MiB, t=2, p=1). Not a test hook.
const PRODUCTION_MEMORY_KIB: u32 = 19 * 1024;
const PRODUCTION_ITERATIONS: u32 = 2;
const PRODUCTION_PARALLELISM: u32 = 1;

/// Reject before Argon2 builds its memory blocks.
const MAX_MEMORY_KIB: u32 = 64 * 1024;
const MAX_ITERATIONS: u32 = 8;
const MAX_PARALLELISM: u32 = 4;

pub const ERR_PASSPHRASE_TOO_SHORT: &str = "passphrase must be at least 8 characters";
pub const ERR_PASSPHRASE_TOO_LONG: &str = "passphrase is too long";
pub const ERR_PASSPHRASE_MISMATCH: &str = "passphrases do not match";
pub const ERR_PASSPHRASE_REQUIRED: &str =
    "this export is protected by a passphrase; import it from the MeatShell window";
pub const ERR_EXPORT_AUTH: &str = "passphrase incorrect or export file is damaged";
pub const ERR_EXPORT_TRUNCATED: &str = "export file is truncated";
pub const ERR_EXPORT_KDF: &str = "export file key-derivation parameters are not allowed";
pub const ERR_EXPORT_VERSION: &str = "unsupported passphrase export version";
pub const ERR_EXPORT_NOT_PASSPHRASE: &str = "not a passphrase-protected export";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ExportKdf {
    pub memory_kib: u32,
    pub iterations: u32,
    pub parallelism: u32,
}

impl ExportKdf {
    pub(crate) const fn production() -> Self {
        Self {
            memory_kib: PRODUCTION_MEMORY_KIB,
            iterations: PRODUCTION_ITERATIONS,
            parallelism: PRODUCTION_PARALLELISM,
        }
    }

    /// Fast parameters for unit tests only. Production export never calls this.
    #[cfg(test)]
    pub(crate) const fn for_tests() -> Self {
        Self {
            memory_kib: 8,
            iterations: 1,
            parallelism: 1,
        }
    }
}

pub(crate) fn kdf_allowed(memory_kib: u32, iterations: u32, parallelism: u32) -> bool {
    (1..=MAX_PARALLELISM).contains(&parallelism)
        && (1..=MAX_ITERATIONS).contains(&iterations)
        && memory_kib >= parallelism.saturating_mul(8)
        && memory_kib <= MAX_MEMORY_KIB
}

pub fn validate_new_passphrase(passphrase: &str, confirm: &str) -> Result<()> {
    check_passphrase_policy(passphrase)?;
    if passphrase != confirm {
        bail!(ERR_PASSPHRASE_MISMATCH);
    }
    Ok(())
}

fn check_passphrase_policy(passphrase: &str) -> Result<()> {
    if passphrase.chars().count() < PASSPHRASE_MIN_CHARS {
        bail!(ERR_PASSPHRASE_TOO_SHORT);
    }
    if passphrase.len() > PASSPHRASE_MAX_BYTES {
        bail!(ERR_PASSPHRASE_TOO_LONG);
    }
    Ok(())
}

pub(crate) fn is_passphrase_export(bytes: &[u8]) -> bool {
    bytes.starts_with(EXPORT_MAGIC)
}

impl ConfigStore {
    /// Seal every session, including hosts and proxy userinfo, under `passphrase`.
    pub fn export_json(&self, passphrase: &str) -> Result<(Vec<u8>, usize)> {
        self.seal_sessions(passphrase, ExportKdf::production())
    }

    /// Write a passphrase-protected export. A rejected passphrase writes nothing.
    pub fn export_to(&self, path: &Path, passphrase: &str) -> Result<usize> {
        let (bytes, count) = self.export_json(passphrase)?;
        std::fs::write(path, &bytes).map_err(|_| anyhow!("failed to write export file"))?;
        Ok(count)
    }

    #[cfg(test)]
    pub(crate) fn export_json_for_tests(&self, passphrase: &str) -> Result<(Vec<u8>, usize)> {
        self.seal_sessions(passphrase, ExportKdf::for_tests())
    }

    fn seal_sessions(&self, passphrase: &str, kdf: ExportKdf) -> Result<(Vec<u8>, usize)> {
        check_passphrase_policy(passphrase)?;
        if !kdf_allowed(kdf.memory_kib, kdf.iterations, kdf.parallelism) {
            bail!(ERR_EXPORT_KDF);
        }
        let mut payload = ExportFile {
            meatshell_export: 1,
            sessions: self.cache.sessions.clone(),
        };
        for session in &mut payload.sessions {
            session.last_used = None;
        }
        let count = payload.sessions.len();
        let plain = Zeroizing::new(
            serde_json::to_vec(&payload).map_err(|_| anyhow!("failed to encode export"))?,
        );
        let sealed = seal_export(&plain, passphrase, kdf)?;
        Ok((sealed, count))
    }
}

pub(crate) fn seal_export(plain: &[u8], passphrase: &str, kdf: ExportKdf) -> Result<Vec<u8>> {
    check_passphrase_policy(passphrase)?;
    if !kdf_allowed(kdf.memory_kib, kdf.iterations, kdf.parallelism) {
        bail!(ERR_EXPORT_KDF);
    }
    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);
    if nonce.len() != NONCE_LEN {
        bail!("export encryption failed");
    }
    let mut header = [0u8; HEADER_LEN];
    header[..4].copy_from_slice(EXPORT_MAGIC);
    header[4] = EXPORT_VERSION;
    header[5..9].copy_from_slice(&kdf.memory_kib.to_le_bytes());
    header[9..13].copy_from_slice(&kdf.iterations.to_le_bytes());
    header[13..17].copy_from_slice(&kdf.parallelism.to_le_bytes());
    header[17..33].copy_from_slice(&salt);
    header[33..57].copy_from_slice(&nonce);

    let key = derive_key(passphrase, &salt, kdf)?;
    let cipher = XChaCha20Poly1305::new(key.as_ref().into());
    let ciphertext = cipher
        .encrypt(
            &nonce,
            Payload {
                msg: plain,
                aad: &header,
            },
        )
        .map_err(|_| anyhow!("export encryption failed"))?;
    let mut out = Vec::with_capacity(HEADER_LEN + ciphertext.len());
    out.extend_from_slice(&header);
    out.extend_from_slice(&ciphertext);
    if out.len() > MAX_EXPORT_BYTES {
        bail!("import file exceeds the 16 MiB limit");
    }
    Ok(out)
}

/// Decrypt a version-1 export. Wrong passphrase and tampering return the same error.
pub(crate) fn open_export(blob: &[u8], passphrase: &str) -> Result<Zeroizing<Vec<u8>>> {
    if blob.len() > MAX_EXPORT_BYTES {
        bail!("import file exceeds the 16 MiB limit");
    }
    if blob.len() < 4 || &blob[..4] != EXPORT_MAGIC {
        bail!(ERR_EXPORT_NOT_PASSPHRASE);
    }
    if blob.len() < HEADER_LEN + TAG_LEN {
        bail!(ERR_EXPORT_TRUNCATED);
    }
    let header = &blob[..HEADER_LEN];
    if header[4] != EXPORT_VERSION {
        bail!(ERR_EXPORT_VERSION);
    }
    let memory_kib = read_u32(&header[5..9])?;
    let iterations = read_u32(&header[9..13])?;
    let parallelism = read_u32(&header[13..17])?;
    // DoS guard: never construct Argon2 params, and never allocate its blocks,
    // until the header values are inside the production ceiling.
    if !kdf_allowed(memory_kib, iterations, parallelism) {
        bail!(ERR_EXPORT_KDF);
    }
    check_passphrase_policy(passphrase).map_err(|_| anyhow!(ERR_EXPORT_AUTH))?;
    let salt = &header[17..33];
    let nonce_bytes = &header[33..57];
    let kdf = ExportKdf {
        memory_kib,
        iterations,
        parallelism,
    };
    let key = derive_key(passphrase, salt, kdf)?;
    let cipher = XChaCha20Poly1305::new(key.as_ref().into());
    let nonce = chacha20poly1305::XNonce::from_slice(nonce_bytes);
    let plain = cipher
        .decrypt(
            nonce,
            Payload {
                msg: &blob[HEADER_LEN..],
                aad: header,
            },
        )
        .map_err(|_| anyhow!(ERR_EXPORT_AUTH))?;
    Ok(Zeroizing::new(plain))
}

fn derive_key(passphrase: &str, salt: &[u8], kdf: ExportKdf) -> Result<Zeroizing<[u8; 32]>> {
    if !kdf_allowed(kdf.memory_kib, kdf.iterations, kdf.parallelism) {
        bail!(ERR_EXPORT_KDF);
    }
    if salt.len() != SALT_LEN {
        bail!(ERR_EXPORT_KDF);
    }
    let params = Params::new(kdf.memory_kib, kdf.iterations, kdf.parallelism, Some(32))
        .map_err(|_| anyhow!(ERR_EXPORT_KDF))?;
    // Version 1 is Argon2id only. Algorithm is not read from the file.
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = Zeroizing::new([0u8; 32]);
    argon
        .hash_password_into(passphrase.as_bytes(), salt, key.as_mut())
        .map_err(|_| anyhow!(ERR_EXPORT_KDF))?;
    Ok(key)
}

fn read_u32(bytes: &[u8]) -> Result<u32> {
    let sized: [u8; 4] = bytes
        .get(..4)
        .and_then(|slice| slice.try_into().ok())
        .ok_or_else(|| anyhow!(ERR_EXPORT_TRUNCATED))?;
    Ok(u32::from_le_bytes(sized))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Secret;

    fn fixture_session() -> crate::config::Session {
        let mut session = crate::config::Session::new_empty();
        session.name = "synthetic".into();
        session.host = "secret-host.example".into();
        session.port = 22;
        session.user = "fixture".into();
        session.password = Secret::new("session-secret");
        session.private_key_inline = Secret::new("inline-key-secret");
        session.proxy = "http://user:proxy-secret@127.0.0.1:9".into();
        session.triggers.push(crate::config::SessionTrigger {
            expect: "prompt".into(),
            response: Secret::new("trigger-secret"),
            append_enter: true,
            repeat: false,
        });
        session
    }

    #[test]
    fn production_kdf_is_bounded_argon2id_preset() {
        let kdf = ExportKdf::production();
        assert!(kdf_allowed(kdf.memory_kib, kdf.iterations, kdf.parallelism));
        assert_eq!(kdf.memory_kib, 19 * 1024);
        assert_eq!(kdf.iterations, 2);
        assert_eq!(kdf.parallelism, 1);
        assert!(ExportKdf::for_tests().memory_kib < kdf.memory_kib);
        assert!(!kdf_allowed(u32::MAX, 1, 1));
        assert!(!kdf_allowed(8, u32::MAX, 1));
        assert!(!kdf_allowed(8, 1, 2));
        assert!(!kdf_allowed(64 * 1024 + 1, 2, 1));
    }

    #[test]
    fn roundtrip_hides_hosts_proxy_and_secrets_and_never_emits_legacy_prefix() {
        let mut store = super::super::tests::temp_store();
        store.cache.sessions.push(fixture_session());
        let (blob, count) = store.export_json_for_tests("test-passphrase").unwrap();
        assert_eq!(count, 1);
        assert!(blob.starts_with(EXPORT_MAGIC));
        assert_eq!(blob[4], EXPORT_VERSION);
        assert_eq!(u32::from_le_bytes(blob[5..9].try_into().unwrap()), 8);
        let raw = String::from_utf8_lossy(&blob);
        for secret in [
            "enc:exp:v1:",
            "session-secret",
            "inline-key-secret",
            "proxy-secret",
            "secret-host.example",
            "trigger-secret",
        ] {
            assert!(!raw.contains(secret), "{secret} leaked into export");
        }
        let plain = open_export(&blob, "test-passphrase").unwrap();
        let text = String::from_utf8(plain.to_vec()).unwrap();
        assert!(text.contains("secret-host.example"));
        assert!(text.contains("proxy-secret"));
        assert!(!text.contains("enc:exp:v1:"));

        let (again, _) = store.export_json_for_tests("test-passphrase").unwrap();
        assert_ne!(blob, again, "salt or nonce was reused");

        let mut destination = super::super::tests::temp_store();
        destination.key = [9; 32];
        let (summary, legacy) = destination
            .import_portable_bytes(&blob, Some("test-passphrase"), false)
            .unwrap();
        assert!(!legacy);
        assert_eq!((summary.added, summary.skipped), (1, 0));
        assert_eq!(
            destination.cache.sessions[0].password.as_str(),
            "session-secret"
        );
        assert_eq!(
            destination.cache.sessions[0].proxy,
            "http://user:proxy-secret@127.0.0.1:9"
        );
        let _ = std::fs::remove_file(&store.path);
        let _ = std::fs::remove_file(&destination.path);
    }

    #[test]
    fn wrong_passphrase_tamper_truncation_and_kdf_bounds_fail_closed() {
        let mut store = super::super::tests::temp_store();
        store.cache.sessions.push(fixture_session());
        store.save().unwrap();
        let before = serde_json::to_value(&store.cache).unwrap();
        let disk = std::fs::read(&store.path).unwrap();
        let (blob, _) = store.export_json_for_tests("test-passphrase").unwrap();

        let wrong = open_export(&blob, "other-passphrase").unwrap_err();
        assert_eq!(wrong.to_string(), ERR_EXPORT_AUTH);
        assert!(!wrong.to_string().contains("session-secret"));

        let mut header_flip = blob.clone();
        header_flip[20] ^= 0xff;
        let header_err = open_export(&header_flip, "test-passphrase").unwrap_err();
        assert_eq!(header_err.to_string(), ERR_EXPORT_AUTH);

        let mut cipher_flip = blob.clone();
        let last = cipher_flip.len() - 1;
        cipher_flip[last] ^= 0xff;
        let cipher_err = open_export(&cipher_flip, "test-passphrase").unwrap_err();
        assert_eq!(cipher_err.to_string(), ERR_EXPORT_AUTH);

        let truncated = open_export(&blob[..HEADER_LEN], "test-passphrase").unwrap_err();
        assert_eq!(truncated.to_string(), ERR_EXPORT_TRUNCATED);

        let mut huge = blob.clone();
        huge[5..9].copy_from_slice(&u32::MAX.to_le_bytes());
        let kdf_err = open_export(&huge, "test-passphrase").unwrap_err();
        assert_eq!(kdf_err.to_string(), ERR_EXPORT_KDF);

        let mut huge_time = vec![0u8; HEADER_LEN + TAG_LEN];
        huge_time[..4].copy_from_slice(EXPORT_MAGIC);
        huge_time[4] = EXPORT_VERSION;
        huge_time[5..9].copy_from_slice(&8u32.to_le_bytes());
        huge_time[9..13].copy_from_slice(&u32::MAX.to_le_bytes());
        huge_time[13..17].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(
            open_export(&huge_time, "test-passphrase")
                .unwrap_err()
                .to_string(),
            ERR_EXPORT_KDF
        );

        let mut bad_version = blob.clone();
        bad_version[4] = 99;
        assert_eq!(
            open_export(&bad_version, "test-passphrase")
                .unwrap_err()
                .to_string(),
            ERR_EXPORT_VERSION
        );

        let import_err = store
            .import_portable_bytes(&blob, Some("other-passphrase"), false)
            .unwrap_err();
        assert_eq!(import_err.to_string(), ERR_EXPORT_AUTH);
        assert!(!import_err.to_string().contains("session-secret"));
        assert_eq!(serde_json::to_value(&store.cache).unwrap(), before);
        assert_eq!(std::fs::read(&store.path).unwrap(), disk);

        let missing = store.import_portable_bytes(&blob, None, true).unwrap_err();
        assert_eq!(missing.to_string(), ERR_PASSPHRASE_REQUIRED);
        assert!(!missing.to_string().contains("test-passphrase"));
        assert_eq!(std::fs::read(&store.path).unwrap(), disk);

        let path =
            std::env::temp_dir().join(format!("ms-exp-reject-{}.json", uuid::Uuid::new_v4()));
        assert!(store.export_to(&path, "short").is_err());
        assert!(!path.exists());
        assert_eq!(
            validate_new_passphrase("test-passphrase", "other-passphrase")
                .unwrap_err()
                .to_string(),
            ERR_PASSPHRASE_MISMATCH
        );
        let _ = std::fs::remove_file(&store.path);
    }

    #[test]
    fn production_header_records_production_params() {
        let store = super::super::tests::temp_store();
        let (blob, _) = store.export_json("test-passphrase").unwrap();
        assert_eq!(blob[4], EXPORT_VERSION);
        assert_eq!(
            u32::from_le_bytes(blob[5..9].try_into().unwrap()),
            PRODUCTION_MEMORY_KIB
        );
        assert_eq!(
            u32::from_le_bytes(blob[9..13].try_into().unwrap()),
            PRODUCTION_ITERATIONS
        );
        assert_eq!(
            u32::from_le_bytes(blob[13..17].try_into().unwrap()),
            PRODUCTION_PARALLELISM
        );
        let plain = open_export(&blob, "test-passphrase").unwrap();
        assert!(plain.starts_with(b"{"));
        assert!(open_export(&blob, "other-passphrase").is_err());
    }
}
