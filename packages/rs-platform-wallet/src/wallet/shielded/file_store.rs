//! File-backed `ShieldedStore` impl.
//!
//! The Orchard commitment tree is shared across every subwallet
//! that decrypts notes against the same network — the on-chain
//! commitment stream is identical for every consumer. This store
//! therefore persists the tree to a SQLite file (via
//! [`ClientPersistentCommitmentTree`]) and keeps per-subwallet
//! decrypted notes / nullifier bookkeeping in memory, scoped by
//! [`SubwalletId`]. Notes are rediscovered on cold start by
//! re-running [`ShieldedWallet::sync_notes`] against the cached
//! tree (or, when the host persister is wired up, restored from
//! SwiftData before sync runs).

use std::collections::BTreeMap;
use std::error::Error as StdError;
use std::fmt;
use std::path::Path;
use std::sync::Mutex;

use grovedb_commitment_tree::{ClientPersistentCommitmentTree, Position, Retention};

use super::store::{ShieldedNote, ShieldedStore, SubwalletId, SubwalletState};

/// Error type for [`FileBackedShieldedStore`].
#[derive(Debug)]
pub struct FileShieldedStoreError(pub String);

impl fmt::Display for FileShieldedStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl StdError for FileShieldedStoreError {}

/// File-backed shielded store: SQLite-persisted commitment tree
/// plus in-memory per-subwallet decrypted notes / nullifier
/// bookkeeping.
pub struct FileBackedShieldedStore {
    /// SQLite-backed commitment tree. Wrapped in a `Mutex` because
    /// the underlying SQLite store is not `Sync`; the
    /// [`ShieldedStore`] trait requires `Send + Sync`. Outer
    /// concurrency is still serialized through `ShieldedWallet`'s
    /// `RwLock<S>`; this inner mutex is just a `Sync`-restoring
    /// shim and is uncontended in practice.
    tree: Mutex<ClientPersistentCommitmentTree>,
    /// Per-subwallet notes + sync state, keyed by `(wallet_id,
    /// account_index)`. Lazily populated on first use of an id.
    subwallets: BTreeMap<SubwalletId, SubwalletState>,
}

impl FileBackedShieldedStore {
    /// Open or create a shielded store at `path`.
    pub fn open_path(
        path: impl AsRef<Path>,
        max_checkpoints: usize,
    ) -> Result<Self, FileShieldedStoreError> {
        let tree = ClientPersistentCommitmentTree::open_path(path, max_checkpoints)
            .map_err(|e| FileShieldedStoreError(format!("open commitment tree: {e}")))?;
        Ok(Self {
            tree: Mutex::new(tree),
            subwallets: BTreeMap::new(),
        })
    }
}

impl ShieldedStore for FileBackedShieldedStore {
    type Error = FileShieldedStoreError;

    fn save_note(&mut self, id: SubwalletId, note: &ShieldedNote) -> Result<(), Self::Error> {
        self.subwallets.entry(id).or_default().save_note(note);
        Ok(())
    }

    fn get_unspent_notes(&self, id: SubwalletId) -> Result<Vec<ShieldedNote>, Self::Error> {
        Ok(self
            .subwallets
            .get(&id)
            .map(SubwalletState::unspent_notes)
            .unwrap_or_default())
    }

    fn get_all_notes(&self, id: SubwalletId) -> Result<Vec<ShieldedNote>, Self::Error> {
        Ok(self
            .subwallets
            .get(&id)
            .map(SubwalletState::all_notes)
            .unwrap_or_default())
    }

    fn mark_spent(&mut self, id: SubwalletId, nullifier: &[u8; 32]) -> Result<bool, Self::Error> {
        Ok(self
            .subwallets
            .get_mut(&id)
            .map(|sw| sw.mark_spent(nullifier))
            .unwrap_or(false))
    }

    fn append_commitment(&mut self, cmx: &[u8; 32], marked: bool) -> Result<(), Self::Error> {
        let retention: Retention<u32> = if marked {
            Retention::Marked
        } else {
            Retention::Ephemeral
        };
        let mut tree = self
            .tree
            .lock()
            .map_err(|e| FileShieldedStoreError(format!("tree mutex poisoned: {e}")))?;
        tree.append(*cmx, retention)
            .map_err(|e| FileShieldedStoreError(format!("append commitment: {e}")))
    }

    fn checkpoint_tree(&mut self, checkpoint_id: u32) -> Result<(), Self::Error> {
        let mut tree = self
            .tree
            .lock()
            .map_err(|e| FileShieldedStoreError(format!("tree mutex poisoned: {e}")))?;
        tree.checkpoint(checkpoint_id)
            .map(|_| ())
            .map_err(|e| FileShieldedStoreError(format!("checkpoint tree: {e}")))
    }

    fn tree_anchor(&self) -> Result<[u8; 32], Self::Error> {
        let tree = self
            .tree
            .lock()
            .map_err(|e| FileShieldedStoreError(format!("tree mutex poisoned: {e}")))?;
        tree.anchor()
            .map(|a| a.to_bytes())
            .map_err(|e| FileShieldedStoreError(format!("read tree anchor: {e}")))
    }

    fn witness(
        &self,
        position: u64,
        checkpoint_depth: usize,
    ) -> Result<Option<grovedb_commitment_tree::MerklePath>, Self::Error> {
        let tree = self
            .tree
            .lock()
            .map_err(|e| FileShieldedStoreError(format!("tree mutex poisoned: {e}")))?;
        // `checkpoint_depth` indexes our local checkpoints (0 =
        // most recent, 1 = one back, ...). The spend path walks
        // depths to find one whose root matches a Platform-recorded
        // anchor — see `ShieldedWallet::find_anchor_depth`.
        tree.witness(Position::from(position), checkpoint_depth)
            .map_err(|e| {
                FileShieldedStoreError(format!(
                    "witness(position={position}, depth={checkpoint_depth}): {e}"
                ))
            })
    }

    fn last_synced_note_index(&self, id: SubwalletId) -> Result<u64, Self::Error> {
        Ok(self
            .subwallets
            .get(&id)
            .map(|sw| sw.last_synced_index)
            .unwrap_or(0))
    }

    fn set_last_synced_note_index(
        &mut self,
        id: SubwalletId,
        index: u64,
    ) -> Result<(), Self::Error> {
        self.subwallets.entry(id).or_default().last_synced_index = index;
        Ok(())
    }

    fn nullifier_checkpoint(&self, id: SubwalletId) -> Result<Option<(u64, u64)>, Self::Error> {
        Ok(self
            .subwallets
            .get(&id)
            .and_then(|sw| sw.nullifier_checkpoint))
    }

    fn set_nullifier_checkpoint(
        &mut self,
        id: SubwalletId,
        height: u64,
        timestamp: u64,
    ) -> Result<(), Self::Error> {
        self.subwallets.entry(id).or_default().nullifier_checkpoint = Some((height, timestamp));
        Ok(())
    }
}
