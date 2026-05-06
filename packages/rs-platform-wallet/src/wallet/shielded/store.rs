//! Storage abstraction for shielded wallet state.
//!
//! The `ShieldedStore` trait decouples `ShieldedWallet` from any particular
//! persistence backend. Consumers provide their own implementation (e.g.
//! SQLite-backed for production) while tests can use `InMemoryShieldedStore`.
//!
//! Note data is stored as raw bytes (`note_data: Vec<u8>`) — a serialized
//! `orchard::Note`. The witness path, however, is returned as a typed
//! `grovedb_commitment_tree::MerklePath`: that type doesn't implement
//! serde, so a bytes contract would force every caller through a
//! serializer that doesn't exist. Anything spending a note already
//! depends on these types via the DPP shielded builder.

use std::collections::BTreeMap;
use std::error::Error as StdError;
use std::fmt;

/// A note decrypted and owned by this wallet.
///
/// This struct carries all the bookkeeping fields needed by the shielded
/// wallet. The actual `orchard::Note` is stored as opaque bytes in
/// `note_data` so that the storage layer does not need to depend on the
/// Orchard crate.
#[derive(Debug, Clone)]
pub struct ShieldedNote {
    /// Global position in the commitment tree.
    pub position: u64,
    /// Extracted note commitment (32 bytes).
    pub cmx: [u8; 32],
    /// Nullifier for detecting when spent (32 bytes).
    pub nullifier: [u8; 32],
    /// Block height where the note appeared.
    pub block_height: u64,
    /// Whether the nullifier was seen on-chain (spent).
    pub is_spent: bool,
    /// Note value in credits.
    pub value: u64,
    /// Serialized `orchard::Note` bytes (115 bytes).
    /// Format: `recipient(43) || value(8 LE) || rho(32) || rseed(32)`.
    pub note_data: Vec<u8>,
}

/// Storage abstraction for shielded wallet state.
///
/// Consumers implement this for their persistence layer. The trait is
/// object-safe (no generics in method signatures) so it can be stored
/// behind `Arc<RwLock<dyn ShieldedStore>>` when needed.
///
/// All mutating methods take `&mut self` to allow the implementation to
/// batch writes or hold open transactions without interior mutability.
pub trait ShieldedStore: Send + Sync {
    /// The error type returned by storage operations.
    type Error: StdError + Send + Sync + 'static;

    // ── Notes ──────────────────────────────────────────────────────────

    /// Persist a newly decrypted note.
    fn save_note(&mut self, note: &ShieldedNote) -> Result<(), Self::Error>;

    /// Return all unspent (not yet nullified) notes.
    fn get_unspent_notes(&self) -> Result<Vec<ShieldedNote>, Self::Error>;

    /// Return all notes (both spent and unspent).
    fn get_all_notes(&self) -> Result<Vec<ShieldedNote>, Self::Error>;

    /// Mark the note identified by `nullifier` as spent.
    ///
    /// Returns `true` if a matching unspent note was found and marked,
    /// `false` if no unspent note has that nullifier.
    fn mark_spent(&mut self, nullifier: &[u8; 32]) -> Result<bool, Self::Error>;

    // ── Commitment tree ────────────────────────────────────────────────

    /// Append a note commitment to the commitment tree.
    ///
    /// `marked` indicates whether this position should be remembered for
    /// future witness generation (i.e. it belongs to this wallet).
    fn append_commitment(&mut self, cmx: &[u8; 32], marked: bool) -> Result<(), Self::Error>;

    /// Create a tree checkpoint at the given identifier.
    ///
    /// Checkpoints allow the tree to be rewound to this point if a sync
    /// batch needs to be rolled back.
    fn checkpoint_tree(&mut self, checkpoint_id: u32) -> Result<(), Self::Error>;

    /// Return the current tree root (Sinsemilla anchor, 32 bytes).
    fn tree_anchor(&self) -> Result<[u8; 32], Self::Error>;

    /// Generate a Merkle authentication path (witness) for the note at the
    /// given global position, against the current tree state.
    ///
    /// Returns `Ok(None)` if no witness is available (e.g. the position is
    /// not marked or the tree state has been pruned past it). Returns the
    /// typed `MerklePath` so callers can hand it directly to the Orchard
    /// spend builder; `MerklePath` doesn't implement serde, so a bytes
    /// variant would force every caller to round-trip through a
    /// non-existent serializer.
    ///
    /// This is needed when spending a note — the ZK proof must demonstrate
    /// that the note commitment exists in the tree at `anchor`.
    fn witness(
        &self,
        position: u64,
    ) -> Result<Option<grovedb_commitment_tree::MerklePath>, Self::Error>;

    // ── Sync state ─────────────────────────────────────────────────────

    /// The last global note index that was synced from Platform.
    fn last_synced_note_index(&self) -> Result<u64, Self::Error>;

    /// Persist the last synced note index.
    fn set_last_synced_note_index(&mut self, index: u64) -> Result<(), Self::Error>;

    /// The last nullifier sync checkpoint, if any.
    ///
    /// Returns `(height, timestamp)` from the most recent nullifier sync.
    fn nullifier_checkpoint(&self) -> Result<Option<(u64, u64)>, Self::Error>;

    /// Persist the nullifier sync checkpoint.
    fn set_nullifier_checkpoint(&mut self, height: u64, timestamp: u64) -> Result<(), Self::Error>;
}

// ── InMemoryShieldedStore ──────────────────────────────────────────────

