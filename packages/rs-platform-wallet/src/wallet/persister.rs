//! Per-wallet persistence handle.
//!
//! Wraps the shared [`PlatformWalletPersistence`] with a fixed `wallet_id`
//! so callers don't need to pass the ID on every call.

use std::sync::Arc;

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
