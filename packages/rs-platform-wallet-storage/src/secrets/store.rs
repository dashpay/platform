//! [`SecretStore`] — the public, never-leaking secrets entry point.
//!
//! Consumers use this enum, not the `keyring_core` SPI. Its read path
//! ([`SecretStore::get`]) yields a zeroizing [`SecretBytes`]; a raw
//! `Vec<u8>` never crosses this boundary, and the write path
//! ([`SecretStore::set`]) takes `&SecretBytes` so a caller cannot pass an
//! unwrapped buffer (M-STRONG-TYPES).
//!
//! Errors surface as the typed [`FileStoreError`] — losslessly for the
//! [`SecretStore::File`] arm (so `WrongPassphrase` vs `Corruption` vs
//! `Busy` stay distinct), and as a best-effort projection of
//! `keyring_core::Error` for the [`SecretStore::Os`] arm. The internal
//! `keyring_core::api::CredentialApi` / `CredentialStoreApi` impls remain
//! the backend SPI; `SecretStore` delegates through them.

use std::sync::Arc;

use keyring_core::api::CredentialStoreApi;
use keyring_core::{Entry, Error as KeyringError};

use super::file::error::{FileStoreError, OsKeyringErrorKind};
use super::secret::SecretBytes;
use super::validate::WalletId;
use super::{default_credential_store, EncryptedFileStore, SERVICE_PREFIX};

/// A passphrase-or-OS-keyring backed store for wallet secret material.
///
/// The only public read path is [`get`](SecretStore::get), which yields a
/// zeroizing [`SecretBytes`] — a raw `Vec<u8>` never crosses this
/// boundary. Backend selection is an explicit operator decision; there is
/// no silent fallback between the two arms (SEC-REQ-2.1.3 / AR-4).
pub enum SecretStore {
    /// Self-contained Argon2id + XChaCha20-Poly1305 vault file.
    /// Recommended on headless / server hosts.
    File(EncryptedFileStore),
    /// The platform OS keyring (desktop), fail-closed on headless Linux.
    Os(Arc<dyn CredentialStoreApi + Send + Sync>),
}

impl SecretStore {
    /// Open (or prepare to create) a file-backed vault rooted at `dir`,
    /// unlocked by `passphrase`. `dir` is created if missing.
    pub fn file(
        dir: impl AsRef<std::path::Path>,
        passphrase: super::SecretString,
    ) -> Result<Self, FileStoreError> {
        Ok(Self::File(EncryptedFileStore::open(dir, passphrase)?))
    }

    /// Open the platform's default OS keyring, failing closed when none
    /// is reachable (headless / no Secret Service).
    pub fn os() -> Result<Self, FileStoreError> {
        Ok(Self::Os(default_credential_store().map_err(map_spi)?))
    }

    /// Store `secret` under `(service, label)`, overwriting any prior
    /// value. Takes `&SecretBytes` so the caller cannot pass an unwrapped
    /// buffer; the wrapped bytes are exposed to the SPI only at the last
    /// moment.
    pub fn set(
        &self,
        service: &WalletId,
        label: &str,
        secret: &SecretBytes,
    ) -> Result<(), FileStoreError> {
        match self {
            // File arm: the inherent typed path — no lossy SPI seam.
            Self::File(s) => s.put_bytes(service, label, secret.expose_secret()),
            Self::Os(store) => {
                let entry = build_os(store, service, label)?;
                entry.set_secret(secret.expose_secret()).map_err(map_spi)
            }
        }
    }

    /// Retrieve the secret stored under `(service, label)`, or `Ok(None)`
    /// if absent. The plaintext is wrapped into [`SecretBytes`] at the
    /// seam with no named `Vec` intermediate, so the bare-buffer window is
    /// zero statements.
    pub fn get(
        &self,
        service: &WalletId,
        label: &str,
    ) -> Result<Option<SecretBytes>, FileStoreError> {
        match self {
            // File arm: the inherent typed path keeps `WrongPassphrase`
            // vs `Corruption` distinct (lossless).
            Self::File(s) => Ok(s.get_bytes(service, label)?.map(SecretBytes::new)),
            Self::Os(store) => {
                let entry = build_os(store, service, label)?;
                match entry.get_secret() {
                    Ok(v) => Ok(Some(SecretBytes::new(v))),
                    Err(KeyringError::NoEntry) => Ok(None),
                    Err(e) => Err(map_spi(e)),
                }
            }
        }
    }

    /// Delete the secret stored under `(service, label)`. Absent entries
    /// are a no-op (`Ok(())`), so deletion is idempotent.
    pub fn delete(&self, service: &WalletId, label: &str) -> Result<(), FileStoreError> {
        match self {
            Self::File(s) => {
                s.delete_bytes(service, label)?;
                Ok(())
            }
            Self::Os(store) => {
                let entry = build_os(store, service, label)?;
                match entry.delete_credential() {
                    Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
                    Err(e) => Err(map_spi(e)),
                }
            }
        }
    }
}

/// Build the SPI [`Entry`] for `(service, label)` on the OS-keyring arm.
fn build_os(
    store: &Arc<dyn CredentialStoreApi + Send + Sync>,
    service: &WalletId,
    label: &str,
) -> Result<Entry, FileStoreError> {
    let svc = format!("{SERVICE_PREFIX}{}", service.to_hex());
    store.build(&svc, label, None).map_err(map_spi)
}

impl std::fmt::Debug for SecretStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::File(s) => f.debug_tuple("SecretStore::File").field(s).finish(),
            Self::Os(_) => f.write_str("SecretStore::Os(..)"),
        }
    }
}

