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
            // Inherent typed path — no lossy SPI seam, no bare buffer.
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
        self.get_raw(service, label)
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
                    Ok(v) => Ok(Some(SecretBytes::new(v))),
                    Err(KeyringError::NoEntry) => Ok(None),
                    Err(e) => Err(map_spi(e)),
                }
            }
        }
    }

    /// Retrieve the secret under `(service, label)` applying the Tier-2
    /// **strict, fail-closed** read (the L-1 keystone), or `Ok(None)` if
    /// absent.
    ///
    /// `password` IS the caller's protection assertion — supply `Some(pw)`
    /// for an object the caller's trusted model says is protected, `None`
    /// otherwise. The expectation lives ONLY here, never in the stored
    /// blob (see [`envelope::unwrap`]):
    ///
    /// - `Some(pw)` + valid scheme-1 → the secret (or
    ///   [`WrongPassword`](SecretStoreError::WrongPassword) on tag fail);
    /// - `Some(pw)` + a non-protected blob (scheme-0 / legacy raw) →
    ///   [`ExpectedProtectedButUnsealed`](SecretStoreError::ExpectedProtectedButUnsealed)
    ///   — a strip/downgrade, refused, no bytes returned ★;
    /// - `None` + scheme-1 →
    ///   [`NeedsPassword`](SecretStoreError::NeedsPassword) (never ciphertext);
    /// - `None` + scheme-0 / legacy raw → the secret.
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

    // ===== Tier-2 strict fail-closed read — the L-1 keystone =====
    //
    // Parameterised over BOTH arms. The "attacker who can write the
    // backend" is modelled per arm by `Backend::place_raw`: on File it
    // re-seals the chosen blob under the resident vault key via `put_bytes`
    // (a cold/backup-swap actor could only corrupt → DoS, so the strip
    // requires the vault key — §8.3 arm asymmetry); on Os it overwrites the
    // mock keychain item directly (the bare envelope, no second AEAD — where
    // the L-1 residual bites hardest, GAP-005 / §8.3).

    use crate::secrets::file::crypto::{KdfParams, ARGON2_MIN_M_KIB, ARGON2_MIN_T, ARGON2_P};
    use crate::secrets::file::format::KDF_ID_ARGON2ID;
    use crate::secrets::testing::InMemoryCredentialStore;

    /// Argon2id floor params — fast enough for the keystone tests.
    fn floor() -> KdfParams {
        KdfParams {
            id: KDF_ID_ARGON2ID,
            m_kib: ARGON2_MIN_M_KIB,
            t: ARGON2_MIN_T,
            p: ARGON2_P,
        }
    }

    fn protected(w: &WalletId, label: &str, pw: &str, secret: &[u8]) -> Vec<u8> {
        envelope::wrap_with_params(w, label, Some(&SecretString::new(pw)), secret, floor()).unwrap()
    }

    fn unprotected(w: &WalletId, label: &str, secret: &[u8]) -> Vec<u8> {
        envelope::wrap(w, label, None, secret).unwrap()
    }

    /// A backend under test plus the raw-write hook that plays the
    /// backend-write attacker.
    struct Backend {
        store: SecretStore,
        _dir: Option<tempfile::TempDir>,
        mock: Option<InMemoryCredentialStore>,
        name: &'static str,
    }

    impl Backend {
        /// Write `blob` to `(w, label)` as opaque backend bytes (the
        /// attacker's primitive / the protected-enrol setup).
        fn place_raw(&self, w: &WalletId, label: &str, blob: &[u8]) {
            match (&self.store, &self.mock) {
                (SecretStore::File(fs), _) => fs
                    .put_bytes(w, label, &SecretBytes::from_slice(blob))
                    .unwrap(),
                (SecretStore::Os(_), Some(mock)) => mock.raw_overwrite(w, label, blob),
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
        let mock = InMemoryCredentialStore::new();
        let store = SecretStore::Os(mock.as_dyn());
        Backend {
            store,
            _dir: None,
            mock: Some(mock),
            name: "Os",
        }
    }

    /// TS-L1-001: the strict-read QUADRANT.
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

        // ★ scheme-0 + Some(pw) → ExpectedProtectedButUnsealed (fail closed).
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

        // magic-less legacy raw + None → bytes (adopted §4.1 contingency;
        // deviates from v4 TS-L1-001's Corruption row). + Some → fail closed.
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

    /// TS-L1-002 ★ — the non-vacuous strip-injection regression. The single
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

        // ★ A password-supplied read of the stripped slot fails closed;
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

    /// TS-L1-003: a DET bug alone fails closed in BOTH directions.
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

    /// TS-L1-004: the expectation is NEVER inferred from the blob's scheme
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

    /// TS-L1-005: unprotected→scheme-1 upgrade confusion is availability-
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

    /// TS-L1-006: an in-place scheme-byte flip (1→0). Some(pw) is caught by
    /// the strict rule regardless. None reads the body as scheme-0 opaque
    /// bytes (never the real seed) — the GAP-010 residual, dominated by the
    /// DET-DB residual; pinned, not "fixed".
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
}
