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

use crate::sqlite::error::WalletStorageError;

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
    ) -> Result<(), WalletStorageError> {
        if cs.is_empty() {
            return Ok(());
        }
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| WalletStorageError::LockPoisoned)?;
        guard.entry(wallet_id).or_default().merge(cs);
        Ok(())
    }

    /// Move the buffered changeset out for `wallet_id`. Returns
    /// `None` when nothing is staged. Callers MUST either commit it
    /// (success path) or hand it back via [`Self::restore`] on
    /// transient failure — dropping it on error == data loss.
    pub fn take_for_flush(
        &self,
        wallet_id: &WalletId,
    ) -> Result<Option<PlatformWalletChangeSet>, WalletStorageError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| WalletStorageError::LockPoisoned)?;
        Ok(guard.remove(wallet_id).filter(|cs| !cs.is_empty()))
    }

    /// Re-merge a previously-taken changeset back into the buffer
    /// after a transient flush failure. Uses each sub-changeset's
    /// `Merge` impl so any `store(...)` that arrived between the
    /// `take_for_flush` and the failure wins on overlapping fields
    /// (LWW). No clone: the caller hands ownership back.
    pub fn restore(
        &self,
        wallet_id: WalletId,
        cs: PlatformWalletChangeSet,
    ) -> Result<(), WalletStorageError> {
        if cs.is_empty() {
            return Ok(());
        }
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| WalletStorageError::LockPoisoned)?;
        // Merge `cs` (older snapshot) FIRST, then re-apply anything
        // that arrived later — done by swapping current with `cs` and
        // merging the (originally newer) buffered value on top.
        let entry = guard.entry(wallet_id).or_default();
        let newer = std::mem::take(entry);
        *entry = cs;
        entry.merge(newer);
        Ok(())
    }

    /// Every wallet currently holding buffered data, sorted by id for
    /// deterministic flush ordering.
    pub fn dirty_wallets(&self) -> Result<Vec<WalletId>, WalletStorageError> {
        let guard = self
            .inner
            .lock()
            .map_err(|_| WalletStorageError::LockPoisoned)?;
        let mut ids: Vec<WalletId> = guard.keys().copied().collect();
        ids.sort();
        Ok(ids)
    }

    /// Discard every buffered changeset after the backing connection becomes
    /// permanently unusable.
    pub fn discard_all(&self) -> Result<(), WalletStorageError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| WalletStorageError::LockPoisoned)?;
        guard.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use platform_wallet::changeset::CoreChangeSet;

    fn cs_height(synced: u32, last_processed: u32) -> PlatformWalletChangeSet {
        PlatformWalletChangeSet {
            core: Some(CoreChangeSet {
                synced_height: Some(synced),
                last_processed_height: Some(last_processed),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    #[test]
    fn take_then_restore_with_intervening_store_merges_lww() {
        let buf = Buffer::new();
        let w = [0xAAu8; 32];
        // Stage A (older), take it out.
        buf.store(w, cs_height(10, 10)).unwrap();
        let taken = buf
            .take_for_flush(&w)
            .unwrap()
            .expect("staged value present");
        // B arrives during the imagined flush window.
        buf.store(w, cs_height(20, 5)).unwrap();
        // Restore the taken (older) snapshot — newer must win on the
        // monotonic-max merge of `synced_height` / `last_processed_height`.
        buf.restore(w, taken).unwrap();
        let merged = buf
            .take_for_flush(&w)
            .unwrap()
            .expect("merged value present");
        let core = merged.core.expect("core present");
        assert_eq!(core.synced_height, Some(20));
        assert_eq!(core.last_processed_height, Some(10));
    }

    #[test]
    fn restore_into_empty_slot_inserts() {
        let buf = Buffer::new();
        let w = [0xBBu8; 32];
        // Buffer has nothing for `w`; restore must seed the slot.
        buf.restore(w, cs_height(7, 7)).unwrap();
        let got = buf
            .take_for_flush(&w)
            .unwrap()
            .expect("restored value present");
        let core = got.core.expect("core present");
        assert_eq!(core.synced_height, Some(7));
        assert_eq!(core.last_processed_height, Some(7));
    }
}
