//! Aggregate result of [`load_from_persistor`].
//!
//! [`load_from_persistor`]: super::PlatformWalletManager::load_from_persistor

use crate::seed_provider::{SecretStoreErrorKind, SeedUnavailable};
use crate::wallet::platform_wallet::WalletId;

/// Why a wallet was skipped during a load pass.
///
/// A skip means the wallet's seed/mnemonic was **unavailable** — a
/// recoverable state (retry after the operator provides or unlocks the
/// material). It is distinct from a **wrong** seed, which is a
/// fail-closed `WrongSeedForDatabase` error and never appears here.
///
/// Carries no secret material — variants are structural, mirroring the
/// non-secret secret-store error surface (SECRETS.md SEC-REQ-2.0.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SkipReason {
    /// The store returned a clean "no entry" for this wallet id.
    #[error("no seed material stored for wallet")]
    SeedAbsent,
    /// The secret backend is locked / unavailable; retry after unlock.
    #[error("secret store locked or unavailable: {0}")]
    StoreUnavailable(SecretStoreErrorKind),
    /// Any other typed secret-store error (structural kind only).
    #[error("secret store error: {0}")]
    StoreError(SecretStoreErrorKind),
}

impl From<SeedUnavailable> for SkipReason {
    fn from(e: SeedUnavailable) -> Self {
        match e {
            SeedUnavailable::Absent => SkipReason::SeedAbsent,
            SeedUnavailable::StoreUnavailable(k) => SkipReason::StoreUnavailable(k),
            SeedUnavailable::StoreError(k) => SkipReason::StoreError(k),
        }
    }
}

/// Aggregate, synchronous view of one
/// [`load_from_persistor`](super::PlatformWalletManager::load_from_persistor)
/// pass.
///
/// `Ok(LoadOutcome)` with a non-empty `skipped` is **success** — a
/// skipped wallet is an expected, recoverable state. The `Err` arm is
/// reserved for genuine load failures (persister I/O, decode
/// corruption, the fail-closed `WrongSeedForDatabase` escalation).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoadOutcome {
    /// Wallets fully reconstructed and registered, in load order.
    pub loaded: Vec<WalletId>,
    /// Wallets skipped because their seed was unavailable, in load
    /// order. Never contains a wrong-seed wallet.
    pub skipped: Vec<(WalletId, SkipReason)>,
}
