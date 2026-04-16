//! Bridge from [`key_wallet_manager::WalletPersistence`] to [`PlatformWalletPersistence`].
//!
//! During SPV block processing, `WalletManager::process_block` accumulates
//! core-wallet changesets (UTXOs, transactions, synced height) and calls
//! `WalletPersistence::store` for each wallet. `CorePersistenceBridge`
//! wraps those calls by embedding the `WalletChangeSet` inside a
//! `PlatformWalletChangeSet { core: Some(cs), .. }` and forwarding to the
//! platform persister, so core-wallet state is persisted through the same
//! pipeline as all other platform wallet state.

use std::sync::Arc;

use key_wallet::changeset::WalletChangeSet;
use key_wallet_manager::{WalletId, WalletPersistence};

use crate::changeset::changeset::PlatformWalletChangeSet;
use crate::changeset::traits::PlatformWalletPersistence;

/// Bridges [`WalletPersistence`] (dashcore) to [`PlatformWalletPersistence`] (platform-wallet).
///
/// Wrap a `PlatformWalletPersistence` implementor with this type and pass it to
/// [`key_wallet_manager::WalletManager::new_with_persister`] so that core-wallet
/// changesets produced by SPV block processing are routed through the same
/// persistence pipeline as platform-wallet state.
pub struct CorePersistenceBridge {
    inner: Arc<dyn PlatformWalletPersistence>,
}

impl CorePersistenceBridge {
    pub fn new(inner: Arc<dyn PlatformWalletPersistence>) -> Self {
        Self { inner }
    }
}

impl WalletPersistence for CorePersistenceBridge {
    fn store(
        &self,
        wallet_id: WalletId,
        cs: WalletChangeSet,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let platform_cs = PlatformWalletChangeSet {
            core: Some(cs),
            ..PlatformWalletChangeSet::default()
        };
        self.inner.store(wallet_id, platform_cs)
    }
}
