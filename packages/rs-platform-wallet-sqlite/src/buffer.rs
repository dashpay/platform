//! Per-wallet in-memory buffer.
//!
//! `store` merges the incoming changeset into a per-wallet accumulator
//! using each sub-changeset's `Merge` impl. `flush` drains one wallet's
//! accumulator and returns the owned changeset for the schema dispatcher
//! to write under one SQLite transaction. The buffer never owns the
//! database connection.

use std::collections::HashMap;
use std::sync::Mutex;

use platform_wallet::changeset::{Merge, PlatformWalletChangeSet};
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::error::SqlitePersisterError;

#[derive(Default)]
pub struct Buffer {
    inner: Mutex<HashMap<WalletId, PlatformWalletChangeSet>>,
}

impl Buffer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Merge a changeset into the buffer for `wallet_id`.
    pub fn store(
        &self,
        wallet_id: WalletId,
        cs: PlatformWalletChangeSet,
    ) -> Result<(), SqlitePersisterError> {
        if cs.is_empty() {
            return Ok(());
        }
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| SqlitePersisterError::LockPoisoned)?;
        guard.entry(wallet_id).or_default().merge(cs);
        Ok(())
    }

    /// Drain (return) the buffered changeset for `wallet_id`. Returns
    /// `None` if there is no pending data.
    pub fn drain(
        &self,
        wallet_id: &WalletId,
    ) -> Result<Option<PlatformWalletChangeSet>, SqlitePersisterError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| SqlitePersisterError::LockPoisoned)?;
        Ok(guard.remove(wallet_id).filter(|cs| !cs.is_empty()))
    }

    /// Every wallet currently holding buffered data, sorted by id for
    /// deterministic flush ordering.
    pub fn dirty_wallets(&self) -> Result<Vec<WalletId>, SqlitePersisterError> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| SqlitePersisterError::LockPoisoned)?;
        let mut ids: Vec<WalletId> = guard.keys().copied().collect();
        ids.sort();
        Ok(ids)
    }
}
