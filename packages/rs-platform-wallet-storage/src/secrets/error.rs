//! Secret-store error taxonomy and its `keyring_core::Error` projection.
//!
//! One concrete `thiserror` enum shared by both
//! [`SecretStore`](crate::secrets::SecretStore) backends (the encrypted
//! file vault and the OS keyring), no `#[non_exhaustive]`, **no** secret
//! byte, passphrase, plaintext, or stringified source that could carry
//! one in any variant. `#[error]` strings are static + structural; only
//! non-secret diagnostics (POSIX mode bits, header version int, vault
//! path) are carried as typed fields (CWE-209/CWE-532).
//!
//! The `EncryptedFileStore` surfaces this enum at its construction /
//! `rekey` API; its `keyring_core::api::CredentialApi` /
//! `CredentialStoreApi` impls project it into `keyring_core::Error` via
//! [`From`] so SPI callers see a uniform error. The `WrongPassphrase` /
//! `AlreadyLocked` variants box the typed `SecretStoreError` as the
//! `NoStorageAccess` source, so an SPI consumer can recover them
//! losslessly via `source().downcast_ref::<SecretStoreError>()`; the
//! `BadStoreFormat` group has no box slot and carries only a secret-free
//! string. Either way, the fully typed path is the public
//! [`SecretStore`](crate::secrets::SecretStore) API, which returns
//! `SecretStoreError` directly.

use std::path::Path;

use keyring_core::Error as KeyringError;

/// Errors produced by a [`SecretStore`](crate::secrets::SecretStore) —
/// both the `EncryptedFileStore` vault backend and the OS keyring arm.
#[derive(Debug, thiserror::Error)]
pub enum SecretStoreError {
    /// AEAD tag failure on the header verify-token: the supplied
    /// passphrase did not unlock the vault. Carries **no** plaintext and
    /// no source (CWE-347).
    #[error("wrong passphrase")]
    WrongPassphrase,

    /// AEAD tag failure on a stored entry (or a rekey re-encrypt) *after*
    /// the header verify-token already passed: the entry ciphertext is
    /// corrupt or tampered, **not** a wrong passphrase. Carries no
    /// plaintext (CWE-347).
    #[error("vault entry failed integrity check (corruption or tampering)")]
    Corruption,

    /// Argon2 key derivation failed. The upstream error carries no
    /// useful non-secret diagnostic, so it is intentionally not
    /// embedded.
    #[error("key derivation failed")]
    KdfFailure,

    /// The vault header declared a `format_version` this build does not
    /// understand.
    #[error("unsupported vault format version {found}")]
    VersionUnsupported {
        /// The version byte read from the (authenticated) header.
        found: u32,
    },

    /// The vault file was malformed (bad magic, truncated header, bad
    /// record framing) — no plaintext was produced.
    #[error("malformed vault file")]
    MalformedVault,

    /// `label` failed the `^[A-Za-z0-9._-]{1,64}$` allowlist
    /// (CWE-22/CWE-20).
    #[error("invalid label")]
    InvalidLabel,

    /// A pre-existing vault file had permissions looser than `0600`.
    /// Refuse rather than tighten-and-trust.
    #[error("vault file has insecure permissions")]
    InsecurePermissions {
        /// The offending POSIX mode bits (not secret).
        mode: u32,
    },

    /// The vault sidecar (`<vault-path>.lock`) is already held by
    /// another `EncryptedFileStore` handle — in this process or in
    /// another process. The resident-vault model requires exclusive
    /// ownership of the vault file for the store's lifetime, so the
    /// second `open()` fails fast (no retry, no wait budget). Drop the
    /// other handle, or wait for the other process to exit, and retry.
    /// A recoverable runtime state, not a logic bug.
    #[error("vault is already locked by another store handle")]
    AlreadyLocked,