/// Trivial error type for the in-memory store (infallible in practice).
#[derive(Debug, Clone)]
pub struct InMemoryStoreError(String);

impl fmt::Display for InMemoryStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl StdError for InMemoryStoreError {}

/// In-memory implementation of [`ShieldedStore`] for tests and short-lived
/// wallets.
///
/// Notes are stored in a `Vec`; the commitment tree is represented as a flat
/// list of commitments (sufficient for anchor computation via the incremental
/// merkle tree crate, but witness generation is **not** implemented — use a
/// real store for operations that require Merkle paths).
#[derive(Debug)]
pub struct InMemoryShieldedStore {
    /// All notes, keyed by nullifier for O(1) lookup during `mark_spent`.
    notes: Vec<ShieldedNote>,
    /// Nullifier -> index into `notes` for fast spend marking.
    nullifier_index: BTreeMap<[u8; 32], usize>,
    /// Flat list of commitments appended to the tree.
    commitments: Vec<[u8; 32]>,
    /// Positions that are marked (belong to this wallet).
    marked_positions: Vec<bool>,
    /// Checkpoint IDs in order.
    checkpoints: Vec<u32>,
    /// Current anchor (recomputed lazily — for the in-memory store we
    /// store a dummy zero value; production stores compute from the real tree).
    anchor: [u8; 32],
    /// Last synced note index.
    last_synced_index: u64,
    /// Nullifier sync checkpoint: `(height, timestamp)`.
    nullifier_checkpoint: Option<(u64, u64)>,
}

impl InMemoryShieldedStore {
    /// Create a new empty in-memory store.
    pub fn new() -> Self {
        Self {
            notes: Vec::new(),
            nullifier_index: BTreeMap::new(),
            commitments: Vec::new(),
            marked_positions: Vec::new(),
            checkpoints: Vec::new(),
            anchor: [0u8; 32],
            last_synced_index: 0,
            nullifier_checkpoint: None,
        }
    }
}

impl Default for InMemoryShieldedStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ShieldedStore for InMemoryShieldedStore {
    type Error = InMemoryStoreError;

    fn save_note(&mut self, note: &ShieldedNote) -> Result<(), Self::Error> {
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
        self.commitments.push(*cmx);
        self.marked_positions.push(marked);
        Ok(())
    }

    fn checkpoint_tree(&mut self, checkpoint_id: u32) -> Result<(), Self::Error> {
        self.checkpoints.push(checkpoint_id);
        Ok(())
    }

    fn tree_anchor(&self) -> Result<[u8; 32], Self::Error> {
        // The in-memory store returns a dummy anchor.
        // Production implementations should compute the real Sinsemilla root.
        Ok(self.anchor)
    }

    fn witness(
        &self,
        _position: u64,
    ) -> Result<Option<grovedb_commitment_tree::MerklePath>, Self::Error> {
        // In-memory store does not support real Merkle witness generation.
        // Production implementations use ClientPersistentCommitmentTree.
        Err(InMemoryStoreError(
            "Merkle witness not supported in in-memory store".into(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_and_retrieve_notes() {
        let mut store = InMemoryShieldedStore::new();
        let note = ShieldedNote {
            position: 42,
            cmx: [1u8; 32],
            nullifier: [2u8; 32],
            block_height: 100,
            is_spent: false,
            value: 1000,
            note_data: vec![0u8; 115],
        };
        store.save_note(&note).unwrap();

        let unspent = store.get_unspent_notes().unwrap();
        assert_eq!(unspent.len(), 1);
        assert_eq!(unspent[0].value, 1000);
        assert_eq!(unspent[0].position, 42);
    }

    #[test]
    fn test_mark_spent() {
        let mut store = InMemoryShieldedStore::new();
        let nullifier = [3u8; 32];
        let note = ShieldedNote {
            position: 0,
            cmx: [1u8; 32],
            nullifier,
            block_height: 50,
            is_spent: false,
            value: 500,
            note_data: vec![0u8; 115],
        };
        store.save_note(&note).unwrap();

        // Mark spent
        let found = store.mark_spent(&nullifier).unwrap();
        assert!(found);

        // Should no longer appear in unspent
        let unspent = store.get_unspent_notes().unwrap();
        assert!(unspent.is_empty());

        // But should appear in all notes
        let all = store.get_all_notes().unwrap();
        assert_eq!(all.len(), 1);
        assert!(all[0].is_spent);

        // Marking again returns false
        let found_again = store.mark_spent(&nullifier).unwrap();
        assert!(!found_again);
    }

    #[test]
    fn test_sync_state() {
        let mut store = InMemoryShieldedStore::new();

        assert_eq!(store.last_synced_note_index().unwrap(), 0);
        store.set_last_synced_note_index(100).unwrap();
        assert_eq!(store.last_synced_note_index().unwrap(), 100);

        assert!(store.nullifier_checkpoint().unwrap().is_none());
        store.set_nullifier_checkpoint(200, 1234567890).unwrap();
        assert_eq!(
            store.nullifier_checkpoint().unwrap(),
            Some((200, 1234567890))
        );
    }

    #[test]
    fn test_commitment_tree_operations() {
        let mut store = InMemoryShieldedStore::new();

        store.append_commitment(&[1u8; 32], true).unwrap();
        store.append_commitment(&[2u8; 32], false).unwrap();
        store.checkpoint_tree(1).unwrap();

        // Anchor is dummy for in-memory
        let anchor = store.tree_anchor().unwrap();
        assert_eq!(anchor, [0u8; 32]);
    }
}
