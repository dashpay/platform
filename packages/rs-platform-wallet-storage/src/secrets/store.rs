//! [`SecretStore`] — the public, never-leaking secrets entry point.
//!
//! Consumers use this enum, not the `keyring_core` SPI. Its read path
//! ([`SecretStore::get`]) yields a zeroizing [`SecretBytes`]; a raw
//! `Vec<u8>` never crosses this boundary, and the write path
//! ([`SecretStore::set`]) takes `&SecretBytes` so a caller cannot pass an
//! unwrapped buffer (M-STRONG-TYPES).
//!
//! Errors surface as the typed [`SecretStoreError`] — losslessly for the
//! [`SecretStore::File`] arm (so `WrongPassphrase` vs `Corruption` vs
//! `AlreadyLocked` stay distinct), and as a best-effort projection of
//! `keyring_core::Error` for the [`SecretStore::Os`] arm. The internal
//! `keyring_core::api::CredentialApi` / `CredentialStoreApi` impls remain
//! the backend SPI; `SecretStore` delegates through them.

use std::sync::Arc;

use keyring_core::api::CredentialStoreApi;
use keyring_core::{Entry, Error as KeyringError};

use super::error::{OsKeyringErrorKind, SecretStoreError};
use super::secret::SecretBytes;
use super::validate::WalletId;
use super::{default_credential_store, EncryptedFileStore, SERVICE_PREFIX};

/// A passphrase-or-OS-keyring backed store for wallet secret material.
///
/// The only public read path is [`get`](SecretStore::get), which yields a
/// zeroizing [`SecretBytes`] — a raw `Vec<u8>` never crosses this
/// boundary. Backend selection is an explicit operator decision; there is
/// no silent fallback between the two arms.
pub enum SecretStore {
    /// Self-contained Argon2id + XChaCha20-Poly1305 vault file.
    /// Recommended on headless / server hosts.
    File(EncryptedFileStore),
    /// The platform OS keyring (desktop), fail-closed on headless Linux.
    Os(Arc<dyn CredentialStoreApi + Send + Sync>),
}

impl SecretStore {
    /// Open (or prepare to create) a file-backed vault at `path`,
    /// unlocked by `passphrase`. `path` is the vault file itself
    /// (operator picks the filename); the parent directory is
    /// materialized on the first write.
    pub fn file(
        path: impl AsRef<std::path::Path>,
        passphrase: super::SecretString,
    ) -> Result<Self, SecretStoreError> {
        Ok(Self::File(EncryptedFileStore::open(path, passphrase)?))
    }

