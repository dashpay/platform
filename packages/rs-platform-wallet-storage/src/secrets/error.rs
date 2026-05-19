//! Typed errors for the `SecretStore` backends.
//!
//! Concrete `thiserror` enum — no boxed dynamic error trait object
//! (SEC-REQ-4.4 / TC-082), no `#[non_exhaustive]` (prior project
//! decision), and **no** secret byte, passphrase, plaintext, or
//! stringified source that could carry one in any variant.
//! `#[error("...")]` strings are static and structural; only
//! non-secret diagnostics (a permission `mode`, a format `found`
//! version) are carried as typed fields (SEC-REQ-2.0.1 / 2.2.8,
//! CWE-209/CWE-532).

/// Errors returned by [`SecretStore`](super::SecretStore) backends.
///
/// Variant taxonomy lets a caller distinguish "no secure backend, ask
/// the operator" from "wrong passphrase, re-prompt" without ever
/// inspecting a secret.
#[derive(Debug, thiserror::Error)]
pub enum SecretStoreError {
    /// No secure OS keyring is reachable (headless / no Secret Service /
    /// no D-Bus session). Fail closed — never degrade to plaintext.
    #[error("secret backend unavailable")]
    BackendUnavailable,

    /// The OS keyring exists but its collection is locked.
    #[error("keyring is locked")]
    KeyringLocked,

    /// No secret stored under the requested `(wallet_id, label)`.
    #[error("secret not found")]
    NotFound,

    /// AEAD tag verification failed. Carries **no** decrypted-but-
    /// unverified bytes and no source (SEC-REQ-2.2.8, CWE-347).
    #[error("decryption/integrity check failed")]
    Decrypt,

    /// The supplied passphrase did not unlock the vault.
    #[error("wrong passphrase")]
    WrongPassphrase,

    /// `label` failed the `^[A-Za-z0-9._-]{1,64}$` allowlist
    /// (SEC-REQ-4.3, CWE-22/CWE-20).
    #[error("invalid label")]
    InvalidLabel,

    /// Filesystem error (open / write / rename / fsync). The inner
    /// `io::Error` carries an OS code and a path *the caller supplied*,
    /// never a secret.
    #[error("io error")]
    Io(#[from] std::io::Error),

    /// Argon2 key derivation failed. The upstream error carries no
    /// useful non-secret diagnostic, so it is intentionally not
    /// embedded (SEC-REQ-2.2.8).
    #[error("key derivation failed")]
    KdfFailure,

    /// The vault header declared a `format_version` this build does not
    /// understand. Refuse, fail closed (SEC-REQ-2.2.9).
    #[error("unsupported vault format version {found}")]
    VersionUnsupported {
        /// The version byte read from the (authenticated) header.
        found: u32,
    },

    /// The vault file was malformed (bad magic, truncated header, bad
    /// record framing) — no plaintext was produced.
    #[error("malformed vault file")]
    MalformedVault,

    /// A pre-existing vault file had permissions looser than `0600`.
    /// Refuse rather than tighten-and-trust (SEC-REQ-2.2.10).
    #[error("vault file has insecure permissions")]
    InsecurePermissions {
        /// The offending POSIX mode bits (not secret).
        mode: u32,
    },
}
