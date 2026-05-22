//! File-backend error taxonomy and its `keyring_core::Error` projection.
//!
//! One concrete `thiserror` enum, no `#[non_exhaustive]`, **no** secret
//! byte, passphrase, plaintext, or stringified source that could carry
//! one in any variant. `#[error]` strings are static + structural; only
//! non-secret diagnostics (POSIX mode bits, header version int) are
//! carried as typed fields (SEC-REQ-2.0.1 / 2.2.8, CWE-209/CWE-532).
//!
//! The `EncryptedFileStore` surfaces this enum at its construction /
//! `rekey` API; its `keyring_core::api::CredentialApi` /
//! `CredentialStoreApi` impls project it into `keyring_core::Error` via
//! [`From`] so SPI callers see a uniform error. That projection is
//! lossy by design — the structural distinction is preserved on the
//! typed `FileStoreError` path, and only callers reading the raw
//! `keyring_core::Error` see the collapse.

use keyring_core::Error as KeyringError;

/// Errors produced by the `EncryptedFileStore` vault backend.
#[derive(Debug, thiserror::Error)]
pub enum FileStoreError {
    /// AEAD tag failure on the header verify-token: the supplied
    /// passphrase did not unlock the vault. Carries **no** plaintext and
    /// no source (SEC-REQ-2.2.8, CWE-347).
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
    /// understand (SEC-REQ-2.2.9).
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
    /// (SEC-REQ-4.3, CWE-22/CWE-20).
    #[error("invalid label")]
    InvalidLabel,

    /// A pre-existing vault file had permissions looser than `0600`.
    /// Refuse rather than tighten-and-trust (SEC-REQ-2.2.10).
    #[error("vault file has insecure permissions")]
    InsecurePermissions {
        /// The offending POSIX mode bits (not secret).
        mode: u32,
    },

    /// `rekey` was called while an `EncryptedFileCredential` (built via
    /// `CredentialStoreApi::build`) still holds a clone of the inner
    /// `Arc`, so the store lacks the exclusive reference the atomic
    /// passphrase swap requires. A recoverable runtime state — drop the
    /// outstanding credentials and retry — not a logic bug.
    #[error("store is busy: outstanding credentials prevent rekey")]
    Busy,

    /// Internal AEAD tag failure with no vault context yet attached. The
    /// crypto seam (`crypto::open`) cannot tell *why* a tag failed, so it
    /// returns this; callers translate it to [`WrongPassphrase`] (in the
    /// verify-token context) or [`Corruption`] (in an entry context).
    /// Never escapes to the SPI / public surface.
    ///
    /// [`WrongPassphrase`]: FileStoreError::WrongPassphrase
    /// [`Corruption`]: FileStoreError::Corruption
    #[error("decryption/integrity check failed")]
    Decrypt,

    /// Filesystem error (open / write / rename / fsync). The inner
    /// `io::Error` carries an OS code and a path *the caller supplied*,
    /// never a secret.
    #[error("io error")]
    Io(#[from] std::io::Error),
}

impl From<super::super::validate::InvalidLabel> for FileStoreError {
    fn from(_: super::super::validate::InvalidLabel) -> Self {
        Self::InvalidLabel
    }
}

/// Project a [`FileStoreError`] into `keyring_core::Error` for the
/// `CredentialApi` / `CredentialStoreApi` SPI seam.
///
/// The projection is **lossy by design** (the structural distinction
/// lives on the typed `FileStoreError` path):
///
/// - [`WrongPassphrase`] and [`Busy`] ride in
///   [`KeyringError::NoStorageAccess`] (operator UX: "ask the operator
///   to unlock / retry") with the typed error boxed as the source, so an
///   SPI consumer that needs the distinction can still downcast it.
/// - [`Corruption`], [`KdfFailure`], [`VersionUnsupported`],
///   [`MalformedVault`], [`InsecurePermissions`], and the internal
///   [`Decrypt`] collapse into [`KeyringError::BadStoreFormat`] with a
///   static string (Smythe EDIT-2: never secret data in a format error).
/// - [`InvalidLabel`] becomes `KeyringError::Invalid("user", _)`.
/// - [`Io`] becomes [`KeyringError::PlatformFailure`].
///
/// [`WrongPassphrase`]: FileStoreError::WrongPassphrase
/// [`Busy`]: FileStoreError::Busy
/// [`Corruption`]: FileStoreError::Corruption
/// [`KdfFailure`]: FileStoreError::KdfFailure
/// [`VersionUnsupported`]: FileStoreError::VersionUnsupported
/// [`MalformedVault`]: FileStoreError::MalformedVault
/// [`InsecurePermissions`]: FileStoreError::InsecurePermissions
/// [`Decrypt`]: FileStoreError::Decrypt
/// [`InvalidLabel`]: FileStoreError::InvalidLabel
/// [`Io`]: FileStoreError::Io
impl From<FileStoreError> for KeyringError {
    fn from(e: FileStoreError) -> Self {
        use FileStoreError as E;
        match e {
            E::WrongPassphrase | E::Busy => KeyringError::NoStorageAccess(Box::new(e)),
            E::Corruption
            | E::KdfFailure
            | E::VersionUnsupported { .. }
            | E::MalformedVault
            | E::InsecurePermissions { .. }
            | E::Decrypt => KeyringError::BadStoreFormat(e.to_string()),
            E::InvalidLabel => {
                KeyringError::Invalid("user".to_string(), "label allowlist violation".to_string())
            }
            E::Io(io) => KeyringError::PlatformFailure(Box::new(io)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrong_passphrase_and_busy_ride_no_storage_access() {
        for e in [FileStoreError::WrongPassphrase, FileStoreError::Busy] {
            let k: KeyringError = e.into();
            assert!(matches!(k, KeyringError::NoStorageAccess(_)));
        }
    }

    #[test]
    fn corruption_and_format_errors_ride_bad_store_format() {
        for e in [
            FileStoreError::Corruption,
            FileStoreError::Decrypt,
            FileStoreError::KdfFailure,
            FileStoreError::VersionUnsupported { found: 999 },
            FileStoreError::MalformedVault,
            FileStoreError::InsecurePermissions { mode: 0o644 },
        ] {
            let k: KeyringError = e.into();
            assert!(matches!(k, KeyringError::BadStoreFormat(_)));
        }
    }

    #[test]
    fn invalid_label_maps_to_invalid_user() {
        let k: KeyringError = FileStoreError::InvalidLabel.into();
        match k {
            KeyringError::Invalid(attr, _) => assert_eq!(attr, "user"),
            other => panic!("expected Invalid, got {other:?}"),
        }
    }

    #[test]
    fn io_maps_to_platform_failure() {
        let k: KeyringError = FileStoreError::Io(std::io::Error::other("boom")).into();
        assert!(matches!(k, KeyringError::PlatformFailure(_)));
    }

    #[test]
    fn projection_carries_no_secret_in_display() {
        // Corruption / wrong-passphrase render static text only.
        let k: KeyringError = FileStoreError::Corruption.into();
        assert!(!format!("{k}").contains("plaintext"));
        let k: KeyringError = FileStoreError::WrongPassphrase.into();
        assert!(format!("{k:?}").contains("NoStorageAccess"));
    }
}
