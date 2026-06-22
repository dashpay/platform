//! [`SecretStore`] — the public, never-leaking secrets entry point.
//!
//! Consumers use this enum, not the `keyring_core` SPI it delegates to.
//! Reads yield a zeroizing [`SecretBytes`] and writes take `&SecretBytes`
//! so a raw buffer never crosses the boundary. Errors are the typed
//! [`SecretStoreError`] — lossless on the [`SecretStore::File`] arm, a
//! best-effort projection of `keyring_core::Error` on the
//! [`SecretStore::Os`] arm.

use std::sync::Arc;

use keyring_core::api::CredentialStoreApi;
use keyring_core::{Entry, Error as KeyringError};

use super::envelope;
use super::error::{OsKeyringErrorKind, SecretStoreError};
use super::secret::{SecretBytes, SecretString};
use super::validate::WalletId;
use super::{default_credential_store, EncryptedFileStore, MAX_SECRET_LEN, SERVICE_PREFIX};

/// A passphrase-or-OS-keyring backed store for wallet secret material.
///
/// Every read path ([`get`](SecretStore::get),
/// [`get_secret`](SecretStore::get_secret), and the read inside
/// [`reprotect`](SecretStore::reprotect)) yields a zeroizing
/// [`SecretBytes`] — a raw `Vec<u8>` never crosses this boundary. Backend
/// selection is an explicit operator decision; there is no silent fallback
/// between the two arms.
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

    /// Open (or create) a **deliberately keyless** file-backed vault — the
    /// only door that takes no passphrase. Obfuscation, not confidentiality
    /// (the key derives from an empty passphrase under the public salt): use
    /// it where the stored secrets carry their own Tier-2 object password,
    /// or as a staging step before [`EncryptedFileStore::rekey`] to a real
    /// passphrase. [`file`](SecretStore::file) rejects a blank passphrase;
    /// this is the explicit keyless alternative.
    pub fn file_unprotected(path: impl AsRef<std::path::Path>) -> Result<Self, SecretStoreError> {
        Ok(Self::File(EncryptedFileStore::open_unprotected(path)?))
    }

    /// Open the platform's default OS keyring, failing closed when none
    /// is reachable (headless / no Secret Service).
    pub fn os() -> Result<Self, SecretStoreError> {
        Ok(Self::Os(default_credential_store().map_err(map_spi)?))
    }

    /// Store `secret` under `(service, label)` UNPROTECTED (Tier-2
    /// scheme-0), overwriting any prior value — a `set_secret(.., None)`
    /// wrapper kept for non-breaking back-compat. Takes `&SecretBytes` so
    /// the caller cannot pass an unwrapped buffer.
    pub fn set(
        &self,
        service: &WalletId,
        label: &str,
        secret: &SecretBytes,
    ) -> Result<(), SecretStoreError> {
        self.set_secret(service, label, secret, None)
    }

    /// Store `secret` under `(service, label)`, overwriting any prior value.
    ///
    /// `password` selects the protection: `None` writes an unprotected
    /// envelope; `Some(pw)` seals the bytes under the object password `pw`
    /// (Argon2id + XChaCha20-Poly1305) **before** they reach the backend, so
    /// a protected object stays confidential even under a full backend
    /// compromise. A blank `pw` is rejected
    /// ([`BlankPassphrase`](SecretStoreError::BlankPassphrase)).
    ///
    /// **No recovery (availability):** if a protected object's password is
    /// lost, the object is permanently unrecoverable — there is no reset
    /// path. The UX must state this plainly.
    ///
    /// **Entropy is the caller's:** a protected object's confidentiality
    /// rests entirely on the password's entropy against an offline Argon2id
    /// attacker who already holds the backend. This crate enforces only
    /// non-blank; strength estimation / policy is the caller's job.
    ///
    /// The write is a same-slot overwrite that leaves the prior value intact
    /// on a crash: on the `File` arm via the vault's atomic replace; on the
    /// `Os` arm via the backend's single-item-replace contract.
    /// Add/change/remove flows go through [`reprotect`](SecretStore::reprotect).
    pub fn set_secret(
        &self,
        service: &WalletId,
        label: &str,
        secret: &SecretBytes,
        password: Option<&SecretString>,
    ) -> Result<(), SecretStoreError> {
        // Wrap above the backend: the backend only ever stores the opaque
        // envelope (ciphertext for a protected object).
        let blob = envelope::wrap(service, label, password, secret.expose_secret())?;
        self.put_raw(service, label, &blob)
    }

    /// Store the already-enveloped opaque `blob` under `(service, label)`.
    /// The shared write seam under [`set`] and [`set_secret`].
    ///
    /// [`set`]: SecretStore::set
    fn put_raw(
        &self,
        service: &WalletId,
        label: &str,
        blob: &SecretBytes,
    ) -> Result<(), SecretStoreError> {
        match self {
            // Inherent typed path — no lossy SPI seam, no bare buffer.
            Self::File(s) => s.put_bytes(service, label, blob),
            Self::Os(store) => {
                let entry = build_os(store, service, label)?;
                entry.set_secret(blob.expose_secret()).map_err(map_spi)
            }
        }
    }

    /// Retrieve the UNPROTECTED secret stored under `(service, label)`, or
    /// `Ok(None)` if absent — a `get_secret(.., None)` wrapper kept for
    /// non-breaking back-compat. A scheme-1 (password-protected) object read
    /// through this path returns
    /// [`NeedsPassword`](SecretStoreError::NeedsPassword); use
    /// [`get_secret`](SecretStore::get_secret) with the object password.
    pub fn get(
        &self,
        service: &WalletId,
        label: &str,
    ) -> Result<Option<SecretBytes>, SecretStoreError> {
        self.get_secret(service, label, None)
    }

    /// Read the opaque bytes stored under `(service, label)`, or `Ok(None)`
    /// if absent — the raw backend value (a Tier-2 envelope once writes go
    /// through [`set_secret`](SecretStore::set_secret), or a legacy raw
    /// value). The typed-vs-SPI distinction is preserved exactly as the
    /// pre-Tier-2 path did. This is the shared seam under [`get`] and
    /// [`get_secret`]; it does NOT interpret the envelope.
    ///
    /// [`get`]: SecretStore::get
    fn get_raw(
        &self,
        service: &WalletId,
        label: &str,
    ) -> Result<Option<SecretBytes>, SecretStoreError> {
        match self {
            // Inherent typed path: keeps WrongPassphrase vs Corruption
            // distinct; plaintext rides as SecretBytes, no rewrap.
            Self::File(s) => s.get_bytes(service, label),
            Self::Os(store) => {
                let entry = build_os(store, service, label)?;
                match entry.get_secret() {
                    Ok(v) => {
                        // Defense-in-depth: reject an oversized backend blob
                        // before it reaches the envelope parse/derive path.
                        // The File arm's stored bytes are already capped at
                        // MAX_SECRET_LEN by `put_bytes`; the Os backend has no
                        // such ceiling, so cap here. A legitimate envelope
                        // never exceeds MAX_SECRET_LEN; the overhead is
                        // headroom.
                        let cap = MAX_SECRET_LEN + envelope::MAX_ENVELOPE_OVERHEAD;
                        if v.len() > cap {
                            return Err(SecretStoreError::SecretTooLarge {
                                found: v.len(),
                                max: cap,
                            });
                        }
                        Ok(Some(SecretBytes::new(v)))
                    }
                    Err(KeyringError::NoEntry) => Ok(None),
                    Err(e) => Err(map_spi(e)),
                }
            }
        }
    }

    /// Retrieve the secret under `(service, label)` applying the strict,
    /// fail-closed read, or `Ok(None)` if absent.
    ///
    /// `password` IS the caller's protection assertion — supply `Some(pw)`
    /// for an object the caller's trusted model says is protected, `None`
    /// otherwise. The expectation lives ONLY here, never in the stored
    /// blob (see [`envelope::unwrap`]):
    ///
    /// - `Some(pw)` + a protected blob → the secret (or
    ///   [`WrongPassword`](SecretStoreError::WrongPassword) on tag fail);
    /// - `Some(pw)` + a non-protected blob (unprotected / legacy raw) →
    ///   [`ExpectedProtectedButUnsealed`](SecretStoreError::ExpectedProtectedButUnsealed)
    ///   — a strip/downgrade, refused, no bytes returned;
    /// - `None` + a protected blob →
    ///   [`NeedsPassword`](SecretStoreError::NeedsPassword) (never ciphertext);
    /// - `None` + an unprotected / legacy raw blob → the secret.
    ///
    /// **Documented residual:** an attacker who ALSO rewrites the
    /// consumer's trusted DB so the caller passes `None` for a stripped
    /// object can still downgrade — out of this library's reach by
    /// construction (the protection expectation is the caller's; see
    /// `SECRETS.md`). The expectation is NEVER persisted by the library.
    pub fn get_secret(
        &self,
        service: &WalletId,
        label: &str,
        password: Option<&SecretString>,
    ) -> Result<Option<SecretBytes>, SecretStoreError> {
        // Absence is availability-only (deletion = DoS, never injection):
        // a missing entry is Ok(None) under either password argument.
        let Some(stored) = self.get_raw(service, label)? else {
            return Ok(None);
        };
        envelope::unwrap(service, label, password, stored.expose_secret()).map(Some)
    }

    /// Add / change / remove an object password in one same-slot
    /// unwrap→rewrap→overwrite — the canonical re-protection flow.
    ///
    /// Reads the object under the `current` expectation (so a strip is
    /// caught fail-closed before any rewrap), then re-writes it under
    /// `new`:
    /// - **add:** `current = None`, `new = Some(pw)`;
    /// - **change:** `current = Some(old)`, `new = Some(pw_new)`;
    /// - **remove:** `current = Some(old)`, `new = None`.
    ///
    /// An absent object is a no-op (`Ok(())`). The rewrite is the same-slot
    /// overwrite of [`set_secret`], so a crash between the read and the
    /// commit leaves the prior value intact and readable under `current`.
    /// After a successful call the consumer MUST update its own trusted
    /// protection-status record (the protection expectation lives there).
    ///
    /// **No recovery:** changing or removing requires the `current`
    /// password; if it is lost the object cannot be re-protected or read,
    /// and is permanently unrecoverable (availability trade-off).
    ///
    /// **Entropy is the caller's:** the `new` password's entropy is the
    /// whole confidentiality guarantee for the re-protected object; this
    /// crate enforces only non-blank, not strength.
    pub fn reprotect(
        &self,
        service: &WalletId,
        label: &str,
        current: Option<&SecretString>,
        new: Option<&SecretString>,
    ) -> Result<(), SecretStoreError> {
        let Some(secret) = self.get_secret(service, label, current)? else {
            return Ok(());
        };
        self.set_secret(service, label, &secret, new)
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
/// Enforces the label allowlist (`^[A-Za-z0-9._-]{1,64}$`) before the
/// call crosses into the OS backend, so the `(service, label)` invariant
/// stays identical to the `File` arm and across every OS keyring (each
/// accepts / normalizes / rejects non-allowlisted bytes differently).
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
    /// Surfaces the backend identity without any secret material: the `Os`
    /// arm reports the SPI `vendor()`/`id()` tags; the `File` arm delegates
    /// to [`EncryptedFileStore`]'s redacting `Debug` (path only).
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
/// [`SecretStoreError`] for the [`Os`](SecretStore::Os) arm. Best-effort:
/// variants map into [`SecretStoreError::OsKeyring`] (non-secret
/// discriminant only) or the closest existing variant; byte-bearing
/// keyring variants are collapsed so their bytes never enter the type.
/// The [`File`](SecretStore::File) arm never reaches this projection.
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
        SecretStore::file(secure_vault_path(dir), SecretString::new("pw-correct")).unwrap()
    }

    /// Tighten the umask-0002 tempdir (0o775) to 0o700 so it passes the
    /// parent-dir perm check, then return a vault path inside it.
    fn secure_vault_path(dir: &std::path::Path) -> std::path::PathBuf {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
        }
        dir.join("vault.pwsvault")
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
        // Resident-vault model verifies the passphrase at open() (header
        // verify-token), so a wrong-pass reopen fails at open(), losslessly.
        let dir = tempfile::tempdir().unwrap();
        file_store(dir.path())
            .set(&wid(1), "seed", &SecretBytes::from_slice(b"orig"))
            .unwrap();
        let err = SecretStore::file(secure_vault_path(dir.path()), SecretString::new("pw-wrong"))
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
        // Corrupt the entry ciphertext but leave the verify-token intact:
        // passphrase still correct, so this is Corruption, not WrongPassphrase.
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
        // A second open() of a path the first store still holds returns
        // AlreadyLocked, losslessly on the public path.
        let dir = tempfile::tempdir().unwrap();
        let path = secure_vault_path(dir.path());
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

    /// The shim must enforce the label allowlist before reaching the OS
    /// backend (per-backend policies differ). A `CredentialStoreApi` that
    /// panics on `build()` proves a bad label is rejected with
    /// `InvalidLabel` before it ever crosses the SPI seam.
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
        // Every OS-arm op goes through `build_os`, so the allowlist
        // rejection fires before the panicking SPI is reached.
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

    // ===== Tier-2 strict fail-closed read =====
    //
    // Parameterised over BOTH arms. The "attacker who can write the
    // backend" is modelled per arm by `Backend::place_raw`: on File it
    // re-seals the chosen blob under the resident vault key via `put_bytes`
    // (a cold/backup-swap actor could only corrupt → DoS, so the strip
    // requires the vault key — the File-arm asymmetry); on Os it overwrites
    // the keychain item directly (the bare envelope, no second AEAD — where
    // the strip residual bites hardest). The writable Os fixture is the
    // upstream `keyring_core::mock::Store` (a raw SPI `set_secret` bypasses
    // the envelope), so no bespoke mock is needed.

    use keyring_core::mock;

    use crate::secrets::file::crypto::{KdfParams, ARGON2_MIN_M_KIB, ARGON2_MIN_T, ARGON2_P};
    use crate::secrets::file::format::KDF_ID_ARGON2ID;

    /// Argon2id floor params — fast enough for these tests.
    fn floor() -> KdfParams {
        KdfParams {
            id: KDF_ID_ARGON2ID,
            m_kib: ARGON2_MIN_M_KIB,
            t: ARGON2_MIN_T,
            p: ARGON2_P,
        }
    }

    fn protected(w: &WalletId, label: &str, pw: &str, secret: &[u8]) -> Vec<u8> {
        envelope::wrap_with_params(w, label, Some(&SecretString::new(pw)), secret, floor())
            .unwrap()
            .expose_secret()
            .to_vec()
    }

    fn unprotected(w: &WalletId, label: &str, secret: &[u8]) -> Vec<u8> {
        envelope::wrap(w, label, None, secret)
            .unwrap()
            .expose_secret()
            .to_vec()
    }

    /// A backend under test plus the raw-write hook that plays the
    /// backend-write attacker.
    struct Backend {
        store: SecretStore,
        _dir: Option<tempfile::TempDir>,
        mock: Option<Arc<mock::Store>>,
        name: &'static str,
    }

    impl Backend {
        /// Write `blob` to `(w, label)` as opaque backend bytes (the
        /// attacker's primitive / the protected-enrol setup). On Os this is
        /// a raw SPI `set_secret` on the shared mock store, bypassing the
        /// `SecretStore` envelope layer exactly as a breached keychain write
        /// would.
        fn place_raw(&self, w: &WalletId, label: &str, blob: &[u8]) {
            match (&self.store, &self.mock) {
                (SecretStore::File(fs), _) => fs
                    .put_bytes(w, label, &SecretBytes::from_slice(blob))
                    .unwrap(),
                (SecretStore::Os(_), Some(mock)) => {
                    let service = format!("{SERVICE_PREFIX}{}", w.to_hex());
                    mock.build(&service, label, None)
                        .unwrap()
                        .set_secret(blob)
                        .unwrap();
                }
                _ => unreachable!("os backend must carry its mock"),
            }
        }
    }

    fn file_backend() -> Backend {
        let dir = tempfile::tempdir().unwrap();
        let store = file_store(dir.path());
        Backend {
            store,
            _dir: Some(dir),
            mock: None,
            name: "File",
        }
    }

    fn os_backend() -> Backend {
        // The upstream in-memory mock store. The clone handed to
        // `SecretStore::Os` and the handle kept for raw attacker writes
        // share the same backing credentials by `Arc`.
        let mock = mock::Store::new().unwrap();
        let store = SecretStore::Os(mock.clone());
        Backend {
            store,
            _dir: None,
            mock: Some(mock),
            name: "Os",
        }
    }

    /// The strict-read quadrant.
    fn run_quadrant(b: &Backend) {
        let w = wid(1);
        let pw = SecretString::new("object-pw");

        // scheme-0 + None → bytes (the ONLY byte-returning quadrant).
        b.place_raw(&w, "u0", &unprotected(&w, "u0", b"plain-seed"));
        assert_eq!(
            b.store
                .get_secret(&w, "u0", None)
                .unwrap()
                .unwrap()
                .expose_secret(),
            b"plain-seed",
            "[{}] scheme-0 + None",
            b.name
        );

        // scheme-1 + None → NeedsPassword (never ciphertext).
        b.place_raw(&w, "p1", &protected(&w, "p1", "object-pw", b"real-seed"));
        assert!(
            matches!(
                b.store.get_secret(&w, "p1", None).unwrap_err(),
                SecretStoreError::NeedsPassword
            ),
            "[{}] scheme-1 + None",
            b.name
        );

        // scheme-1 + Some(correct) → secret.
        assert_eq!(
            b.store
                .get_secret(&w, "p1", Some(&pw))
                .unwrap()
                .unwrap()
                .expose_secret(),
            b"real-seed",
            "[{}] scheme-1 + Some(correct)",
            b.name
        );

        // scheme-1 + Some(wrong) → WrongPassword.
        assert!(
            matches!(
                b.store
                    .get_secret(&w, "p1", Some(&SecretString::new("nope")))
                    .unwrap_err(),
                SecretStoreError::WrongPassword
            ),
            "[{}] scheme-1 + Some(wrong)",
            b.name
        );

        // scheme-0 + Some(pw) → ExpectedProtectedButUnsealed (fail closed).
        assert!(
            matches!(
                b.store.get_secret(&w, "u0", Some(&pw)).unwrap_err(),
                SecretStoreError::ExpectedProtectedButUnsealed
            ),
            "[{}] scheme-0 + Some",
            b.name
        );

        // magic-present-but-truncated + None → Corruption.
        let mut trunc = envelope::MAGIC.to_vec();
        trunc.push(envelope::ENVELOPE_VERSION); // no scheme byte
        b.place_raw(&w, "broken", &trunc);
        assert!(
            matches!(
                b.store.get_secret(&w, "broken", None).unwrap_err(),
                SecretStoreError::Corruption
            ),
            "[{}] truncated-with-magic + None",
            b.name
        );

        // magic-less legacy raw + None → returns the bytes (legacy
        // tolerance); + Some(pw) → fails closed, so the strict rule holds.
        b.place_raw(&w, "legacy", b"raw-legacy-seed-no-magic");
        assert_eq!(
            b.store
                .get_secret(&w, "legacy", None)
                .unwrap()
                .unwrap()
                .expose_secret(),
            b"raw-legacy-seed-no-magic",
            "[{}] legacy magic-less + None",
            b.name
        );
        assert!(
            matches!(
                b.store.get_secret(&w, "legacy", Some(&pw)).unwrap_err(),
                SecretStoreError::ExpectedProtectedButUnsealed
            ),
            "[{}] legacy magic-less + Some",
            b.name
        );

        // absent entry → Ok(None) under either arg (deletion = DoS).
        assert!(b.store.get_secret(&w, "absent", None).unwrap().is_none());
        assert!(b
            .store
            .get_secret(&w, "absent", Some(&pw))
            .unwrap()
            .is_none());
    }

    #[test]
    fn l1_quadrant_file() {
        run_quadrant(&file_backend());
    }

    #[test]
    fn l1_quadrant_os() {
        run_quadrant(&os_backend());
    }

    /// The non-vacuous strip-injection regression. The single
    /// test the whole feature exists to make pass.
    fn run_strip_injection(b: &Backend) {
        let w = wid(2);
        let pw = SecretString::new("object-pw");

        // Enrol protected: stored = a valid scheme-1 envelope of S_real.
        b.place_raw(
            &w,
            "seed",
            &protected(&w, "seed", "object-pw", b"REAL-SEED-S_real"),
        );
        assert_eq!(
            b.store
                .get_secret(&w, "seed", Some(&pw))
                .unwrap()
                .unwrap()
                .expose_secret(),
            b"REAL-SEED-S_real",
            "[{}] legit protected read",
            b.name
        );

        // Attacker overwrites the slot with a fresh, internally-valid
        // scheme-0 envelope carrying a DIFFERENT seed S_evil.
        let attacker_blob = unprotected(&w, "seed", b"EVIL-SEED-S_evil");
        b.place_raw(&w, "seed", &attacker_blob);

        // A password-supplied read of the stripped slot fails closed;
        // S_evil is NEVER returned.
        let err = b.store.get_secret(&w, "seed", Some(&pw)).unwrap_err();
        assert!(
            matches!(err, SecretStoreError::ExpectedProtectedButUnsealed),
            "[{}] strip must fail closed, got {err:?}",
            b.name
        );

        // Non-vacuity: the attacker blob IS a valid unprotected envelope
        // that WOULD decode to S_evil under `None` — so the refusal above is
        // caused SOLELY by the Some(pw)+scheme-0 strict rule, not by any
        // malformation (without the strict rule, S_evil would be returned).
        let would_be = envelope::unwrap(&w, "seed", None, &attacker_blob).unwrap();
        assert_eq!(
            would_be.expose_secret(),
            b"EVIL-SEED-S_evil",
            "[{}] non-vacuity: blob decodes to S_evil under None",
            b.name
        );
    }

    #[test]
    fn l1_strip_injection_file() {
        run_strip_injection(&file_backend());
    }

    #[test]
    fn l1_strip_injection_os() {
        run_strip_injection(&os_backend());
    }

    /// A consumer bug alone fails closed in BOTH directions.
    fn run_both_det_bug_directions(b: &Backend) {
        let w = wid(3);
        let pw = SecretString::new("pw");
        // (a) over-supply a password on a genuinely unprotected object.
        b.place_raw(&w, "u", &unprotected(&w, "u", b"x"));
        assert!(matches!(
            b.store.get_secret(&w, "u", Some(&pw)).unwrap_err(),
            SecretStoreError::ExpectedProtectedButUnsealed
        ));
        // (b) under-supply on a genuinely protected object.
        b.place_raw(&w, "p", &protected(&w, "p", "pw", b"y"));
        assert!(matches!(
            b.store.get_secret(&w, "p", None).unwrap_err(),
            SecretStoreError::NeedsPassword
        ));
    }

    #[test]
    fn l1_both_det_bug_directions_file() {
        run_both_det_bug_directions(&file_backend());
    }

    #[test]
    fn l1_both_det_bug_directions_os() {
        run_both_det_bug_directions(&os_backend());
    }

    /// The expectation is NEVER inferred from the blob's scheme
    /// byte — identical scheme-1 blobs diverge solely on the password arg.
    fn run_expectation_not_inferred(b: &Backend) {
        let w = wid(4);
        let pw = SecretString::new("pw");
        let blob = protected(&w, "a", "pw", b"seed");
        b.place_raw(&w, "a", &blob);
        b.place_raw(&w, "b", &blob);
        assert_eq!(
            b.store
                .get_secret(&w, "a", Some(&pw))
                .unwrap()
                .unwrap()
                .expose_secret(),
            b"seed"
        );
        assert!(matches!(
            b.store.get_secret(&w, "b", None).unwrap_err(),
            SecretStoreError::NeedsPassword
        ));
    }

    #[test]
    fn l1_expectation_not_inferred_file() {
        run_expectation_not_inferred(&file_backend());
    }

    #[test]
    fn l1_expectation_not_inferred_os() {
        run_expectation_not_inferred(&os_backend());
    }

    /// Unprotected→protected upgrade confusion is availability-
    /// only, fail-closed (NeedsPassword), no leak / no injection.
    fn run_upgrade_confusion(b: &Backend) {
        let w = wid(5);
        b.place_raw(&w, "x", &protected(&w, "x", "attacker-pw", b"whatever"));
        assert!(matches!(
            b.store.get_secret(&w, "x", None).unwrap_err(),
            SecretStoreError::NeedsPassword
        ));
    }

    #[test]
    fn l1_upgrade_confusion_file() {
        run_upgrade_confusion(&file_backend());
    }

    #[test]
    fn l1_upgrade_confusion_os() {
        run_upgrade_confusion(&os_backend());
    }

    /// An in-place scheme-byte flip (protected→unprotected). Some(pw) is caught by
    /// the strict rule regardless. None reads the body as scheme-0 opaque
    /// bytes (never the real seed) — a known residual, dominated by the
    /// consumer-DB residual; pinned, not "fixed".
    fn run_scheme_flip(b: &Backend) {
        let w = wid(6);
        let pw = SecretString::new("pw");
        let mut blob = protected(&w, "x", "pw", b"real-seed");
        let scheme_off = envelope::MAGIC.len() + 1;
        assert_eq!(blob[scheme_off], envelope::SCHEME_PASSWORD);
        blob[scheme_off] = envelope::SCHEME_UNPROTECTED;
        b.place_raw(&w, "x", &blob);

        assert!(matches!(
            b.store.get_secret(&w, "x", Some(&pw)).unwrap_err(),
            SecretStoreError::ExpectedProtectedButUnsealed
        ));
        let got = b.store.get_secret(&w, "x", None).unwrap().unwrap();
        assert_ne!(
            got.expose_secret(),
            b"real-seed",
            "the real seed must never surface from a flipped scheme byte"
        );
    }

    #[test]
    fn l1_scheme_flip_file() {
        run_scheme_flip(&file_backend());
    }

    #[test]
    fn l1_scheme_flip_os() {
        run_scheme_flip(&os_backend());
    }

    // ===== Add / change / remove password + arm matrix =====
    //
    // These exercise the PUBLIC set_secret/get_secret/reprotect API, so the
    // protected writes/reads run the real (default 64 MiB) Argon2 — kept to
    // a small number of derivations per test.

    /// The full enrol → change → remove lifecycle, each
    /// step verified through the strict read.
    fn run_pw_lifecycle(b: &Backend) {
        let w = wid(10);
        let pw1 = SecretString::new("pw-one");
        let pw2 = SecretString::new("pw-two");

        // ADD: start unprotected, enrol a password.
        b.store
            .set(&w, "seed", &SecretBytes::from_slice(b"SEED"))
            .unwrap();
        assert_eq!(
            b.store.get(&w, "seed").unwrap().unwrap().expose_secret(),
            b"SEED"
        );
        b.store.reprotect(&w, "seed", None, Some(&pw1)).unwrap();
        assert!(
            matches!(
                b.store.get(&w, "seed").unwrap_err(),
                SecretStoreError::NeedsPassword
            ),
            "[{}] after add, None read needs a password",
            b.name
        );
        assert_eq!(
            b.store
                .get_secret(&w, "seed", Some(&pw1))
                .unwrap()
                .unwrap()
                .expose_secret(),
            b"SEED"
        );

        // CHANGE: rotate to a new password (unwrap-old → rewrap-new).
        b.store
            .reprotect(&w, "seed", Some(&pw1), Some(&pw2))
            .unwrap();
        assert_eq!(
            b.store
                .get_secret(&w, "seed", Some(&pw2))
                .unwrap()
                .unwrap()
                .expose_secret(),
            b"SEED"
        );
        assert!(
            matches!(
                b.store.get_secret(&w, "seed", Some(&pw1)).unwrap_err(),
                SecretStoreError::WrongPassword
            ),
            "[{}] old password no longer unlocks after change",
            b.name
        );

        // REMOVE: back to unprotected.
        b.store.reprotect(&w, "seed", Some(&pw2), None).unwrap();
        assert_eq!(
            b.store.get(&w, "seed").unwrap().unwrap().expose_secret(),
            b"SEED"
        );
        assert!(
            matches!(
                b.store.get_secret(&w, "seed", Some(&pw2)).unwrap_err(),
                SecretStoreError::ExpectedProtectedButUnsealed
            ),
            "[{}] after remove, a password read fails closed until the consumer updates its DB",
            b.name
        );
    }

    #[test]
    fn pw_lifecycle_file() {
        run_pw_lifecycle(&file_backend());
    }

    #[test]
    fn pw_lifecycle_os() {
        run_pw_lifecycle(&os_backend());
    }

    /// Losing the object password bricks the object — no recovery
    /// path exists, every read fails closed.
    fn run_pw_no_recovery(b: &Backend) {
        let w = wid(11);
        let pw = SecretString::new("the-only-pw");
        b.store
            .set_secret(&w, "seed", &SecretBytes::from_slice(b"SEED"), Some(&pw))
            .unwrap();
        assert!(matches!(
            b.store
                .get_secret(&w, "seed", Some(&SecretString::new("guess")))
                .unwrap_err(),
            SecretStoreError::WrongPassword
        ));
        assert!(matches!(
            b.store.get(&w, "seed").unwrap_err(),
            SecretStoreError::NeedsPassword
        ));
    }

    #[test]
    fn pw_no_recovery_file() {
        run_pw_no_recovery(&file_backend());
    }

    #[test]
    fn pw_no_recovery_os() {
        run_pw_no_recovery(&os_backend());
    }

    /// `set`/`get` are additive `..,None` wrappers — `set`
    /// writes a scheme-0 envelope, `get` reads it byte-exact, and a
    /// password-supplied read of that unprotected object fails closed.
    fn run_set_get_wrappers(b: &Backend) {
        let w = wid(12);
        b.store
            .set(&w, "seed", &SecretBytes::from_slice(b"plain"))
            .unwrap();
        assert_eq!(
            b.store.get(&w, "seed").unwrap().unwrap().expose_secret(),
            b"plain"
        );
        assert!(matches!(
            b.store
                .get_secret(&w, "seed", Some(&SecretString::new("pw")))
                .unwrap_err(),
            SecretStoreError::ExpectedProtectedButUnsealed
        ));
    }

    #[test]
    fn set_get_wrappers_file() {
        run_set_get_wrappers(&file_backend());
    }

    #[test]
    fn set_get_wrappers_os() {
        run_set_get_wrappers(&os_backend());
    }

    /// The Os arm has no passphrase concept; the Tier-1 blank
    /// guard never fires and the round-trip is byte-exact.
    #[test]
    fn os_arm_roundtrip_no_blank_guard() {
        let b = os_backend();
        let w = wid(13);
        b.store
            .set(&w, "seed", &SecretBytes::from_slice(b"abc"))
            .unwrap();
        assert_eq!(
            b.store.get(&w, "seed").unwrap().unwrap().expose_secret(),
            b"abc"
        );
        b.store.delete(&w, "seed").unwrap();
        assert!(b.store.get(&w, "seed").unwrap().is_none());
    }

    /// [File]: a crash (disk-write failure) between the unwrap
    /// and the overwrite-commit leaves the OLD protected value intact and
    /// readable — no half-rotated / unprotected state.
    #[cfg(unix)]
    #[test]
    fn pw_change_crash_safety_leaves_old_intact_file() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let s = file_store(dir.path());
        let w = wid(14);
        let old = SecretString::new("old-pw");
        let new = SecretString::new("new-pw");

        s.set_secret(&w, "seed", &SecretBytes::from_slice(b"REAL"), Some(&old))
            .unwrap();

        // Make the vault's parent read-only so the atomic temp-write fails
        // mid-change (mirrors rekey_does_not_corrupt_on_disk_temp_failure).
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o500)).unwrap();
        let err = s.reprotect(&w, "seed", Some(&old), Some(&new)).unwrap_err();
        assert!(matches!(err, SecretStoreError::Io(_)), "got {err:?}");

        // Restore write so the resident store can sync/clean up at drop.
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();

        // The OLD value is still readable under the OLD password; the new
        // password does not unlock it (no half-rotation).
        assert_eq!(
            s.get_secret(&w, "seed", Some(&old))
                .unwrap()
                .unwrap()
                .expose_secret(),
            b"REAL"
        );
        assert!(matches!(
            s.get_secret(&w, "seed", Some(&new)).unwrap_err(),
            SecretStoreError::WrongPassword
        ));
    }

    /// [Os]: a backend failure during the rewrite's write (after the read
    /// succeeds) leaves the OLD value intact — no half-rotation. The mock's
    /// one-shot error injection fails the next write, simulating a crash
    /// mid-rewrite. `reprotect` is read-then-`set_secret`, split here so the
    /// error lands on the write.
    #[test]
    fn os_rewrite_mid_write_failure_leaves_old_intact() {
        let mock = mock::Store::new().unwrap();
        let store = SecretStore::Os(mock.clone());
        let w = wid(15);
        let old = SecretString::new("old-pw");
        let new = SecretString::new("new-pw");
        store
            .set_secret(&w, "seed", &SecretBytes::from_slice(b"REAL"), Some(&old))
            .unwrap();

        // Read succeeds (the rewrite's first step) …
        let secret = store.get_secret(&w, "seed", Some(&old)).unwrap().unwrap();
        // … then inject a one-shot backend error so the write fails.
        let service = format!("{SERVICE_PREFIX}{}", w.to_hex());
        let entry = mock.build(&service, "seed", None).unwrap();
        let cred: &mock::Cred = entry.as_any().downcast_ref().unwrap();
        cred.set_error(KeyringError::PlatformFailure(Box::new(
            std::io::Error::other("simulated backend write failure"),
        )));
        let err = store
            .set_secret(&w, "seed", &secret, Some(&new))
            .unwrap_err();
        assert!(
            matches!(err, SecretStoreError::OsKeyring { .. }),
            "got {err:?}"
        );

        // The OLD value is still readable; nothing rotated to `new`.
        assert_eq!(
            store
                .get_secret(&w, "seed", Some(&old))
                .unwrap()
                .unwrap()
                .expose_secret(),
            b"REAL"
        );
        assert!(matches!(
            store.get_secret(&w, "seed", Some(&new)).unwrap_err(),
            SecretStoreError::WrongPassword
        ));
    }
}
