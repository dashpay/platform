//! Per-wallet in-memory buffer.
//!
//! `store` merges the incoming changeset into a per-wallet accumulator
//! using each sub-changeset's `Merge` impl. `flush` drains one wallet's
//! accumulator and returns the owned changeset for the schema dispatcher
//! to write under one SQLite transaction. The buffer never owns the
//! database connection: a caller that must validate an incoming
//! changeset against disk hands the probe to `store_checked` as a
//! closure, so the probe and the merge share one critical section.

use std::collections::HashMap;
use std::sync::Mutex;

use platform_wallet::changeset::{IdentityChangeSet, Merge, PlatformWalletChangeSet};
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::error::WalletStorageError;

#[derive(Default)]
pub struct Buffer {
    inner: Mutex<HashMap<WalletId, PendingWrites>>,
}

/// Merged segments separated by identity removal/re-addition boundaries.
/// All segments commit in one transaction, preserving incarnation cleanup.
#[derive(Default)]
pub struct PendingWrites {
    pub segments: Vec<PlatformWalletChangeSet>,
}

impl PendingWrites {
    fn push(&mut self, mut incoming: PlatformWalletChangeSet) {
        if incoming.is_empty() {
            return;
        }
        incoming.identities = incoming
            .identities
            .as_ref()
            .map(|ids| self.normalize_identities(ids));
        if let Some(last) = self.segments.last_mut() {
            let readds_removed =
                readds_removed(last.identities.as_ref(), incoming.identities.as_ref());
            if !readds_removed {
                last.merge(incoming);
                return;
            }
        }
        self.segments.push(incoming);
    }

    /// Project the candidate batch's final identity slots for admission.
    pub fn identities_with(&self, incoming: &PlatformWalletChangeSet) -> IdentityChangeSet {
        let mut incoming = incoming
            .identities
            .as_ref()
            .map(|ids| self.normalize_identities(ids));
        let mut effective = IdentityChangeSet::default();
        for (index, segment) in self.segments.iter().enumerate() {
            let mut ids = segment.identities.clone().unwrap_or_default();
            if index + 1 == self.segments.len() && !readds_removed(Some(&ids), incoming.as_ref()) {
                if let Some(new) = incoming.take() {
                    ids.merge(new);
                }
            }
            project_identities(&mut effective, ids);
        }
        if let Some(incoming) = incoming {
            project_identities(&mut effective, incoming);
        }
        effective
    }

    // A boundary for one identity must not reset another's revision gate or
    // snapshot collections. Removal ends the lookup before any older upsert.
    fn normalize_identities(&self, incoming: &IdentityChangeSet) -> IdentityChangeSet {
        let mut normalized = IdentityChangeSet::default();
        for id in incoming.identities.keys() {
            for segment in self.segments.iter().rev() {
                let Some(ids) = &segment.identities else {
                    continue;
                };
                if ids.removed.contains(id) {
                    break;
                }
                if let Some(entry) = ids.identities.get(id) {
                    normalized.identities.insert(*id, entry.clone());
                    break;
                }
            }
        }
        normalized.merge(incoming.clone());
        normalized
    }
}

fn readds_removed(old: Option<&IdentityChangeSet>, new: Option<&IdentityChangeSet>) -> bool {
    old.is_some_and(|old| {
        new.is_some_and(|new| new.identities.keys().any(|id| old.removed.contains(id)))
    })
}

fn project_identities(effective: &mut IdentityChangeSet, next: IdentityChangeSet) {
    for (id, entry) in next.identities {
        effective.removed.remove(&id);
        effective.identities.insert(id, entry);
    }
    for id in next.removed {
        effective.identities.remove(&id);
        effective.removed.insert(id);
    }
}

