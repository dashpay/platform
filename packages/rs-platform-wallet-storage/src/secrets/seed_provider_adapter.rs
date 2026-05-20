//! `keyring_core::CredentialStoreApi` → `platform_wallet::SeedProvider` adapter.
//!
//! Lives in this crate (downstream of `platform-wallet`) so the
//! dependency stays acyclic: `platform-wallet` defines the
//! storage-agnostic `SeedProvider` port; the concrete adapter over the
//! upstream [`CredentialStoreApi`] SPI is here, behind the `secrets`
//! feature (ports-and-adapters, M-DI-HIERARCHY).
//!
//! # Label convention (fixed)
//!
//! For a wallet id the adapter looks up, in order:
//! 1. `"mnemonic"` → a UTF-8 BIP-39 phrase → `Wallet::from_mnemonic`.
//! 2. else `"seed"` → 64 raw bytes → `Wallet::from_seed_bytes`.
//!
//! Mnemonic is preferred when both exist (matches the live creation
//! path's mnemonic-first ergonomics).
//!
//! # Memory hygiene
//!
//! The upstream SPI returns a bare `Vec<u8>` from `Entry::get_secret`.
//! The adapter wraps it via [`SecretBytes::new`] **immediately**, with
//! no named intermediate `Vec` binding, so the bare buffer's window is
//! zero statements (Smythe EDIT-1 / SEC-REQ-3.5). The wrapped bytes are
//! copied once into `platform-wallet`'s own zeroize-on-drop newtype and
//! the wrapper is dropped immediately, so no extra long-lived copy
//! exists. No secret byte, label value, or stringified backend source
//! ever reaches a log line or an error (SECRETS.md SEC-REQ-2.0.1).
//!
//! [`CredentialStoreApi`]: keyring_core::api::CredentialStoreApi

use std::sync::Arc;

use keyring_core::api::CredentialStoreApi;
use keyring_core::Error as KeyringError;
use platform_wallet::seed_provider::{
    SecretPhrase, SecretSeed, SecretStoreErrorKind, SeedProvider, SeedUnavailable, WalletSecret,
};

use super::{downcast_failure, FileStoreFailure, SecretBytes, WalletId, SERVICE_PREFIX};

/// Fixed labels (subset of SECRETS.md's reserved set).
const LABEL_MNEMONIC: &str = "mnemonic";
const LABEL_SEED: &str = "seed";

/// Adapts an [`Arc<dyn CredentialStoreApi>`](CredentialStoreApi) to
/// `platform_wallet`'s [`SeedProvider`].
pub struct CredentialStoreSeedProvider {
    store: Arc<dyn CredentialStoreApi + Send + Sync>,
}

impl CredentialStoreSeedProvider {
    /// Wrap a credential store.
    pub fn new(store: Arc<dyn CredentialStoreApi + Send + Sync>) -> Self {
        Self { store }
    }

    /// `SERVICE_PREFIX + hex(wallet_id)` — the per-wallet keyring
    /// service namespace used by `EncryptedFileStore` and accepted by
    /// any other backend.
    fn service_for(wid: &WalletId) -> String {
        format!("{SERVICE_PREFIX}{}", wid.to_hex())
    }
}

impl SeedProvider for CredentialStoreSeedProvider {
    fn seed_for(&self, wallet_id: [u8; 32]) -> Result<WalletSecret, SeedUnavailable> {
        let wid = WalletId::from(wallet_id);
        let service = Self::service_for(&wid);

        // Build the entry and fetch the bytes. Absence rides as
        // `KeyringError::NoEntry` from `get_secret` (build does not
        // probe presence). Wrap into `SecretBytes` immediately with no
        // named intermediate (EDIT-1).
        let try_label = |label: &str| -> Result<Option<SecretBytes>, KeyringError> {
            let entry = self.store.build(&service, label, None)?;
            match entry.get_secret() {
                Ok(bytes) => Ok(Some(SecretBytes::new(bytes))),
                Err(KeyringError::NoEntry) => Ok(None),
                Err(e) => Err(e),
            }
        };

        // 1. Mnemonic (preferred).
        match try_label(LABEL_MNEMONIC) {
            Ok(Some(sb)) => {
                let phrase = std::str::from_utf8(sb.expose_secret())
                    .map_err(|_| SeedUnavailable::StoreError(SecretStoreErrorKind::Other))?
                    .to_string();
                return Ok(WalletSecret::Mnemonic(SecretPhrase::new(phrase)));
            }
            Ok(None) => {}
            Err(e) => return Err(to_unavailable(&e)),
        }

        // 2. Raw 64-byte seed.
        match try_label(LABEL_SEED) {
            Ok(Some(sb)) => Ok(WalletSecret::Seed(SecretSeed::new(
                sb.expose_secret().to_vec(),
            ))),
            Ok(None) => Err(SeedUnavailable::Absent),
            Err(e) => Err(to_unavailable(&e)),
        }
    }
}

/// Project a [`KeyringError`] onto the port's structural
/// [`SeedUnavailable`] taxonomy.
///
/// File-backend errors carry a typed [`FileStoreFailure`] marker
/// recovered via [`downcast_failure`] so the operator-actionable
/// `WrongPassphrase` case is preserved across the SPI seam.
/// `NoEntry` is handled at the call site as a clean "absent" — if it
/// reaches here defensively it still maps to `Absent`.
fn to_unavailable(e: &KeyringError) -> SeedUnavailable {
    use KeyringError::*;
    match e {
        NoEntry => SeedUnavailable::Absent,
        NoDefaultStore => {
            SeedUnavailable::StoreUnavailable(SecretStoreErrorKind::BackendUnavailable)
        }
        NoStorageAccess(_) | BadStoreFormat(_) => {
            let kind = match downcast_failure(e) {
                Some(FileStoreFailure::WrongPassphrase) => SecretStoreErrorKind::WrongPassphrase,
                Some(FileStoreFailure::Decrypt) => SecretStoreErrorKind::IntegrityCheckFailed,
                Some(FileStoreFailure::KdfFailure) => SecretStoreErrorKind::KeyDerivation,
                Some(FileStoreFailure::VersionUnsupported | FileStoreFailure::MalformedVault) => {
                    SecretStoreErrorKind::MalformedVault
                }
                Some(FileStoreFailure::InsecurePermissions) => {
                    SecretStoreErrorKind::InsecurePermissions
                }
                None => match e {
                    NoStorageAccess(_) => SecretStoreErrorKind::KeyringLocked,
                    _ => SecretStoreErrorKind::MalformedVault,
                },
            };
            // `WrongPassphrase`, `KeyringLocked`, `BackendUnavailable`
            // are operator-actionable retry-after-unlock cases; the
            // rest are terminal store errors.
            match kind {
                SecretStoreErrorKind::WrongPassphrase
                | SecretStoreErrorKind::KeyringLocked
                | SecretStoreErrorKind::BackendUnavailable => {
                    SeedUnavailable::StoreUnavailable(kind)
                }
                _ => SeedUnavailable::StoreError(kind),
            }
        }
        Invalid(_, _) => SeedUnavailable::StoreError(SecretStoreErrorKind::InvalidLabel),
        PlatformFailure(_) => SeedUnavailable::StoreError(SecretStoreErrorKind::Io),
        _ => SeedUnavailable::StoreError(SecretStoreErrorKind::Other),
    }
}