    /// The on-disk vault file exceeds the structural ceiling
    /// ([`MAX_VAULT_SIZE_BYTES`](crate::secrets::MAX_VAULT_SIZE_BYTES)).
    /// Refuse to allocate / parse a multi-GiB attacker-controllable JSON
    /// payload.
    #[error("vault file exceeds maximum size of {max} bytes (got {found})")]
    VaultTooLarge {
        /// The on-disk size (bytes) of the offending file.
        found: u64,
        /// The compiled-in ceiling (bytes).
        max: u64,
    },

    /// Internal AEAD tag failure with no vault context yet attached. The
    /// crypto seam (`crypto::open`) cannot tell *why* a tag failed, so it
    /// returns this; callers translate it to [`WrongPassphrase`] (in the
    /// verify-token context) or [`Corruption`] (in an entry context).
    /// Never escapes to the SPI / public surface.
    ///
    /// [`WrongPassphrase`]: SecretStoreError::WrongPassphrase
    /// [`Corruption`]: SecretStoreError::Corruption
    #[error("decryption/integrity check failed")]
    Decrypt,

    /// Filesystem error (open / write / rename / fsync). The inner
    /// [`IoError`] carries an OS code and, when the failing operation
    /// knew it, the *non-secret* path it was operating on — a
    /// caller-supplied filesystem path, never a secret byte.
    #[error("{0}")]
    Io(#[from] IoError),

    /// An OS-keyring backend (the [`SecretStore::Os`] arm) failure,
    /// projected to a non-secret discriminant. Keyring variants that
    /// carry raw bytes (`BadEncoding`, `BadDataFormat`) are collapsed to
    /// [`OsKeyringErrorKind::BadStoreFormat`] — their bytes never enter
    /// this type (CWE-209/CWE-532).
    ///
    /// [`SecretStore::Os`]: crate::secrets::SecretStore::Os
    #[error("os keyring error: {kind}")]
    OsKeyring {
        /// The non-secret keyring failure discriminant.
        kind: OsKeyringErrorKind,
    },
}

impl SecretStoreError {
    /// Build an [`Io`](SecretStoreError::Io) error that names the
    /// non-secret filesystem `path` the failing operation touched.
    /// Use at the vault read / write / lock seams where the path is
    /// known; the bare `?`/`From<std::io::Error>` conversion (path
    /// unknown) stays available for the deep helpers.
    pub(crate) fn io_at(path: &Path, source: std::io::Error) -> Self {
        Self::Io(IoError {
            path: Some(path.to_path_buf()),
            source,
        })
    }
}

/// Filesystem-error payload for [`SecretStoreError::Io`]. Wraps the OS
/// [`std::io::Error`] and, when the failing operation knew it, the
/// non-secret path it was operating on. `From<std::io::Error>` is
/// derived so a bare `?` still works (path defaults to `None`); the
/// path-aware seams attach it via [`SecretStoreError::io_at`].
#[derive(Debug, thiserror::Error)]
pub struct IoError {
    /// The non-secret filesystem path, when the failing operation knew
    /// it. A caller-supplied path, never a secret.
    pub path: Option<std::path::PathBuf>,
    /// The underlying OS error.
    pub source: std::io::Error,
}

impl std::fmt::Display for IoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.path {
            Some(p) => write!(f, "io error at {}: {}", p.display(), self.source),
            None => write!(f, "io error: {}", self.source),
        }
    }
}

impl From<std::io::Error> for IoError {
    fn from(source: std::io::Error) -> Self {
        Self { path: None, source }
    }
}

/// Non-secret discriminant for an OS-keyring backend failure, projected
/// from `keyring_core::Error` for the [`SecretStore::Os`] arm. Carries no
/// payload, so no secret byte, path, or attribute value can ride along.
///
/// [`SecretStore::Os`]: crate::secrets::SecretStore::Os
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsKeyringErrorKind {
    /// `keyring_core::Error::NoEntry`.
    NoEntry,
    /// `keyring_core::Error::NoStorageAccess` (store locked / inaccessible).
    NoStorageAccess,
    /// `keyring_core::Error::NoDefaultStore` (no reachable backend).
    NoDefaultStore,
    /// A store-format failure (`BadStoreFormat` / `BadEncoding` /
    /// `BadDataFormat`); any raw bytes are dropped at the seam.
    BadStoreFormat,
    /// Any other backend failure (`PlatformFailure`, `TooLong`,
    /// `Ambiguous`, `NotSupportedByStore`).
    Backend,
}

