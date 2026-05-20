//! Runtime seed/mnemonic supply for wallet rehydration.
//!
//! The SQLite persister never receives or persists seed material
//! (SECRETS.md). At load time the manager obtains the seed/mnemonic for
//! each persisted wallet from a [`SeedProvider`] — a storage-agnostic
//! port. The concrete `SecretStore`-backed adapter lives downstream in
//! `platform-wallet-storage` behind its `secrets` feature; this crate
//! only defines the port and storage-free payload newtypes
//! (M-DONT-LEAK-TYPES).
//!
//! # Memory hygiene
//!
//! [`SecretSeed`] / [`SecretPhrase`] zeroize on drop and redact their
//! `Debug`. The transient secret is borrowed only across the
//! `Wallet::from_*` call and dropped at the end of that wallet's load
//! iteration. No secret byte is cloned into a long-lived buffer,
//! logged, or placed in an error payload.

use std::fmt;

use zeroize::Zeroize;

/// Zeroize-on-drop wrapper for a BIP-39 mnemonic phrase (UTF-8).
///
/// `Display`/`Deref`/`Serialize` are intentionally absent; `Debug` is
/// redacted; the buffer is wiped on drop. This is the trait crate's
/// storage-free analogue of `platform-wallet-storage`'s `SecretString`
/// — the adapter copies across the boundary explicitly.
pub struct SecretPhrase(String);

impl SecretPhrase {
    /// Wrap a phrase.
    pub fn new(phrase: impl Into<String>) -> Self {
        Self(phrase.into())
    }

    /// Borrow the plaintext. The only read path.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl Drop for SecretPhrase {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl fmt::Debug for SecretPhrase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretPhrase(***)")
    }
}

/// Zeroize-on-drop wrapper for a 64-byte BIP-32 seed.
///
/// Same hygiene contract as [`SecretPhrase`]; `Debug` is redacted.
pub struct SecretSeed(Vec<u8>);

impl SecretSeed {
    /// Wrap raw seed bytes.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// Borrow the plaintext bytes. The only read path.
    pub fn expose(&self) -> &[u8] {
        &self.0
    }
}

impl Drop for SecretSeed {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl fmt::Debug for SecretSeed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecretSeed([REDACTED; {}])", self.0.len())
    }
}

/// The secret material a [`SeedProvider`] yields for one wallet.
///
/// Carries either a BIP-39 mnemonic (preferred) or raw 64-byte seed.
/// Never logged, serialized, or placed in an error.
#[derive(Debug)]
pub enum WalletSecret {
    /// A BIP-39 mnemonic phrase → `Wallet::from_mnemonic`.
    Mnemonic(SecretPhrase),
    /// Raw 64-byte BIP-32 seed → `Wallet::from_seed_bytes`.
    Seed(SecretSeed),
}

/// Why a [`SeedProvider`] could not yield secret material for a wallet.
///
/// Carries **no** secret byte, label value, or stringified backend
/// source — variants are structural only (SECRETS.md SEC-REQ-2.0.1).
/// Mirrors the non-secret kind surface of the underlying secret store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SeedUnavailable {
    /// No seed/mnemonic is stored for this wallet id (the store
    /// returned a clean "absent"). Recoverable — the wallet is skipped
    /// and can be loaded later once material is provided.
    #[error("no seed material stored for wallet")]
    Absent,
    /// The secret backend is locked or unreachable (keyring locked,
    /// backend unavailable, wrong passphrase). Recoverable — retry
    /// after unlock.
    #[error("secret store locked or unavailable: {0}")]
    StoreUnavailable(SecretStoreErrorKind),
    /// Any other typed secret-store error (structural kind only).
    #[error("secret store error: {0}")]
    StoreError(SecretStoreErrorKind),
}

/// Non-secret, `Copy` projection of the underlying secret store's error
/// taxonomy. Lets a caller distinguish "ask the operator to unlock"
/// from other failure modes without ever inspecting a secret. Every
/// variant maps to a static, structural string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretStoreErrorKind {
    /// No secure backend reachable (headless / no keyring service).
    BackendUnavailable,
    /// The OS keyring exists but its collection is locked.
    KeyringLocked,
    /// The supplied passphrase did not unlock the vault.
    WrongPassphrase,
    /// Decryption / integrity check failed.
    IntegrityCheckFailed,
    /// `label` failed the store's allowlist.
    InvalidLabel,
    /// Filesystem error (open / write / rename / fsync).
    Io,
    /// Key derivation (Argon2) failed.
    KeyDerivation,
    /// Vault format version unsupported, or vault malformed.
    MalformedVault,
    /// Stored vault file had insecure permissions.
    InsecurePermissions,
    /// Any other structural store error.
    Other,
}

impl fmt::Display for SecretStoreErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::BackendUnavailable => "backend unavailable",
            Self::KeyringLocked => "keyring locked",
            Self::WrongPassphrase => "wrong passphrase",
            Self::IntegrityCheckFailed => "integrity check failed",
            Self::InvalidLabel => "invalid label",
            Self::Io => "io error",
            Self::KeyDerivation => "key derivation failed",
            Self::MalformedVault => "malformed vault",
            Self::InsecurePermissions => "insecure permissions",
            Self::Other => "store error",
        };
        f.write_str(s)
    }
}

/// Supplies the runtime secret material a wallet needs to rehydrate.
///
/// Implementations MUST NOT log, clone into a long-lived buffer, or
/// place any secret byte in an error. `Ok` carries the material; `Err`
/// carries only the structural [`SeedUnavailable`] reason.
pub trait SeedProvider: Send + Sync {
    /// Yield the seed/mnemonic for `wallet_id`, or the typed reason
    /// none is available.
    fn seed_for(&self, wallet_id: [u8; 32]) -> Result<WalletSecret, SeedUnavailable>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_phrase_debug_redacted() {
        let p = SecretPhrase::new("correct horse battery staple");
        assert_eq!(format!("{p:?}"), "SecretPhrase(***)");
        assert!(!format!("{p:?}").contains("horse"));
        assert_eq!(p.expose(), "correct horse battery staple");
    }

    #[test]
    fn secret_seed_debug_redacted() {
        let s = SecretSeed::new(vec![1, 2, 3, 4, 5]);
        assert_eq!(format!("{s:?}"), "SecretSeed([REDACTED; 5])");
        assert!(!format!("{s:?}").contains('1'));
        assert_eq!(s.expose(), &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn wallet_secret_debug_does_not_leak() {
        let dbg = format!("{:?}", WalletSecret::Mnemonic(SecretPhrase::new("abandon")));
        assert!(!dbg.contains("abandon"), "Debug leaked phrase: {dbg}");
        let dbg = format!("{:?}", WalletSecret::Seed(SecretSeed::new(vec![9u8; 64])));
        assert!(dbg.contains("REDACTED"));
    }

    #[test]
    fn seed_unavailable_display_is_structural() {
        assert_eq!(
            SeedUnavailable::Absent.to_string(),
            "no seed material stored for wallet"
        );
        assert_eq!(
            SeedUnavailable::StoreUnavailable(SecretStoreErrorKind::KeyringLocked).to_string(),
            "secret store locked or unavailable: keyring locked"
        );
    }

    // Newtypes must run Drop (zeroize).
    const _: () = {
        assert!(std::mem::needs_drop::<SecretPhrase>());
        assert!(std::mem::needs_drop::<SecretSeed>());
    };
}