    /// Open the platform's default OS keyring, failing closed when none
    /// is reachable (headless / no Secret Service).
    pub fn os() -> Result<Self, SecretStoreError> {
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
    ) -> Result<(), SecretStoreError> {
        match self {
            // File arm: the inherent typed path — no lossy SPI seam.
            // `put_bytes` takes `&SecretBytes` directly, so the
            // bare-buffer view never crosses this boundary.
            Self::File(s) => s.put_bytes(service, label, secret),
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
    ) -> Result<Option<SecretBytes>, SecretStoreError> {
        match self {
            // File arm: the inherent typed path keeps `WrongPassphrase`
            // vs `Corruption` distinct (lossless). Plaintext rides as
            // `SecretBytes` all the way; no rewrap needed.
            Self::File(s) => s.get_bytes(service, label),
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
    pub fn delete(&self, service: &WalletId, label: &str) -> Result<(), SecretStoreError> {
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
///
/// The reject-not-sanitize label allowlist (`^[A-Za-z0-9._-]{1,64}$`)
/// is enforced here before the call crosses into the OS backend.
/// Different OS keyrings accept, normalize, or reject non-allowlisted
/// bytes inconsistently; enforcing the allowlist at
/// this shim keeps `(service, label)` invariants identical to the
/// `File` arm and across every OS backend.
fn build_os(
    store: &Arc<dyn CredentialStoreApi + Send + Sync>,
    service: &WalletId,
    label: &str,
) -> Result<Entry, SecretStoreError> {
    let label = super::validate::validated_label(label).map_err(SecretStoreError::from)?;
    let svc = format!("{SERVICE_PREFIX}{}", service.to_hex());
    store.build(&svc, label, None).map_err(map_spi)
}

impl std::fmt::Debug for SecretStore {
    /// Surfaces the backend engine/service identity without exposing any
    /// secret material. The `Os` arm reports the SPI
    /// `vendor()`/`id()` — non-secret backend tags (e.g. which OS keyring
    /// is wired up) — rather than an opaque `Os(..)`. The `File` arm
    /// delegates to [`EncryptedFileStore`]'s redacting `Debug` (path
    /// only, no key/passphrase).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::File(s) => f.debug_tuple("SecretStore::File").field(s).finish(),
            Self::Os(store) => f
                .debug_struct("SecretStore::Os")
                .field("vendor", &store.vendor())
                .field("id", &store.id())
                .finish(),
        }
    }
}

/// Project an OS-keyring SPI [`KeyringError`] into the typed
/// [`SecretStoreError`] for the [`Os`](SecretStore::Os) arm.
///
/// The OS keyring has no typed `SecretStoreError` origin, so its variants
/// map best-effort into [`SecretStoreError::OsKeyring`] (carrying only a
/// non-secret discriminant) or the closest existing variant. Secret-
/// bearing keyring variants (`BadEncoding`, `BadDataFormat`) are
/// collapsed to a discriminant — their raw bytes never enter
/// `SecretStoreError`. (The [`File`](SecretStore::File) arm never reaches
/// this projection: it uses the inherent typed path.)
fn map_spi(e: KeyringError) -> SecretStoreError {
    match e {
        KeyringError::NoEntry => SecretStoreError::OsKeyring {
            kind: OsKeyringErrorKind::NoEntry,
        },
        KeyringError::NoStorageAccess(_) => SecretStoreError::OsKeyring {
            kind: OsKeyringErrorKind::NoStorageAccess,
        },
        KeyringError::NoDefaultStore => SecretStoreError::OsKeyring {
            kind: OsKeyringErrorKind::NoDefaultStore,
        },
        KeyringError::Invalid(_, _) => SecretStoreError::InvalidLabel,
        KeyringError::BadStoreFormat(_)
        | KeyringError::BadEncoding(_)
        | KeyringError::BadDataFormat(_, _) => SecretStoreError::OsKeyring {
            kind: OsKeyringErrorKind::BadStoreFormat,
        },
        _ => SecretStoreError::OsKeyring {
            kind: OsKeyringErrorKind::Backend,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::secrets::SecretString;

    fn file_store(dir: &std::path::Path) -> SecretStore {
        SecretStore::file(dir.join("vault.pwsvault"), SecretString::new("pw-correct")).unwrap()
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
        // Resident-vault model: the passphrase is verified at open()
        // time (header verify-token), so a wrong-pass reopen fails at
        // open() rather than on the first get(). The typed distinction
        // still survives losslessly on the public path.
        let dir = tempfile::tempdir().unwrap();
        file_store(dir.path())
            .set(&wid(1), "seed", &SecretBytes::from_slice(b"orig"))
            .unwrap();
        let err = SecretStore::file(
            dir.path().join("vault.pwsvault"),
            SecretString::new("pw-wrong"),
        )
        .expect_err("wrong pass must fail open");
        assert!(
            matches!(err, SecretStoreError::WrongPassphrase),
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
        let mut vault = fs.test_read_vault_from_disk().unwrap().unwrap();
        vault
            .wallets
            .get_mut(&wid(1).to_hex())
            .unwrap()
            .get_mut("seed")
            .unwrap()
            .ciphertext[0] ^= 0x01;
        fs.test_write_vault_to_disk(&vault).unwrap();
        fs.test_reload_from_disk().unwrap();
        let err = s.get(&wid(1), "seed").unwrap_err();
        assert!(
            matches!(err, SecretStoreError::Corruption),
            "expected Corruption, got {err:?}"
        );
    }

    #[test]
    fn already_locked_surfaces_typed_lossless() {
        // Resident-vault model: a second open() of the same path while
        // the first store is alive returns AlreadyLocked. The typed
        // distinction survives losslessly on the public path.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vault.pwsvault");
        let _s1 = SecretStore::file(&path, SecretString::new("pw")).unwrap();
        let err = SecretStore::file(&path, SecretString::new("pw")).unwrap_err();
        assert!(
            matches!(err, SecretStoreError::AlreadyLocked),
            "got {err:?}"
        );
    }

    #[test]
    fn debug_redacts() {
        let dir = tempfile::tempdir().unwrap();
        let s = file_store(dir.path());
        let dbg = format!("{s:?}");
        assert!(!dbg.contains("pw-correct"));
    }

    /// The OS-keyring shim must enforce the label allowlist BEFORE
    /// handing the value to the OS backend. The per-backend label
    /// policies (macOS Keychain vs Windows
    /// Credential Manager vs Secret Service) differ in what they accept,
    /// normalize, or reject; the shim must keep the `(service, label)`
    /// invariant uniform across every arm.
    ///
    /// A mock `CredentialStoreApi` that panics if its `build()` is
    /// invoked proves the bad label never crosses the SPI seam — the
    /// shim rejects with `SecretStoreError::InvalidLabel` first.
    #[test]
    fn build_os_rejects_invalid_label_before_spi() {
        use std::any::Any;
        use std::collections::HashMap;
        use std::sync::Arc;

        use keyring_core::api::CredentialStoreApi;
        use keyring_core::{Entry, Result as KeyringResult};

        struct PanickingStore;

        impl CredentialStoreApi for PanickingStore {
            fn vendor(&self) -> String {
                "test".to_string()
            }
            fn id(&self) -> String {
                "panicking".to_string()
            }
            fn build(
                &self,
                _service: &str,
                _user: &str,
                _modifiers: Option<&HashMap<&str, &str>>,
            ) -> KeyringResult<Entry> {
                panic!("build_os must reject the label before reaching the SPI");
            }
            fn as_any(&self) -> &dyn Any {
                self
            }
        }

        let store: Arc<dyn CredentialStoreApi + Send + Sync> = Arc::new(PanickingStore);
        let os = SecretStore::Os(store);
        // Every operation on the OS arm goes through `build_os`; the
        // allowlist rejection MUST fire here, so the panicking SPI is
        // never reached.
        for bad in ["lab el", "../escape", "", "a:b", "a/b", "lab\0el"] {
            let err = os
                .set(&wid(1), bad, &SecretBytes::from_slice(b"x"))
                .unwrap_err();
            assert!(
                matches!(err, SecretStoreError::InvalidLabel),
                "set with label {bad:?} should reject as InvalidLabel, got {err:?}"
            );
            let err = os.get(&wid(1), bad).unwrap_err();
            assert!(
                matches!(err, SecretStoreError::InvalidLabel),
                "get with label {bad:?} should reject as InvalidLabel, got {err:?}"
            );
            let err = os.delete(&wid(1), bad).unwrap_err();
            assert!(
                matches!(err, SecretStoreError::InvalidLabel),
                "delete with label {bad:?} should reject as InvalidLabel, got {err:?}"
            );
        }
    }
}