/// Project an OS-keyring SPI [`KeyringError`] into the typed
/// [`FileStoreError`] for the [`Os`](SecretStore::Os) arm.
///
/// The OS keyring has no typed `FileStoreError` origin, so its variants
/// map best-effort into [`FileStoreError::OsKeyring`] (carrying only a
/// non-secret discriminant) or the closest existing variant. Secret-
/// bearing keyring variants (`BadEncoding`, `BadDataFormat`) are
/// collapsed to a discriminant — their raw bytes never enter
/// `FileStoreError`. (The [`File`](SecretStore::File) arm never reaches
/// this projection: it uses the inherent typed path.)
fn map_spi(e: KeyringError) -> FileStoreError {
    match e {
        KeyringError::NoEntry => FileStoreError::OsKeyring {
            kind: OsKeyringErrorKind::NoEntry,
        },
        KeyringError::NoStorageAccess(_) => FileStoreError::OsKeyring {
            kind: OsKeyringErrorKind::NoStorageAccess,
        },
        KeyringError::NoDefaultStore => FileStoreError::OsKeyring {
            kind: OsKeyringErrorKind::NoDefaultStore,
        },
        KeyringError::Invalid(_, _) => FileStoreError::InvalidLabel,
        KeyringError::BadStoreFormat(_)
        | KeyringError::BadEncoding(_)
        | KeyringError::BadDataFormat(_, _) => FileStoreError::OsKeyring {
            kind: OsKeyringErrorKind::BadStoreFormat,
        },
        _ => FileStoreError::OsKeyring {
            kind: OsKeyringErrorKind::Backend,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::SecretString;

    fn file_store(dir: &std::path::Path) -> SecretStore {
        SecretStore::file(dir, SecretString::new("pw-correct")).unwrap()
    }

    fn wid(b: u8) -> WalletId {
        WalletId::from([b; 32])
    }

    #[test]
    fn get_returns_secret_bytes_not_vec() {
        let dir = tempfile::tempdir().unwrap();
        let s = file_store(dir.path());
        s.set(&wid(1), "seed", &SecretBytes::from_slice(b"abc"))
            .unwrap();
        let got: Option<SecretBytes> = s.get(&wid(1), "seed").unwrap();
        let got = got.expect("present");
        assert_eq!(got.expose_secret(), b"abc");
    }

    #[test]
    fn get_absent_is_none() {
        let dir = tempfile::tempdir().unwrap();
        let s = file_store(dir.path());
        assert!(s.get(&wid(1), "seed").unwrap().is_none());
        s.set(&wid(1), "seed", &SecretBytes::from_slice(b"x"))
            .unwrap();
        assert!(s.get(&wid(1), "other").unwrap().is_none());
    }

    #[test]
    fn delete_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let s = file_store(dir.path());
        // Absent → Ok, no error.
        s.delete(&wid(1), "seed").unwrap();
        s.set(&wid(1), "seed", &SecretBytes::from_slice(b"x"))
            .unwrap();
        s.delete(&wid(1), "seed").unwrap();
        assert!(s.get(&wid(1), "seed").unwrap().is_none());
        // Second delete on the now-absent entry is still Ok.
        s.delete(&wid(1), "seed").unwrap();
    }

    #[test]
    fn wrong_passphrase_surfaces_typed_lossless() {
        let dir = tempfile::tempdir().unwrap();
        file_store(dir.path())
            .set(&wid(1), "seed", &SecretBytes::from_slice(b"orig"))
            .unwrap();
        let bad = SecretStore::file(dir.path(), SecretString::new("pw-wrong")).unwrap();
        let err = bad.get(&wid(1), "seed").unwrap_err();
        assert!(
            matches!(err, FileStoreError::WrongPassphrase),
            "expected WrongPassphrase, got {err:?}"
        );
    }

    #[test]
    fn corruption_surfaces_typed_lossless_distinct_from_wrong_passphrase() {
        let dir = tempfile::tempdir().unwrap();
        let s = file_store(dir.path());
        s.set(&wid(1), "seed", &SecretBytes::from_slice(b"value"))
            .unwrap();
        // Corrupt the entry ciphertext while leaving the verify-token
        // intact: the passphrase is still correct, so this is corruption,
        // not a wrong passphrase. The lossless typed path keeps them apart.
        let SecretStore::File(ref fs) = s else {
            unreachable!()
        };
        let path = fs.test_vault_path(&wid(1));
        let (header, mut entries) = fs.test_read_vault(&path).unwrap().unwrap();
        entries[0].ciphertext[0] ^= 0x01;
        fs.test_write_vault(&path, &header, &entries).unwrap();
        let err = s.get(&wid(1), "seed").unwrap_err();
        assert!(
            matches!(err, FileStoreError::Corruption),
            "expected Corruption, got {err:?}"
        );
    }

    #[test]
    fn busy_surfaces_typed_lossless() {
        // `set` builds a credential that clones the inner `Arc`, but it is
        // dropped at the end of `set`, so `rekey` then has the exclusive
        // reference. To observe `Busy` we hold a live credential across a
        // rekey on the same store.
        let dir = tempfile::tempdir().unwrap();
        let mut fs = EncryptedFileStore::open(dir.path(), SecretString::new("pw")).unwrap();
        let svc = format!("{SERVICE_PREFIX}{}", wid(1).to_hex());
        let live = fs.build(&svc, "seed", None).unwrap();
        live.set_secret(b"value").unwrap();
        let err = fs.rekey(wid(1), SecretString::new("pw-new")).unwrap_err();
        assert!(matches!(err, FileStoreError::Busy), "got {err:?}");
        drop(live);
    }

    #[test]
    fn debug_redacts() {
        let dir = tempfile::tempdir().unwrap();
        let s = file_store(dir.path());
        let dbg = format!("{s:?}");
        assert!(!dbg.contains("pw-correct"));
    }
}
