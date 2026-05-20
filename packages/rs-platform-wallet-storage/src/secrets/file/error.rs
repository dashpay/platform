//! File-backend-unique error taxonomy.
//!
//! Concrete `thiserror` enum (SEC-REQ-4.4 / TC-082), no
//! `#[non_exhaustive]`, **no** secret byte, passphrase, plaintext, or
//! stringified source that could carry one in any variant. `#[error]`
//! strings are static + structural; only non-secret diagnostics (POSIX
//! mode bits, header version int) are carried as typed fields
//! (SEC-REQ-2.0.1 / 2.2.8, CWE-209/CWE-532).
//!
//! The `EncryptedFileStore` surfaces this enum at its construction /
//! `rekey` API; its `keyring_core::api::CredentialApi` /
//! `CredentialStoreApi` impls bridge it through
//! [`into_keyring`](super::error_bridge::into_keyring) so SPI callers
//! see a uniform `keyring_core::Error`.

/// Errors produced by the `EncryptedFileStore` vault backend.
#[derive(Debug, thiserror::Error)]
pub enum FileStoreError {
    /// AEAD tag verification failed. Carries **no** decrypted-but-
    /// unverified bytes and no source (SEC-REQ-2.2.8, CWE-347).
    #[error("decryption/integrity check failed")]
    Decrypt,

    /// The supplied passphrase did not unlock the vault.
    #[error("wrong passphrase")]
    WrongPassphrase,

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
