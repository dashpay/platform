//! Per-wallet persistence handles.
//!
//! Contains:
//! - [`WalletPersister`] — wraps the shared [`PlatformWalletPersistence`] with
//!   a fixed `wallet_id` so callers don't need to pass the ID on every call.

use std::sync::Arc;

use crate::changeset::{PlatformWalletChangeSet, PlatformWalletPersistence};
use crate::wallet::platform_wallet::WalletId;

/// Per-wallet persistence handle.
///
/// Thin wrapper around the shared [`PlatformWalletPersistence`] that binds
/// a specific wallet's ID. Created by [`PlatformWallet::new`] and used
/// internally for `queue_persist` / `flush_persist`.
#[derive(Clone)]
pub struct WalletPersister {
    wallet_id: WalletId,
    inner: Arc<dyn PlatformWalletPersistence>,
}

impl WalletPersister {
    pub fn new(wallet_id: WalletId, inner: Arc<dyn PlatformWalletPersistence>) -> Self {
        Self { wallet_id, inner }
    }

    pub(crate) fn store(
        &self,
        changeset: PlatformWalletChangeSet,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.inner.store(self.wallet_id, changeset)
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

/// No-op platform persistence for standalone wallets.
pub struct NoPlatformPersistence;

impl PlatformWalletPersistence for NoPlatformPersistence {
    fn store(
        &self,
        _wallet_id: WalletId,
        _changeset: PlatformWalletChangeSet,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    fn flush(&self, _wallet_id: WalletId) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    fn load(
        &self,
        _wallet_id: WalletId,
    ) -> Result<PlatformWalletChangeSet, Box<dyn std::error::Error + Send + Sync>> {
        Ok(PlatformWalletChangeSet::default())
    }
}
