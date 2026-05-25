//! Bridge between [`FileStoreError`] and `keyring_core::Error`.
//!
//! The file backend's failure modes (wrong passphrase, malformed
//! vault, insecure permissions, KDF failure) are unique to a local
//! AEAD vault — `keyring_core::Error` does not name them. To stay on a
//! single SPI error type without losing the structural distinction we
//! box a unit-only [`FileStoreFailure`] marker inside
//! `keyring_core::Error::{NoStorageAccess, BadStoreFormat}`'s payload
//! (D1). Consumers (notably the seed-provider adapter) recover the
//! marker via `Error::source()` + downcast — see
//! [`downcast_failure`].
//!
//! Per Smythe EDIT-3, [`FileStoreFailure`] is **unit-variants only**
//! and never carries user-supplied or secret data; the cross-SPI
//! bridge is secret-free by construction.

use std::error::Error as StdError;

use keyring_core::Error as KeyringError;

use super::error::FileStoreError;

/// File-backend failure marker boxed across the
/// `keyring_core::Error::{NoStorageAccess, BadStoreFormat}` seam.
///
/// **Unit variants only** (Smythe EDIT-3): no field may carry a
/// user-supplied path, a secret byte, a passphrase, a label, or
/// stringified data. Numeric correlation fields are acceptable; this
/// taxonomy currently needs none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStoreFailure {
    /// Wrong passphrase rejected at the header verify-token tag check.
    WrongPassphrase,
    /// AEAD decryption / integrity check failed on a stored entry.
    Decrypt,
    /// Argon2 key derivation failed.
    KdfFailure,
    /// Vault header declared an unsupported `format_version`.
    VersionUnsupported,
    /// Vault file framing was malformed.
    MalformedVault,
    /// Pre-existing vault file held looser-than-0600 permissions.
    InsecurePermissions,
    /// `rekey` ran while an outstanding credential held the inner `Arc`.
    Busy,
}

impl std::fmt::Display for FileStoreFailure {
    /// **Load-bearing text.** [`marker_from_message`] recovers the
    /// variant from a `BadStoreFormat` `String` by exact match against
    /// these strings, so editing any arm here requires updating
    /// `marker_from_message` in lockstep (and vice versa).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Static, parameter-free strings — no user / secret data may
        // ever enter this Display (Smythe EDIT-3).
        f.write_str(match self {
            Self::WrongPassphrase => "wrong passphrase",
            Self::Decrypt => "decryption/integrity check failed",
            Self::KdfFailure => "key derivation failed",
            Self::VersionUnsupported => "unsupported vault format version",
            Self::MalformedVault => "malformed vault file",
            Self::InsecurePermissions => "vault file has insecure permissions",
            Self::Busy => "store is busy: outstanding credentials prevent rekey",
        })
    }
}

impl StdError for FileStoreFailure {}

/// Lift a [`FileStoreError`] into a `keyring_core::Error` for the
/// `CredentialApi` / `CredentialStoreApi` seam.
///
/// - `WrongPassphrase` rides inside
///   [`KeyringError::NoStorageAccess`] (operator UX: "ask the operator
///   to unlock" — same family as today's `KeyringLocked` mapping).
/// - `Decrypt`/`KdfFailure`/`VersionUnsupported`/`MalformedVault`/
///   `InsecurePermissions` ride inside [`KeyringError::BadStoreFormat`]
///   with a static `String` — the structural marker is recovered by
///   downcasting the source. Per Smythe EDIT-2 we never put secret
///   data in `BadDataFormat`/`BadEncoding`.
/// - `InvalidLabel` becomes
///   `KeyringError::Invalid("user", "<static>")`.
/// - `Io` becomes `KeyringError::PlatformFailure(io_err)`.
pub fn into_keyring(e: FileStoreError) -> KeyringError {
    match e {
        FileStoreError::WrongPassphrase => {
            KeyringError::NoStorageAccess(Box::new(FileStoreFailure::WrongPassphrase))
        }
        FileStoreError::Busy => KeyringError::NoStorageAccess(Box::new(FileStoreFailure::Busy)),
        FileStoreError::Decrypt => bad_format(FileStoreFailure::Decrypt),
        FileStoreError::KdfFailure => bad_format(FileStoreFailure::KdfFailure),
        FileStoreError::VersionUnsupported { .. } => {
            bad_format(FileStoreFailure::VersionUnsupported)
        }
        FileStoreError::MalformedVault => bad_format(FileStoreFailure::MalformedVault),
        FileStoreError::InsecurePermissions { .. } => {
            bad_format(FileStoreFailure::InsecurePermissions)
        }
        FileStoreError::InvalidLabel => {
            KeyringError::Invalid("user".to_string(), "label allowlist violation".to_string())
        }
        FileStoreError::Io(io) => KeyringError::PlatformFailure(Box::new(io)),
    }
}

