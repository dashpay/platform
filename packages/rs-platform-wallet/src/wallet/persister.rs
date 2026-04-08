//! Per-wallet persistence handles.
//!
//! Contains:
//! - [`WalletPersister`] — wraps the shared [`PlatformWalletPersistence`] with
//!   a fixed `wallet_id` so callers don't need to pass the ID on every call.
//! - [`PlatformWalletPersisterBridge`] — implements dashcore's
//!   [`WalletPersistence`](key_wallet_manager::WalletPersistence) trait by
//!   wrapping each `WalletChangeSet` into a `PlatformWalletChangeSet` and
//!   delegating to `PlatformWalletPersistence`.

use std::sync::Arc;

use key_wallet::changeset::WalletChangeSet;
use key_wallet_manager::persistence::WalletPersistence;

use crate::changeset::{PlatformWalletChangeSet, PlatformWalletPersistence};
use crate::wallet::platform_wallet::WalletId;

/// Per-wallet persistence handle.
///
/// Thin wrapper around the shared [`PlatformWalletPersistence`] that binds
/// a specific wallet's ID. Created by [`PlatformWallet::new`] and used
/// internally for `queue_persist` / `flush_persist`.
#[derive(Clone)]
pub(crate) struct WalletPersister {
    wallet_id: WalletId,
    inner: Arc<dyn PlatformWalletPersistence>,
}

impl WalletPersister {
    pub(crate) fn new(wallet_id: WalletId, inner: Arc<dyn PlatformWalletPersistence>) -> Self {
        Self { wallet_id, inner }
    }

    pub(crate) fn store(&self, changeset: PlatformWalletChangeSet) {
        self.inner.store(self.wallet_id, changeset);
    }

    pub(crate) fn flush(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.inner.flush(self.wallet_id)
    }

    pub(crate) fn load(
        &self,
    ) -> Result<PlatformWalletChangeSet, Box<dyn std::error::Error + Send + Sync>> {
        self.inner.load(self.wallet_id)
    }
}

/// Bridge from dashcore's [`WalletPersistence`] to platform-wallet's
/// [`PlatformWalletPersistence`].
///
/// When `ManagedWalletState<PlatformWalletPersisterBridge>` processes a
/// transaction and produces a `WalletChangeSet`, the bridge wraps it into a
/// `PlatformWalletChangeSet { wallet: Some(changeset), ..Default::default() }`
/// and delegates to the shared `PlatformWalletPersistence`. This enables
/// automatic changeset persistence during `check_core_transaction`.
#[derive(Clone)]
pub struct PlatformWalletPersisterBridge {
    wallet_id: WalletId,
    inner: Arc<dyn PlatformWalletPersistence>,
}

impl std::fmt::Debug for PlatformWalletPersisterBridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlatformWalletPersisterBridge")
            .field("wallet_id", &hex::encode(self.wallet_id))
            .finish()
    }
}

impl PlatformWalletPersisterBridge {
    /// Create a new bridge for a specific wallet.
    pub fn new(wallet_id: WalletId, inner: Arc<dyn PlatformWalletPersistence>) -> Self {
        Self { wallet_id, inner }
    }
}

impl Default for PlatformWalletPersisterBridge {
    fn default() -> Self {
        // Default is required by ManagedWalletState's trait bounds
        // (WalletInfoInterface::from_wallet needs P: Default).
        // This should never be used in practice — we always construct
        // with an explicit persister.
        Self {
            wallet_id: [0u8; 32],
            inner: Arc::new(NoPlatformPersistence),
        }
    }
}

impl WalletPersistence for PlatformWalletPersisterBridge {
    fn store(
        &self,
        changeset: WalletChangeSet,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let platform_changeset = PlatformWalletChangeSet {
            wallet: Some(changeset),
            ..Default::default()
        };
        self.inner.store(self.wallet_id, platform_changeset);
        Ok(())
    }

    fn flush(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.inner.flush(self.wallet_id)
    }
}

/// No-op platform persistence used as the default for
/// `PlatformWalletPersisterBridge::default()`.
struct NoPlatformPersistence;

impl PlatformWalletPersistence for NoPlatformPersistence {
    fn store(&self, _wallet_id: WalletId, _changeset: PlatformWalletChangeSet) {}

    fn flush(
        &self,
        _wallet_id: WalletId,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    fn load(
        &self,
        _wallet_id: WalletId,
    ) -> Result<PlatformWalletChangeSet, Box<dyn std::error::Error + Send + Sync>> {
        Ok(PlatformWalletChangeSet::default())
    }
}