impl std::fmt::Display for OsKeyringErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::NoEntry => "no entry",
            Self::NoStorageAccess => "storage inaccessible",
            Self::NoDefaultStore => "no default store",
            Self::BadStoreFormat => "bad store format",
            Self::Backend => "backend failure",
        };
        f.write_str(s)
    }
}

impl From<super::validate::InvalidLabel> for SecretStoreError {
    fn from(_: super::validate::InvalidLabel) -> Self {
        Self::InvalidLabel
    }
}

/// Bare `?` on a [`std::io::Error`] inside a function returning
/// [`SecretStoreError`] threads through [`IoError`] (path `None`); the
/// path-aware seams call [`SecretStoreError::io_at`] instead.
impl From<std::io::Error> for SecretStoreError {
    fn from(source: std::io::Error) -> Self {
        Self::Io(IoError::from(source))
    }
}

/// Project a [`SecretStoreError`] into `keyring_core::Error` for the
/// `CredentialApi` / `CredentialStoreApi` SPI seam.
///
/// - [`WrongPassphrase`] and [`AlreadyLocked`] ride in
///   [`KeyringError::NoStorageAccess`] (operator UX: "ask the operator to
///   unlock / retry") with the typed `SecretStoreError` boxed as the
///   source, so an SPI consumer can losslessly recover the variant via
///   `err.source().and_then(|s| s.downcast_ref::<SecretStoreError>())`.
/// - [`Corruption`], [`KdfFailure`], [`VersionUnsupported`],
///   [`MalformedVault`], [`InsecurePermissions`], the internal
///   [`Decrypt`], and [`OsKeyring`] collapse into
///   [`KeyringError::BadStoreFormat`], whose `String` payload has no box
///   slot, so they carry only a static secret-free string (never secret
///   data in a format error). They remain losslessly typed on the
///   [`SecretStore`](crate::secrets::SecretStore) path.
/// - [`InvalidLabel`] becomes `KeyringError::Invalid("user", _)`.
/// - [`Io`] becomes [`KeyringError::PlatformFailure`].
///
/// [`WrongPassphrase`]: SecretStoreError::WrongPassphrase
/// [`AlreadyLocked`]: SecretStoreError::AlreadyLocked
/// [`Corruption`]: SecretStoreError::Corruption
/// [`KdfFailure`]: SecretStoreError::KdfFailure
/// [`VersionUnsupported`]: SecretStoreError::VersionUnsupported
/// [`MalformedVault`]: SecretStoreError::MalformedVault
/// [`InsecurePermissions`]: SecretStoreError::InsecurePermissions
/// [`Decrypt`]: SecretStoreError::Decrypt
/// [`OsKeyring`]: SecretStoreError::OsKeyring
/// [`InvalidLabel`]: SecretStoreError::InvalidLabel
/// [`Io`]: SecretStoreError::Io
impl From<SecretStoreError> for KeyringError {
    fn from(e: SecretStoreError) -> Self {
        use SecretStoreError as E;
        match e {
            E::WrongPassphrase | E::AlreadyLocked => KeyringError::NoStorageAccess(Box::new(e)),
            E::Corruption
            | E::KdfFailure
            | E::VersionUnsupported { .. }
            | E::MalformedVault
            | E::InsecurePermissions { .. }
            | E::VaultTooLarge { .. }
            | E::Decrypt
            | E::OsKeyring { .. } => KeyringError::BadStoreFormat(e.to_string()),
            E::InvalidLabel => {
                KeyringError::Invalid("user".to_string(), "label allowlist violation".to_string())
            }
            E::Io(io) => KeyringError::PlatformFailure(Box::new(io.source)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrong_passphrase_and_already_locked_ride_no_storage_access() {
        for e in [
            SecretStoreError::WrongPassphrase,
            SecretStoreError::AlreadyLocked,
        ] {
            let k: KeyringError = e.into();
            assert!(matches!(k, KeyringError::NoStorageAccess(_)));
        }
    }

    #[test]
    fn corruption_and_format_errors_ride_bad_store_format() {
        for e in [
            SecretStoreError::Corruption,
            SecretStoreError::Decrypt,
            SecretStoreError::KdfFailure,
            SecretStoreError::VersionUnsupported { found: 999 },
            SecretStoreError::MalformedVault,
            SecretStoreError::InsecurePermissions { mode: 0o644 },
        ] {
            let k: KeyringError = e.into();
            assert!(matches!(k, KeyringError::BadStoreFormat(_)));
        }
    }

    #[test]
    fn invalid_label_maps_to_invalid_user() {
        let k: KeyringError = SecretStoreError::InvalidLabel.into();
        match k {
            KeyringError::Invalid(attr, _) => assert_eq!(attr, "user"),
            other => panic!("expected Invalid, got {other:?}"),
        }
    }

    #[test]
    fn io_maps_to_platform_failure() {
        let k: KeyringError = SecretStoreError::from(std::io::Error::other("boom")).into();
        assert!(matches!(k, KeyringError::PlatformFailure(_)));
    }

    #[test]
    fn io_at_names_path_in_display_without_leaking_secret() {
        // The path-aware Io error renders the offending path so operators
        // can see which file failed; the source message rides along, but
        // no secret byte does (the path is caller-supplied).
        let err = SecretStoreError::io_at(
            std::path::Path::new("/var/lib/wallet/vault.pwsvault"),
            std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
        );
        let rendered = err.to_string();
        assert!(
            rendered.contains("/var/lib/wallet/vault.pwsvault"),
            "expected the path in the message, got {rendered:?}"
        );
        assert!(rendered.contains("denied"));
    }

    #[test]
    fn bare_io_conversion_has_no_path() {
        let err: SecretStoreError = std::io::Error::other("boom").into();
        let SecretStoreError::Io(io) = err else {
            panic!("expected Io variant");
        };
        assert!(io.path.is_none());
    }

    #[test]
    fn projection_carries_no_secret_in_display() {
        // Corruption / wrong-passphrase render static text only.
        let k: KeyringError = SecretStoreError::Corruption.into();
        assert!(!format!("{k}").contains("plaintext"));
        let k: KeyringError = SecretStoreError::WrongPassphrase.into();
        assert!(format!("{k:?}").contains("NoStorageAccess"));
    }

    #[test]
    fn wrong_passphrase_is_recoverable_from_no_storage_access_source() {
        // WrongPassphrase / AlreadyLocked box the typed SecretStoreError
        // as the NoStorageAccess source, so an SPI consumer recovers the
        // variant losslessly via `source().downcast_ref::<SecretStoreError>()`.
        use std::error::Error as _;
        for original in [
            SecretStoreError::WrongPassphrase,
            SecretStoreError::AlreadyLocked,
        ] {
            let want = original.to_string();
            let k: KeyringError = original.into();
            let recovered = k
                .source()
                .and_then(|s| s.downcast_ref::<SecretStoreError>());
            assert!(
                matches!(recovered, Some(e) if e.to_string() == want),
                "expected recoverable {want}, got {recovered:?}"
            );
        }
    }

    #[test]
    fn bad_store_format_group_renders_secret_free_string() {
        use std::error::Error as _;
        let k: KeyringError = SecretStoreError::Corruption.into();
        // No box slot on BadStoreFormat: a static, secret-free message,
        // nothing to downcast.
        assert!(matches!(&k, KeyringError::BadStoreFormat(s) if !s.is_empty()));
        assert!(k.source().is_none());
        assert!(!format!("{k}").contains("plaintext"));
    }

    #[test]
    fn os_keyring_projects_to_bad_store_format() {
        let k: KeyringError = SecretStoreError::OsKeyring {
            kind: OsKeyringErrorKind::NoDefaultStore,
        }
        .into();
        assert!(matches!(k, KeyringError::BadStoreFormat(_)));
    }
}