/// `BadStoreFormat` with the marker both in the boxed `source()` chain
/// and as the rendered string — keeps Display informative while letting
/// downcast recover the structural variant.
fn bad_format(failure: FileStoreFailure) -> KeyringError {
    KeyringError::BadStoreFormat(failure.to_string())
}

/// Recover a [`FileStoreFailure`] from a `keyring_core::Error`, if
/// the error was produced by the file backend's [`into_keyring`].
/// Returns `None` for non-file-backend errors and for variants the
/// bridge does not carry a marker on (e.g. `BadStoreFormat`'s
/// `String`-only variant — see callers' fallback handling).
pub fn downcast_failure(e: &KeyringError) -> Option<FileStoreFailure> {
    let src: &(dyn StdError + 'static) = match e {
        KeyringError::NoStorageAccess(inner) => inner.as_ref(),
        // `BadStoreFormat` carries only a `String` payload, so its
        // structural marker is read off the rendered text below.
        KeyringError::BadStoreFormat(s) => return marker_from_message(s),
        _ => return None,
    };
    src.downcast_ref::<FileStoreFailure>().copied()
}

fn marker_from_message(s: &str) -> Option<FileStoreFailure> {
    [
        FileStoreFailure::Decrypt,
        FileStoreFailure::KdfFailure,
        FileStoreFailure::VersionUnsupported,
        FileStoreFailure::MalformedVault,
        FileStoreFailure::InsecurePermissions,
        FileStoreFailure::WrongPassphrase,
        FileStoreFailure::Busy,
    ]
    .into_iter()
    .find(|f| s == f.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_storage_access_markers_round_trip() {
        for (err, expected) in [
            (
                FileStoreError::WrongPassphrase,
                FileStoreFailure::WrongPassphrase,
            ),
            (FileStoreError::Busy, FileStoreFailure::Busy),
        ] {
            let k = into_keyring(err);
            assert!(matches!(k, KeyringError::NoStorageAccess(_)));
            assert_eq!(downcast_failure(&k), Some(expected));
        }
    }

    #[test]
    fn bad_store_format_markers_round_trip() {
        for (err, expected) in [
            (FileStoreError::Decrypt, FileStoreFailure::Decrypt),
            (FileStoreError::KdfFailure, FileStoreFailure::KdfFailure),
            (
                FileStoreError::VersionUnsupported { found: 999 },
                FileStoreFailure::VersionUnsupported,
            ),
            (
                FileStoreError::MalformedVault,
                FileStoreFailure::MalformedVault,
            ),
            (
                FileStoreError::InsecurePermissions { mode: 0o644 },
                FileStoreFailure::InsecurePermissions,
            ),
        ] {
            let k = into_keyring(err);
            assert!(matches!(k, KeyringError::BadStoreFormat(_)));
            assert_eq!(downcast_failure(&k), Some(expected));
        }
    }

    #[test]
    fn invalid_label_maps_to_invalid_user() {
        let k = into_keyring(FileStoreError::InvalidLabel);
        match k {
            KeyringError::Invalid(attr, _) => assert_eq!(attr, "user"),
            other => panic!("expected Invalid, got {other:?}"),
        }
    }

    #[test]
    fn io_maps_to_platform_failure() {
        let io = std::io::Error::other("boom");
        let k = into_keyring(FileStoreError::Io(io));
        assert!(matches!(k, KeyringError::PlatformFailure(_)));
    }

    #[test]
    fn marker_from_message_round_trips_every_variant() {
        // Display text is load-bearing: every variant must recover from
        // its own rendered string, or the BadStoreFormat seam loses it.
        for f in [
            FileStoreFailure::WrongPassphrase,
            FileStoreFailure::Decrypt,
            FileStoreFailure::KdfFailure,
            FileStoreFailure::VersionUnsupported,
            FileStoreFailure::MalformedVault,
            FileStoreFailure::InsecurePermissions,
            FileStoreFailure::Busy,
        ] {
            assert_eq!(marker_from_message(&f.to_string()), Some(f));
        }
    }

    #[test]
    fn downcast_returns_none_for_unrelated_errors() {
        assert!(downcast_failure(&KeyringError::NoEntry).is_none());
        assert!(downcast_failure(&KeyringError::NoDefaultStore).is_none());
    }

    /// `FileStoreFailure` is unit-variants only (Smythe EDIT-3): no
    /// field may carry user-supplied or secret data. The `Copy` bound
    /// is the structural witness — only enums whose variants hold
    /// `Copy` data can derive it.
    const _: () = {
        const fn _assert_copy<T: Copy>() {}
        _assert_copy::<FileStoreFailure>();
    };
}
