//! File-backed `ShieldedStore` impl.
//!
//! The Orchard commitment tree is shared across every wallet that
//! decrypts notes against the same network — there is one global
//! tree of commitments and each wallet keeps its own decrypted-note
//! subset. This store therefore persists the tree to a SQLite file
//! (via [`ClientPersistentCommitmentTree`]) while keeping the
//! per-wallet decrypted notes and nullifier bookkeeping in memory.
//! Notes are rediscovered on cold start by re-running
//! [`ShieldedWallet::sync_notes`](super::ShieldedWallet::sync_notes)
//! against the cached tree.
//!
//! Witness generation (needed for spends) is intentionally not
//! implemented yet — the spend signer pipeline that drives it lands
//! in a follow-up.

use std::collections::BTreeMap;
use std::error::Error as StdError;
use std::fmt;
use std::path::Path;
use std::sync::Mutex;

use grovedb_commitment_tree::{ClientPersistentCommitmentTree, Position, Retention};

use super::store::{ShieldedNote, ShieldedStore};

/// Error type for [`FileBackedShieldedStore`].
#[derive(Debug)]
pub struct FileShieldedStoreError(pub String);

impl fmt::Display for FileShieldedStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl StdError for FileShieldedStoreError {}

/// File-backed shielded store: SQLite-persisted commitment tree plus
/// in-memory decrypted notes / nullifier bookkeeping.
///
/// The commitment tree is keyed per-network at the call site (the
/// path is supplied by [`Self::open_path`]). Decrypted notes are
/// kept in memory and rediscovered via trial decryption on every
/// cold start — same shape the previous `ShieldedPoolClient` had,
/// suitable for the MVP shielded sync path. Persisting notes via
/// the host's data store is a follow-up.
pub struct FileBackedShieldedStore {
    /// SQLite-backed commitment tree. Wrapped in a `Mutex` rather than
    /// relying on `&mut self` because the underlying SQLite store is
    /// not `Sync` on its own and the [`ShieldedStore`] trait requires
    /// `Send + Sync`. Outer concurrency is still serialized through
    /// `ShieldedWallet`'s `RwLock<S>`; this inner mutex is just a
    /// `Sync`-restoring shim and is uncontended in practice.
    tree: Mutex<ClientPersistentCommitmentTree>,
    notes: Vec<ShieldedNote>,
    /// Nullifier → index into `notes`, for `mark_spent` lookups.
    nullifier_index: BTreeMap<[u8; 32], usize>,
    /// Last global note index synced from Platform.
    last_synced_index: u64,
    /// `(height, timestamp)` from the most recent nullifier sync.
    nullifier_checkpoint: Option<(u64, u64)>,
}

impl FileBackedShieldedStore {
    /// Open or create a shielded store at `path`.
    ///
    /// `max_checkpoints` controls how many tree checkpoints the
    /// underlying [`ClientPersistentCommitmentTree`] retains for
    /// witness generation. A value of `100` matches what the previous
    /// SDK-side client used.
    pub fn open_path(
        path: impl AsRef<Path>,
        max_checkpoints: usize,
    ) -> Result<Self, FileShieldedStoreError> {
        let tree = ClientPersistentCommitmentTree::open_path(path, max_checkpoints)
            .map_err(|e| FileShieldedStoreError(format!("open commitment tree: {e}")))?;
        Ok(Self {
            tree: Mutex::new(tree),
            notes: Vec::new(),
            nullifier_index: BTreeMap::new(),
            last_synced_index: 0,
            nullifier_checkpoint: None,
        })
    }
}

impl ShieldedStore for FileBackedShieldedStore {
    type Error = FileShieldedStoreError;

    fn save_note(&mut self, note: &ShieldedNote) -> Result<(), Self::Error> {
        // Re-saving an already-known note (e.g. a re-scan after a
        // cold start trial-decrypts the same chunk) used to append
        // a duplicate `ShieldedNote` while overwriting the
        // nullifier index. The result was a double-counted balance
        // (`get_unspent_notes` returned both copies) and a stuck
        // unspent flag (`mark_spent` only marked the second copy).
        // Orchard nullifiers are globally unique, so an existing
        // entry for the same nullifier means we already have this
        // note — overwrite-in-place rather than append.
        if let Some(&existing_idx) = self.nullifier_index.get(&note.nullifier) {
            self.notes[existing_idx] = note.clone();
            return Ok(());
        }
        let idx = self.notes.len();
        self.nullifier_index.insert(note.nullifier, idx);
        self.notes.push(note.clone());
        Ok(())
    }

    fn get_unspent_notes(&self) -> Result<Vec<ShieldedNote>, Self::Error> {
        Ok(self.notes.iter().filter(|n| !n.is_spent).cloned().collect())
    }

    fn get_all_notes(&self) -> Result<Vec<ShieldedNote>, Self::Error> {
        Ok(self.notes.clone())
    }

    fn mark_spent(&mut self, nullifier: &[u8; 32]) -> Result<bool, Self::Error> {
        if let Some(&idx) = self.nullifier_index.get(nullifier) {
            if !self.notes[idx].is_spent {
                self.notes[idx].is_spent = true;
                return Ok(true);
            }
        }
        Ok(false)
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

    fn witness(&self, _position: u64) -> Result<Vec<u8>, Self::Error> {
        // Witness path serialization lives with the spend signer; the
        // sync path doesn't call this, and spend ops haven't been
        // routed back through `ShieldedStore` yet.
        let _ = Position::from(_position); // keep the import alive
        Err(FileShieldedStoreError(
            "witness generation deferred until spend signer lands".into(),
        ))
    }

    fn last_synced_note_index(&self) -> Result<u64, Self::Error> {
        Ok(self.last_synced_index)
    }

    fn set_last_synced_note_index(&mut self, index: u64) -> Result<(), Self::Error> {
        self.last_synced_index = index;
        Ok(())
    }

    fn nullifier_checkpoint(&self) -> Result<Option<(u64, u64)>, Self::Error> {
        Ok(self.nullifier_checkpoint)
    }

    fn set_nullifier_checkpoint(&mut self, height: u64, timestamp: u64) -> Result<(), Self::Error> {
        self.nullifier_checkpoint = Some((height, timestamp));
        Ok(())
    }
}
