//! Secret-store error taxonomy and its `keyring_core::Error` projection.
//!
//! Variants carry only non-secret diagnostics (POSIX mode bits, header
//! version, vault path) — never a secret byte, passphrase, plaintext, or
//! stringified source (CWE-209/CWE-532). The public, fully-typed path is
//! the [`SecretStore`](crate::secrets::SecretStore) API; the SPI
//! projection into `keyring_core::Error` is lossy (see the [`From`] impl).

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

    /// AEAD tag failure on a stored entry (or rekey re-encrypt) *after*
    /// the header verify-token passed: the entry ciphertext is corrupt or
    /// tampered, **not** a wrong passphrase. No plaintext (CWE-347).
    #[error("vault entry failed integrity check (corruption or tampering)")]
    Corruption,

    /// Argon2 key derivation failed. The upstream error carries no useful
    /// non-secret diagnostic, so it is not embedded.
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

    /// The vault file's parent directory was group/other WRITABLE
    /// (`mode & 0o022 != 0`). Directory write governs rename/unlink, so a
    /// writable parent lets another local user swap the vault despite its
    /// own `0600`. Read-only group access (`0o750`) is fine — it leaks
    /// filenames, not the 0600-protected contents.
    #[error("vault parent directory has insecure permissions")]
    InsecureParentDir {
        /// The offending POSIX mode bits on the parent directory (not
        /// secret).
        mode: u32,
    },

    /// A secret offered for storage exceeded the per-secret write cap
    /// ([`MAX_SECRET_LEN`](crate::secrets::MAX_SECRET_LEN)). Rejected at
    /// the write boundary so an oversized entry never inflates the shared
    /// vault past the read-side ceiling and bricks every wallet on reopen.
    #[error("secret exceeds maximum size of {max} bytes (got {found})")]
    SecretTooLarge {
        /// The offered secret length (bytes).
        found: usize,
        /// The compiled-in per-secret ceiling (bytes).
        max: usize,
    },

    /// The vault sidecar (`<vault-path>.lock`) is already held by another
    /// `EncryptedFileStore` handle in this or another process. The
    /// resident-vault model needs exclusive ownership for the store's
    /// lifetime, so a second `open()` fails fast (no retry). Recoverable:
    /// drop the other handle and retry.
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

    /// Internal AEAD tag failure with no vault context attached:
    /// `crypto::open` cannot tell *why* a tag failed, so callers translate
    /// this to [`WrongPassphrase`] (verify-token context) or
    /// [`Corruption`] (entry context). Never escapes to the SPI surface.
    ///
    /// [`WrongPassphrase`]: SecretStoreError::WrongPassphrase
    /// [`Corruption`]: SecretStoreError::Corruption
    #[error("decryption/integrity check failed")]
    Decrypt,

    /// AEAD encrypt-side failure (cipher construction or `encrypt`).
    /// Effectively unreachable — the key is always 32 bytes and plaintext
    /// never approaches XChaCha20's ~256 GiB limit — but kept typed so a
    /// write failure is never mislabeled a [`KdfFailure`].
    ///
    /// [`KdfFailure`]: SecretStoreError::KdfFailure
    #[error("encryption failed")]
    Encrypt,

    /// Filesystem error (open / write / rename / fsync). The inner
    /// [`IoError`] carries an OS code and, when known, the *non-secret*
    /// caller-supplied path — never a secret byte.
    #[error("{0}")]
    Io(#[from] IoError),

    /// An OS-keyring backend ([`SecretStore::Os`] arm) failure, projected
    /// to a non-secret discriminant. Byte-bearing keyring variants
    /// (`BadEncoding`, `BadDataFormat`) collapse to
    /// [`OsKeyringErrorKind::BadStoreFormat`]; their bytes never enter
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
    /// Build an [`Io`](SecretStoreError::Io) error naming the non-secret
    /// `path` the failing operation touched. Use at the read/write/lock
    /// seams; deep helpers can still use the bare `?` (path unknown).
    pub(crate) fn io_at(path: &Path, source: std::io::Error) -> Self {
        Self::Io(IoError {
            path: Some(path.to_path_buf()),
            source,
        })
    }
}

/// Filesystem-error payload for [`SecretStoreError::Io`]. Wraps the OS
/// [`std::io::Error`] plus the non-secret path, when known. A bare `?`
/// works (path `None`); path-aware seams use [`SecretStoreError::io_at`].
#[derive(Debug, thiserror::Error)]
pub struct IoError {
    /// The non-secret caller-supplied path, when the failing operation
    /// knew it.
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
/// from `keyring_core::Error` for the [`SecretStore::Os`] arm. Payload-
/// less, so no secret byte / path / attribute value can ride along.
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

/// Bare `?` on an [`std::io::Error`] threads through [`IoError`] with
/// path `None`; path-aware seams call [`SecretStoreError::io_at`].
impl From<std::io::Error> for SecretStoreError {
    fn from(source: std::io::Error) -> Self {
        Self::Io(IoError::from(source))
    }
}

/// Project a [`SecretStoreError`] into `keyring_core::Error` for the SPI
/// seam. Lossy by design — the lossless typed path is the
/// [`SecretStore`](crate::secrets::SecretStore) API.
///
/// - [`WrongPassphrase`] / [`AlreadyLocked`] ride in
///   [`KeyringError::NoStorageAccess`] with the typed error boxed as the
///   source, recoverable via
///   `err.source().and_then(|s| s.downcast_ref::<SecretStoreError>())`.
/// - The format/crypto group collapses into
///   [`KeyringError::BadStoreFormat`] (a static secret-free string — that
///   variant has no box slot).
/// - [`InvalidLabel`] → `KeyringError::Invalid("user", _)`;
///   [`Io`] → [`KeyringError::PlatformFailure`].
///
/// [`WrongPassphrase`]: SecretStoreError::WrongPassphrase
/// [`AlreadyLocked`]: SecretStoreError::AlreadyLocked
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
            | E::InsecureParentDir { .. }
            | E::SecretTooLarge { .. }
            | E::VaultTooLarge { .. }
            | E::Decrypt
            | E::Encrypt
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
            SecretStoreError::Encrypt,
            SecretStoreError::KdfFailure,
            SecretStoreError::VersionUnsupported { found: 999 },
            SecretStoreError::MalformedVault,
            SecretStoreError::InsecurePermissions { mode: 0o644 },
            SecretStoreError::InsecureParentDir { mode: 0o777 },
            SecretStoreError::SecretTooLarge {
                found: 100,
                max: 10,
            },
            SecretStoreError::VaultTooLarge {
                found: 100,
                max: 10,
            },
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
