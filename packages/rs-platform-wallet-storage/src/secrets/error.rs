//! Secret-store error taxonomy and its `keyring_core::Error` projection.
//!
//! Variants carry only non-secret diagnostics (POSIX mode bits, header
//! version, vault path) — never a secret byte, passphrase, or plaintext
//! (CWE-209/CWE-532). The single carried source is the [`Io`] variant's
//! OS error (an errno plus the non-secret caller-supplied path); every
//! other variant is source-free so a crypto/format failure can't stringify
//! a secret. The public, fully-typed path is the
//! [`SecretStore`](crate::secrets::SecretStore) API; the SPI projection into
//! `keyring_core::Error` is lossy (see the [`From`] impl).
//!
//! [`Io`]: SecretStoreError::Io

use std::path::{Path, PathBuf};

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

    /// Tier-2 strip/downgrade guard: the caller asserted — by supplying
    /// an object password — that this object MUST be password-protected,
    /// but the stored value is a well-formed UNPROTECTED envelope
    /// (scheme-0), i.e. a strip/downgrade. **Fails closed:** the stored
    /// bytes are NEVER returned (CWE-757/CWE-345).
    #[error("expected a password-protected secret but the stored value is unprotected")]
    ExpectedProtectedButUnsealed,

    /// Tier-2: a valid password-protected (scheme-1) envelope was read
    /// with NO object password supplied. Never returns ciphertext.
    #[error("secret is password-protected; a password is required")]
    NeedsPassword,

    /// Tier-2: the object password failed the envelope's AEAD tag. Carries
    /// **no** plaintext and no source (CWE-347). Distinct from
    /// [`WrongPassphrase`] (the Tier-1 vault passphrase). On the
    /// [`SecretStore::Os`] arm a tag failure may also indicate keychain
    /// corruption rather than a wrong password — documented in
    /// `SECRETS.md`; one AEAD tag cannot disambiguate the two.
    ///
    /// [`WrongPassphrase`]: SecretStoreError::WrongPassphrase
    /// [`SecretStore::Os`]: crate::secrets::SecretStore::Os
    #[error("wrong object password")]
    WrongPassword,

    /// A vault passphrase (Tier-1 `open`/`rekey`) or an object password
    /// (Tier-2 enrol/unwrap) was shorter than [`MIN_PASSPHRASE_LEN`] after
    /// trimming. CWE-521.
    ///
    /// Neutral wording: the variant covers both Tier-1 vault passphrases and
    /// Tier-2 per-object passwords; the caller's context determines which.
    /// Tier-1 callers wanting a deliberately keyless vault should use
    /// [`EncryptedFileStore::open_unprotected`](crate::secrets::EncryptedFileStore::open_unprotected).
    ///
    /// [`MIN_PASSPHRASE_LEN`]: crate::secrets::MIN_PASSPHRASE_LEN
    #[error("passphrase or password is blank or too short")]
    BlankPassphrase,

    /// A vault passphrase (Tier-1 `open`/`rekey`) or an object password
    /// (Tier-2 enrol/unwrap) was longer than [`MAX_PASSPHRASE_LEN`].
    ///
    /// Passphrases live in guarded, `mlock`ed pages for as long as the
    /// store they unlock, and up to three are resident at once during a
    /// re-protect, so an unbounded one would blow the crate's
    /// locked-memory budget (documented at
    /// [`MAX_SECRET_LEN`](crate::secrets::MAX_SECRET_LEN)). The ceiling is
    /// far above any human-typed passphrase; only a programmatic or
    /// config-supplied value realistically reaches it. Carries lengths
    /// only, never any part of the value (CWE-209).
    ///
    /// [`MAX_PASSPHRASE_LEN`]: crate::secrets::MAX_PASSPHRASE_LEN
    #[error("passphrase exceeds maximum length of {max} bytes (got {found})")]
    PassphraseTooLong {
        /// Length of the offending passphrase, in bytes.
        found: usize,
        /// The enforced ceiling, in bytes.
        max: usize,
    },

    /// AEAD tag failure on a stored entry (or rekey re-encrypt) *after*
    /// the header verify-token passed: the entry ciphertext is corrupt or
    /// tampered, **not** a wrong passphrase. No plaintext (CWE-347).
    #[error("vault entry failed integrity check (corruption or tampering)")]
    Corruption,

    /// Argon2 key derivation failed. The upstream error carries no useful
    /// non-secret diagnostic, so it is not embedded.
    #[error("key derivation failed")]
    KdfFailure,

    /// The OS CSPRNG (`getrandom`) could not supply entropy for a salt,
    /// nonce, or key draw. The upstream error carries no useful non-secret
    /// diagnostic, so it is not embedded. Kept distinct from
    /// [`KdfFailure`] so an exhausted/blocked entropy source is not
    /// misdiagnosed as an Argon2 parameter problem — the CSPRNG backs the
    /// nonce and salt draws too, not just key derivation.
    ///
    /// [`KdfFailure`]: SecretStoreError::KdfFailure
    #[error("system entropy source unavailable")]
    EntropyUnavailable,

    /// The vault header declared a `format_version` this build does not
    /// understand.
    #[error("unsupported vault format version {found}")]
    VersionUnsupported {
        /// The version byte read from the (authenticated) header.
        found: u32,
    },

    /// A Tier-2 secret envelope decoded with a `version` this build does
    /// not understand. Fails closed REGARDLESS of the password argument
    /// — an unparseable future format can be neither safely unwrapped
    /// nor safely treated as unprotected, so it is refused both ways.
    /// Mirrors [`VersionUnsupported`] for the vault format.
    ///
    /// [`VersionUnsupported`]: SecretStoreError::VersionUnsupported
    #[error("unsupported secret envelope version {found}")]
    UnsupportedEnvelopeVersion {
        /// The full `version` field read from the (unauthenticated)
        /// envelope header. `u32` to match `Envelope.version` — a truncating
        /// `u8` would alias distinct out-of-range versions in diagnostics.
        found: u32,
    },

    /// The vault file was malformed (bad magic, truncated header, bad
    /// record framing) — no plaintext was produced.
    #[error("malformed vault file")]
    MalformedVault,

    /// `label` failed the `^[A-Za-z0-9._-]{1,64}$` allowlist
    /// (CWE-22/CWE-20).
    #[error("invalid secret label; expected ^[A-Za-z0-9._-]{{1,64}}$")]
    InvalidLabel,

    /// No credential exists under `(service, label)` on either arm. Returned
    /// by mutators that need an entry to operate on (e.g. [`reprotect`]) so
    /// absence is a signal, not a silent no-op — caller's protection-status
    /// record disagreeing with the backend must not be swallowed. Surfaced
    /// by the file arm when `delete_bytes` reports `Ok(false)` and by the
    /// OS arm when [`keyring_core::Error::NoEntry`] bubbles out.
    ///
    /// [`reprotect`]: crate::secrets::SecretStore::reprotect
    #[error("secret was not found")]
    NoEntry,

    /// The host's memory pages are larger than the crate's locked-memory
    /// budget assumes, so no store can honour that budget here (CWE-316).
    ///
    /// `memsec` rounds every guarded allocation up to the page size it
    /// reads from the kernel at run time, while the budget documented at
    /// [`MAX_SECRET_LEN`](crate::secrets::MAX_SECRET_LEN) is denominated
    /// in 16 KiB pages. On a larger-paged host the real peak exceeds the
    /// budgeted one by the ratio between the two sizes; `mlock` then
    /// fails open with a warning and seed / xpriv material silently
    /// becomes swappable. Construction refuses instead of degrading.
    ///
    /// Reserved for exotic hosts — 64 KiB-page aarch64 RHEL/SLES builds.
    /// 4 KiB Linux and 16 KiB Apple Silicon / iOS both pass.
    ///
    /// Smaller-than-assumed pages are accepted: they turn every
    /// `locked_cost` figure into an over-estimate, which leaves the budget
    /// conservative rather than overrun.
    #[error(
        "host memory pages are {found} bytes but locked secret memory is budgeted for {assumed}; \
         secret pages would exceed RLIMIT_MEMLOCK and silently become swappable — \
         run this process on a host with {assumed}-byte memory pages"
    )]
    HostPageSizeExceedsBudget {
        /// The page size this host reported (not secret).
        found: usize,
        /// The page size the compiled-in budget assumes (not secret).
        assumed: usize,
    },

    /// A pre-existing vault file had permissions looser than `0600`.
    /// Refuse rather than tighten-and-trust.
    #[error(
        "vault file at {path} has mode {mode:04o}; it must be 0600 — run `chmod 600 {path}`",
        path = .path.display()
    )]
    InsecurePermissions {
        /// The vault path (not secret).
        path: PathBuf,
        /// The offending POSIX mode bits (not secret).
        mode: u32,
    },

    /// A pre-existing vault file is owned by a user other than the process's
    /// effective user. Refuse rather than trust a file another user controls.
    #[error(
        "vault file at {path} is owned by another user (uid {found}); change its owner to the current uid {expected}",
        path = .path.display()
    )]
    InsecureOwnership {
        /// The vault path (not secret).
        path: PathBuf,
        /// The file owner's uid.
        found: u32,
        /// The process's effective uid.
        expected: u32,
    },

    /// A vault ancestor was writable without the sticky bit or owned by
    /// neither the current user nor root. Either condition can allow another
    /// local user to replace the vault despite its own `0600` mode.
    #[error(
        "vault parent path {path} traverses an insecure ancestor with mode {mode:04o}; ensure every ancestor is owned by the current user or root, and run `chmod go-w` on the offending ancestor unless it is an intentional sticky shared directory",
        path = .path.display()
    )]
    InsecureParentDir {
        /// The vault's parent path (not secret).
        path: PathBuf,
        /// The offending POSIX mode bits on the ancestor directory (not secret).
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

    /// `true` when the failure clears on a retry after the caller acts on
    /// it. Mirrors `WalletStorageError::is_transient` on this crate's
    /// SQLite arm so the two typed errors read as one family.
    ///
    /// Only [`AlreadyLocked`](Self::AlreadyLocked) qualifies: drop the
    /// other store handle and re-`open`. Every other variant is a
    /// wrong-credential, malformed-input, crypto, permission, size, or I/O
    /// failure a bare retry cannot fix (a failing CSPRNG or disk may
    /// recover, but not through this store's own retry contract).
    ///
    /// The match is wildcard-free so a new variant forces an explicit
    /// classification here.
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::AlreadyLocked => true,
            Self::WrongPassphrase
            | Self::ExpectedProtectedButUnsealed
            | Self::NeedsPassword
            | Self::WrongPassword
            | Self::BlankPassphrase
            | Self::PassphraseTooLong { .. }
            | Self::Corruption
            | Self::KdfFailure
            | Self::EntropyUnavailable
            | Self::VersionUnsupported { .. }
            | Self::UnsupportedEnvelopeVersion { .. }
            | Self::MalformedVault
            | Self::InvalidLabel
            | Self::NoEntry
            | Self::HostPageSizeExceedsBudget { .. }
            | Self::InsecurePermissions { .. }
            | Self::InsecureOwnership { .. }
            | Self::InsecureParentDir { .. }
            | Self::SecretTooLarge { .. }
            | Self::VaultTooLarge { .. }
            | Self::Decrypt
            | Self::Encrypt
            | Self::Io(_)
            | Self::OsKeyring { .. } => false,
        }
    }

    /// Short, lowercase, snake_case tag per variant for tracing fields —
    /// stable and greppable, mirroring `WalletStorageError::error_kind_str`
    /// on this crate's SQLite arm. Match on this, never on the
    /// human-facing `Display`/`Debug` text (documented unstable).
    pub fn error_kind_str(&self) -> &'static str {
        match self {
            Self::WrongPassphrase => "wrong_passphrase",
            Self::ExpectedProtectedButUnsealed => "expected_protected_but_unsealed",
            Self::NeedsPassword => "needs_password",
            Self::WrongPassword => "wrong_password",
            Self::BlankPassphrase => "blank_passphrase",
            Self::PassphraseTooLong { .. } => "passphrase_too_long",
            Self::Corruption => "corruption",
            Self::KdfFailure => "kdf_failure",
            Self::EntropyUnavailable => "entropy_unavailable",
            Self::VersionUnsupported { .. } => "version_unsupported",
            Self::UnsupportedEnvelopeVersion { .. } => "unsupported_envelope_version",
            Self::MalformedVault => "malformed_vault",
            Self::InvalidLabel => "invalid_label",
            Self::NoEntry => "no_entry",
            Self::HostPageSizeExceedsBudget { .. } => "host_page_size_exceeds_budget",
            Self::InsecurePermissions { .. } => "insecure_permissions",
            Self::InsecureOwnership { .. } => "insecure_ownership",
            Self::InsecureParentDir { .. } => "insecure_parent_dir",
            Self::SecretTooLarge { .. } => "secret_too_large",
            Self::AlreadyLocked => "already_locked",
            Self::VaultTooLarge { .. } => "vault_too_large",
            Self::Decrypt => "decrypt",
            Self::Encrypt => "encrypt",
            Self::Io(_) => "io",
            Self::OsKeyring { .. } => "os_keyring",
        }
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
/// - [`WrongPassphrase`] / [`AlreadyLocked`] and the Tier-2 credential /
///   protection states ([`NeedsPassword`], [`WrongPassword`],
///   [`ExpectedProtectedButUnsealed`], [`BlankPassphrase`]) ride in
///   [`KeyringError::NoStorageAccess`] with the typed error boxed as the
///   source, recoverable via
///   `err.source().and_then(|s| s.downcast_ref::<SecretStoreError>())`.
///   These are all "the caller must act on a credential/expectation to
///   proceed" states, so lossless recovery lets an SPI consumer react
///   precisely.
/// - The format/crypto group — including [`UnsupportedEnvelopeVersion`]
///   (a fail-closed forward-format incompatibility, mirroring
///   [`VersionUnsupported`]) — collapses into
///   [`KeyringError::BadStoreFormat`] (a static secret-free string — that
///   variant has no box slot).
/// - [`InvalidLabel`] → `KeyringError::Invalid("user", _)`;
///   [`Io`] and [`HostPageSizeExceedsBudget`] (a host the crate cannot run
///   on, not a store-format problem) → [`KeyringError::PlatformFailure`].
///
/// [`WrongPassphrase`]: SecretStoreError::WrongPassphrase
/// [`AlreadyLocked`]: SecretStoreError::AlreadyLocked
/// [`NeedsPassword`]: SecretStoreError::NeedsPassword
/// [`WrongPassword`]: SecretStoreError::WrongPassword
/// [`ExpectedProtectedButUnsealed`]: SecretStoreError::ExpectedProtectedButUnsealed
/// [`BlankPassphrase`]: SecretStoreError::BlankPassphrase
/// [`UnsupportedEnvelopeVersion`]: SecretStoreError::UnsupportedEnvelopeVersion
/// [`VersionUnsupported`]: SecretStoreError::VersionUnsupported
/// [`InvalidLabel`]: SecretStoreError::InvalidLabel
/// [`Io`]: SecretStoreError::Io
/// [`HostPageSizeExceedsBudget`]: SecretStoreError::HostPageSizeExceedsBudget
impl From<SecretStoreError> for KeyringError {
    fn from(e: SecretStoreError) -> Self {
        use SecretStoreError as E;
        match e {
            E::WrongPassphrase
            | E::AlreadyLocked
            | E::NeedsPassword
            | E::WrongPassword
            | E::ExpectedProtectedButUnsealed
            | E::BlankPassphrase => KeyringError::NoStorageAccess(Box::new(e)),
            E::Corruption
            | E::KdfFailure
            | E::EntropyUnavailable
            | E::VersionUnsupported { .. }
            | E::UnsupportedEnvelopeVersion { .. }
            | E::MalformedVault
            | E::InsecurePermissions { .. }
            | E::InsecureOwnership { .. }
            | E::InsecureParentDir { .. }
            | E::SecretTooLarge { .. }
            | E::PassphraseTooLong { .. }
            | E::VaultTooLarge { .. }
            | E::Decrypt
            | E::Encrypt
            | E::OsKeyring { .. } => KeyringError::BadStoreFormat(e.to_string()),
            E::InvalidLabel => {
                KeyringError::Invalid("user".to_string(), "label allowlist violation".to_string())
            }
            E::NoEntry => KeyringError::NoEntry,
            E::HostPageSizeExceedsBudget { .. } => KeyringError::PlatformFailure(Box::new(e)),
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
            SecretStoreError::InsecurePermissions {
                path: "/vault".into(),
                mode: 0o644,
            },
            SecretStoreError::InsecureOwnership {
                path: "/vault".into(),
                found: 1001,
                expected: 1000,
            },
            SecretStoreError::InsecureParentDir {
                path: "/parent".into(),
                mode: 0o777,
            },
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
    fn validation_and_permission_errors_are_actionable() {
        let file = SecretStoreError::InsecurePermissions {
            path: "/vault".into(),
            mode: 0o644,
        }
        .to_string();
        assert!(file.contains("0644"));
        assert!(file.contains("chmod 600"));

        let parent = SecretStoreError::InsecureParentDir {
            path: "/parent".into(),
            mode: 0o777,
        }
        .to_string();
        assert!(parent.contains("0777"));
        assert!(parent.contains("chmod"));

        assert!(SecretStoreError::InvalidLabel
            .to_string()
            .contains("A-Za-z0-9._-"));
        assert_eq!(
            SecretStoreError::NoEntry.to_string(),
            "secret was not found"
        );
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

    /// The five new variants exist, are constructable, render
    /// distinct non-empty messages, and the Tier-2 `WrongPassword` is NOT
    /// the Tier-1 `WrongPassphrase` (nor is the unseal error `Corruption`).
    #[test]
    fn new_variants_exist_and_are_distinct() {
        use SecretStoreError as E;
        assert_ne!(E::WrongPassword.to_string(), E::WrongPassphrase.to_string());
        assert_ne!(
            E::ExpectedProtectedButUnsealed.to_string(),
            E::Corruption.to_string()
        );
        let msgs: std::collections::HashSet<String> = [
            E::NeedsPassword.to_string(),
            E::WrongPassword.to_string(),
            E::BlankPassphrase.to_string(),
            E::ExpectedProtectedButUnsealed.to_string(),
            E::UnsupportedEnvelopeVersion { found: 2 }.to_string(),
        ]
        .into_iter()
        .collect();
        assert_eq!(msgs.len(), 5, "all five messages must be distinct");
    }

    /// Display + Debug render static, secret-free text. The
    /// version variant surfaces the (non-secret) version byte and nothing
    /// more.
    #[test]
    fn new_variants_carry_no_secret_in_display() {
        use SecretStoreError as E;
        assert_eq!(
            E::NeedsPassword.to_string(),
            "secret is password-protected; a password is required"
        );
        assert_eq!(E::WrongPassword.to_string(), "wrong object password");
        assert_eq!(
            E::BlankPassphrase.to_string(),
            "passphrase or password is blank or too short"
        );
        assert_eq!(
            E::ExpectedProtectedButUnsealed.to_string(),
            "expected a password-protected secret but the stored value is unprotected"
        );
        assert_eq!(
            E::UnsupportedEnvelopeVersion { found: 7 }.to_string(),
            "unsupported secret envelope version 7"
        );
        // Debug is non-empty and free of plaintext-ish tokens for all.
        for e in [
            E::NeedsPassword,
            E::WrongPassword,
            E::BlankPassphrase,
            E::ExpectedProtectedButUnsealed,
            E::UnsupportedEnvelopeVersion { found: 7 },
        ] {
            let rendered = format!("{e} {e:?}");
            assert!(!rendered.contains("plaintext"));
        }
    }

    /// The four Tier-2 credential /
    /// protection states project to a recoverable `NoStorageAccess` with
    /// the typed error losslessly downcast-able, leaking no secret.
    #[test]
    fn tier2_state_errors_project_to_recoverable_no_storage_access() {
        for original in [
            SecretStoreError::NeedsPassword,
            SecretStoreError::WrongPassword,
            SecretStoreError::ExpectedProtectedButUnsealed,
            SecretStoreError::BlankPassphrase,
        ] {
            let want = original.to_string();
            let k: KeyringError = original.into();
            assert!(!format!("{k}").contains("plaintext"));
            match &k {
                KeyringError::NoStorageAccess(src) => {
                    let recovered = src.downcast_ref::<SecretStoreError>();
                    assert!(
                        matches!(recovered, Some(e) if e.to_string() == want),
                        "expected recoverable {want}, got {recovered:?}"
                    );
                }
                other => panic!("expected NoStorageAccess for {want}, got {other:?}"),
            }
        }
    }

    /// `UnsupportedEnvelopeVersion` projects to the
    /// secret-free `BadStoreFormat` group (forward-format incompat,
    /// mirroring `VersionUnsupported`).
    #[test]
    fn unsupported_envelope_version_projects_to_bad_store_format() {
        let k: KeyringError = SecretStoreError::UnsupportedEnvelopeVersion { found: 9 }.into();
        assert!(matches!(k, KeyringError::BadStoreFormat(_)));
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

    /// `EntropyUnavailable` is a distinct, secret-free CSPRNG-failure
    /// variant — NOT aliased to `KdfFailure` — and projects to the
    /// secret-free `BadStoreFormat` group like the rest of the crypto family.
    #[test]
    fn entropy_unavailable_is_distinct_and_secret_free() {
        use SecretStoreError as E;
        assert_ne!(
            E::EntropyUnavailable.to_string(),
            E::KdfFailure.to_string(),
            "entropy failure must not read as a key-derivation failure"
        );
        assert_eq!(
            E::EntropyUnavailable.to_string(),
            "system entropy source unavailable"
        );
        let k: KeyringError = E::EntropyUnavailable.into();
        assert!(matches!(k, KeyringError::BadStoreFormat(_)));
        assert!(!format!("{k}").contains("plaintext"));
    }

    /// `AlreadyLocked` is the only recoverable-by-retry variant (drop the
    /// other handle and re-`open`); a representative spread of the rest is
    /// non-recoverable.
    #[test]
    fn only_already_locked_is_recoverable() {
        use SecretStoreError as E;
        assert!(E::AlreadyLocked.is_recoverable());
        for e in [
            E::WrongPassphrase,
            E::Corruption,
            E::KdfFailure,
            E::EntropyUnavailable,
            E::MalformedVault,
            E::InvalidLabel,
            E::NoEntry,
            E::Decrypt,
            E::Encrypt,
            E::from(std::io::Error::other("boom")),
            E::OsKeyring {
                kind: OsKeyringErrorKind::Backend,
            },
        ] {
            assert!(
                !e.is_recoverable(),
                "{e} must not be classified recoverable"
            );
        }
    }

    /// `error_kind_str` returns a stable snake_case tag; the sampled tags
    /// are pinned and the full variant set produces no duplicate tag.
    #[test]
    fn error_kind_str_tags_are_stable_and_unique() {
        use SecretStoreError as E;
        assert_eq!(E::AlreadyLocked.error_kind_str(), "already_locked");
        assert_eq!(E::WrongPassphrase.error_kind_str(), "wrong_passphrase");
        assert_eq!(
            E::EntropyUnavailable.error_kind_str(),
            "entropy_unavailable"
        );
        assert_eq!(
            E::Io(std::io::Error::other("x").into()).error_kind_str(),
            "io"
        );

        let tags: Vec<&str> = [
            E::WrongPassphrase,
            E::ExpectedProtectedButUnsealed,
            E::NeedsPassword,
            E::WrongPassword,
            E::BlankPassphrase,
            E::Corruption,
            E::KdfFailure,
            E::EntropyUnavailable,
            E::VersionUnsupported { found: 1 },
            E::UnsupportedEnvelopeVersion { found: 1 },
            E::MalformedVault,
            E::InvalidLabel,
            E::NoEntry,
            E::InsecurePermissions {
                path: "/vault".into(),
                mode: 0,
            },
            E::InsecureOwnership {
                path: "/vault".into(),
                found: 1,
                expected: 2,
            },
            E::InsecureParentDir {
                path: "/parent".into(),
                mode: 0,
            },
            E::SecretTooLarge { found: 1, max: 0 },
            E::AlreadyLocked,
            E::VaultTooLarge { found: 1, max: 0 },
            E::Decrypt,
            E::Encrypt,
            E::from(std::io::Error::other("x")),
            E::OsKeyring {
                kind: OsKeyringErrorKind::Backend,
            },
        ]
        .iter()
        .map(SecretStoreError::error_kind_str)
        .collect();
        let unique: std::collections::HashSet<&str> = tags.iter().copied().collect();
        assert_eq!(unique.len(), tags.len(), "every variant needs a unique tag");
    }
}