impl Buffer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Merge a changeset into the buffer for `wallet_id`.
    ///
    /// Test-only convenience: every production write goes through
    /// [`store_checked`](Self::store_checked), which the persister hands
    /// the identity-slot probe.
    #[cfg(test)]
    pub fn store(
        &self,
        wallet_id: WalletId,
        cs: PlatformWalletChangeSet,
    ) -> Result<(), WalletStorageError> {
        self.store_checked(wallet_id, cs, |_, _| Ok(()))
    }

    /// Merge a changeset into the buffer for `wallet_id`, but only if
    /// `check` accepts it.
    ///
    /// `check` is handed the wallet's currently-buffered changeset (if
    /// any) and the incoming one, and runs UNDER the buffer lock — no
    /// other `store` for this wallet can slip between the check and the
    /// merge. On `Err` the buffered changeset is left exactly as it was
    /// and `cs` is dropped, so only the caller that made the offending
    /// write pays for it.
    pub fn store_checked<F>(
        &self,
        wallet_id: WalletId,
        cs: PlatformWalletChangeSet,
        check: F,
    ) -> Result<(), WalletStorageError>
    where
        F: FnOnce(
            Option<&PendingWrites>,
            &PlatformWalletChangeSet,
        ) -> Result<(), WalletStorageError>,
    {
        if cs.is_empty() {
            return Ok(());
        }
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| WalletStorageError::LockPoisoned)?;
        check(guard.get(&wallet_id), &cs)?;
        guard.entry(wallet_id).or_default().push(cs);
        Ok(())
    }

    /// Move the buffered changeset out for `wallet_id`. Returns
    /// `None` when nothing is staged. Callers MUST either commit it
    /// (success path) or hand it back via [`Self::restore`] on
    /// transient failure — dropping it on error == data loss.
    pub fn take_for_flush(
        &self,
        wallet_id: &WalletId,
    ) -> Result<Option<PendingWrites>, WalletStorageError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| WalletStorageError::LockPoisoned)?;
        Ok(guard.remove(wallet_id).filter(|cs| !cs.segments.is_empty()))
    }

    /// Re-merge a previously-taken changeset back into the buffer
    /// after a transient flush failure. Uses each sub-changeset's
    /// `Merge` impl so any `store(...)` that arrived between the
    /// `take_for_flush` and the failure wins on overlapping fields
    /// (LWW). No clone: the caller hands ownership back.
    pub fn restore(
        &self,
        wallet_id: WalletId,
        cs: PendingWrites,
    ) -> Result<(), WalletStorageError> {
        if cs.segments.is_empty() {
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
        for segment in newer.segments {
            entry.push(segment);
        }
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
        let core = merged
            .segments
            .into_iter()
            .next()
            .unwrap()
            .core
            .expect("core present");
        assert_eq!(core.synced_height, Some(20));
        assert_eq!(core.last_processed_height, Some(10));
    }

    #[test]
    fn store_checked_shows_the_check_what_is_already_buffered() {
        let buf = Buffer::new();
        let w = [0xCCu8; 32];
        buf.store(w, cs_height(10, 10)).unwrap();

        let seen = std::cell::Cell::new(None);
        buf.store_checked(w, cs_height(20, 20), |buffered, incoming| {
            seen.set(Some((
                buffered.and_then(|cs| cs.segments.last()?.core.as_ref()?.synced_height),
                incoming.core.as_ref().unwrap().synced_height,
            )));
            Ok(())
        })
        .unwrap();

        assert_eq!(seen.get(), Some((Some(10), Some(20))));
    }

    #[test]
    fn store_checked_rejection_leaves_the_buffered_value_untouched() {
        let buf = Buffer::new();
        let w = [0xDDu8; 32];
        buf.store(w, cs_height(10, 10)).unwrap();

        let err = buf
            .store_checked(w, cs_height(20, 20), |_, _| {
                Err(WalletStorageError::LockPoisoned)
            })
            .expect_err("the check refused the incoming changeset");

        assert!(matches!(err, WalletStorageError::LockPoisoned));
        let kept = buf.take_for_flush(&w).unwrap().expect("value still staged");
        assert_eq!(
            kept.segments[0]
                .core
                .as_ref()
                .expect("core present")
                .synced_height,
            Some(10)
        );
    }

    #[test]
    fn restore_into_empty_slot_inserts() {
        let buf = Buffer::new();
        let w = [0xBBu8; 32];
        // Buffer has nothing for `w`; restore must seed the slot.
        buf.restore(
            w,
            PendingWrites {
                segments: vec![cs_height(7, 7)],
            },
        )
        .unwrap();
        let got = buf
            .take_for_flush(&w)
            .unwrap()
            .expect("restored value present");
        let core = got
            .segments
            .into_iter()
            .next()
            .unwrap()
            .core
            .expect("core present");
        assert_eq!(core.synced_height, Some(7));
        assert_eq!(core.last_processed_height, Some(7));
    }
}
