//! Storage abstraction for shielded wallet state.
//!
//! The `ShieldedStore` trait decouples `ShieldedWallet` from any
//! particular persistence backend. Consumers provide their own
//! implementation (e.g. SwiftData via the host persister) while
//! tests can use [`InMemoryShieldedStore`].
//!
//! # Multi-tenant scoping
//!
//! Decrypted notes, nullifier bookkeeping, and per-account sync
//! watermarks are scoped by [`SubwalletId`] (a `(wallet_id,
//! account_index)` tuple) so a single store can host every wallet
//! and every shielded account on the same network. The Orchard
//! commitment tree itself is **not** scoped — the on-chain
//! commitment stream is identical for every consumer on a given
//! network, so one tree backs them all.
//!
//! # Note format
//!
//! `ShieldedNote::note_data` is a serialized `orchard::Note` (115
//! bytes). The witness path returned by [`ShieldedStore::witness`]
//! is the typed `grovedb_commitment_tree::MerklePath` because that
//! type doesn't implement serde — a bytes contract would force
//! every caller through a serializer that doesn't exist.

use std::collections::BTreeMap;
use std::error::Error as StdError;
use std::fmt;

/// Identifies a single shielded "subwallet" — one Orchard account
/// within one wallet. Used to scope notes, nullifier indices, and
/// sync watermarks inside a [`ShieldedStore`] so a single store
/// can hold state for many wallets/accounts without leakage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubwalletId {
    /// 32-byte wallet identifier (matches `PlatformWallet::wallet_id`).
    pub wallet_id: [u8; 32],
    /// ZIP-32 account index (`m / 32' / coin_type' / account'`).
    pub account_index: u32,
}

impl SubwalletId {
    /// Construct a [`SubwalletId`] from its parts.
    pub fn new(wallet_id: [u8; 32], account_index: u32) -> Self {
        Self {
            wallet_id,
            account_index,
        }
    }
}

/// A note decrypted and owned by a specific subwallet.
///
/// Carries the bookkeeping the spend pipeline needs without
/// pulling the orchard crate into this trait. The actual
/// `orchard::Note` is in `note_data` as 115 bytes
/// (`recipient(43) || value(8 LE) || rho(32) || rseed(32)`).
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
    pub note_data: Vec<u8>,
}

/// Storage abstraction for shielded wallet state.
///
/// Consumers implement this for their persistence layer. The
/// trait is object-safe (no generics on method signatures) so it
/// can be stored behind `Arc<RwLock<dyn ShieldedStore>>`.
///
/// All mutating methods take `&mut self` so implementations can
/// batch writes without interior mutability.
pub trait ShieldedStore: Send + Sync {
    /// The error type returned by storage operations.
    type Error: StdError + Send + Sync + 'static;

    // ── Notes (per-subwallet) ──────────────────────────────────────────

    /// Persist a newly decrypted note for `id`.
    fn save_note(&mut self, id: SubwalletId, note: &ShieldedNote) -> Result<(), Self::Error>;

    /// Return all unspent notes for `id`.
    fn get_unspent_notes(&self, id: SubwalletId) -> Result<Vec<ShieldedNote>, Self::Error>;

    /// Return all notes (spent and unspent) for `id`.
    fn get_all_notes(&self, id: SubwalletId) -> Result<Vec<ShieldedNote>, Self::Error>;

    /// Mark `id`'s note with `nullifier` as spent. Returns `true`
    /// if a matching unspent note was found.
    fn mark_spent(&mut self, id: SubwalletId, nullifier: &[u8; 32]) -> Result<bool, Self::Error>;

    // ── Commitment tree (network-shared) ───────────────────────────────

    /// Append a note commitment to the shared tree.
    ///
    /// `marked` should be `true` if **any** tracked subwallet owns
    /// this position. The tree only retains authentication paths
    /// for marked positions; unmarked positions are pruned.
    fn append_commitment(&mut self, cmx: &[u8; 32], marked: bool) -> Result<(), Self::Error>;

    /// Create a tree checkpoint at the given identifier.
    fn checkpoint_tree(&mut self, checkpoint_id: u32) -> Result<(), Self::Error>;

    /// Return the current tree root (Sinsemilla anchor, 32 bytes).
    fn tree_anchor(&self) -> Result<[u8; 32], Self::Error>;

    /// Generate a Merkle authentication path for `position`
    /// against the current tree state. Returns `Ok(None)` if no
    /// witness is available (position not marked, or pruned).
    fn witness(
        &self,
        position: u64,
    ) -> Result<Option<grovedb_commitment_tree::MerklePath>, Self::Error>;

    // ── Sync state (per-subwallet) ─────────────────────────────────────

    /// The last global note index that was synced for `id`.
    fn last_synced_note_index(&self, id: SubwalletId) -> Result<u64, Self::Error>;

    /// Persist the last synced note index for `id`.
    fn set_last_synced_note_index(
        &mut self,
        id: SubwalletId,
        index: u64,
    ) -> Result<(), Self::Error>;

    /// The last `(height, timestamp)` nullifier sync checkpoint for `id`, if any.
    fn nullifier_checkpoint(&self, id: SubwalletId) -> Result<Option<(u64, u64)>, Self::Error>;

    /// Persist the nullifier sync checkpoint for `id`.
    fn set_nullifier_checkpoint(
        &mut self,
        id: SubwalletId,
        height: u64,
        timestamp: u64,
    ) -> Result<(), Self::Error>;
}

// ── Per-subwallet bookkeeping ──────────────────────────────────────────

/// Per-subwallet note + sync state used by both the in-memory and
/// file-backed stores. Kept in this module so both share the
/// exact same shape and the persister callback can serialize it
/// without re-defining the structure on the host side.
#[derive(Debug, Default, Clone)]
pub(super) struct SubwalletState {
    /// All known notes (spent + unspent), in insertion order.
    pub notes: Vec<ShieldedNote>,
    /// Nullifier → index into `notes`, for O(1) `mark_spent`.
    pub nullifier_index: BTreeMap<[u8; 32], usize>,
    /// Highest global note index ever scanned.
    pub last_synced_index: u64,
    /// `(height, timestamp)` from the most recent nullifier sync.
    pub nullifier_checkpoint: Option<(u64, u64)>,
}

impl SubwalletState {
    /// Save (or overwrite-by-nullifier) a note.
    ///
    /// Re-saving a note with a known nullifier overwrites the
    /// existing entry instead of appending a duplicate — Orchard
    /// nullifiers are globally unique, so a re-scan of the same
    /// chunk shouldn't double-count.
    pub(super) fn save_note(&mut self, note: &ShieldedNote) {
        if let Some(&existing_idx) = self.nullifier_index.get(&note.nullifier) {
            self.notes[existing_idx] = note.clone();
            return;
        }
        let idx = self.notes.len();
        self.nullifier_index.insert(note.nullifier, idx);
        self.notes.push(note.clone());
    }

    pub(super) fn unspent_notes(&self) -> Vec<ShieldedNote> {
        self.notes.iter().filter(|n| !n.is_spent).cloned().collect()
    }

    pub(super) fn all_notes(&self) -> Vec<ShieldedNote> {
        self.notes.clone()
    }

    pub(super) fn mark_spent(&mut self, nullifier: &[u8; 32]) -> bool {
        if let Some(&idx) = self.nullifier_index.get(nullifier) {
            if !self.notes[idx].is_spent {
                self.notes[idx].is_spent = true;
                return true;
            }
        }
        false
    }
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

/// In-memory implementation of [`ShieldedStore`] for tests and
/// short-lived wallets. Notes are kept per [`SubwalletId`]; the
/// commitment tree is a flat list (anchor is a placeholder, so
/// real witness generation is **not** supported — use a real
/// store for spends).
#[derive(Debug, Default)]
pub struct InMemoryShieldedStore {
    /// Per-subwallet notes + sync state.
    subwallets: BTreeMap<SubwalletId, SubwalletState>,
    /// Flat list of commitments appended to the tree.
    commitments: Vec<[u8; 32]>,
    /// Mark flag per position.
    marked_positions: Vec<bool>,
    /// Checkpoint ids in order.
    checkpoints: Vec<u32>,
    /// Placeholder anchor; production stores compute the real Sinsemilla root.
    anchor: [u8; 32],
}

impl InMemoryShieldedStore {
    /// Create a new empty in-memory store.
    pub fn new() -> Self {
        Self::default()
    }
}

impl ShieldedStore for InMemoryShieldedStore {
    type Error = InMemoryStoreError;

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
        self.commitments.push(*cmx);
        self.marked_positions.push(marked);
        Ok(())
    }

    fn checkpoint_tree(&mut self, checkpoint_id: u32) -> Result<(), Self::Error> {
        self.checkpoints.push(checkpoint_id);
        Ok(())
    }

    fn tree_anchor(&self) -> Result<[u8; 32], Self::Error> {
        Ok(self.anchor)
    }

    fn witness(
        &self,
        _position: u64,
    ) -> Result<Option<grovedb_commitment_tree::MerklePath>, Self::Error> {
        Err(InMemoryStoreError(
            "Merkle witness not supported in in-memory store".into(),
        ))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_id(account: u32) -> SubwalletId {
        SubwalletId::new([0xAA; 32], account)
    }

    #[test]
    fn test_save_and_retrieve_notes() {
        let mut store = InMemoryShieldedStore::new();
        let id = test_id(0);
        let note = ShieldedNote {
            position: 42,
            cmx: [1u8; 32],
            nullifier: [2u8; 32],
            block_height: 100,
            is_spent: false,
            value: 1000,
            note_data: vec![0u8; 115],
        };
        store.save_note(id, &note).unwrap();

        let unspent = store.get_unspent_notes(id).unwrap();
        assert_eq!(unspent.len(), 1);
        assert_eq!(unspent[0].value, 1000);
        assert_eq!(unspent[0].position, 42);

        // A different subwallet sees no notes.
        let other = test_id(1);
        assert!(store.get_unspent_notes(other).unwrap().is_empty());
    }

    #[test]
    fn test_mark_spent() {
        let mut store = InMemoryShieldedStore::new();
        let id = test_id(0);
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
        store.save_note(id, &note).unwrap();

        assert!(store.mark_spent(id, &nullifier).unwrap());
        assert!(store.get_unspent_notes(id).unwrap().is_empty());
        let all = store.get_all_notes(id).unwrap();
        assert_eq!(all.len(), 1);
        assert!(all[0].is_spent);
        // Marking again returns false (already spent).
        assert!(!store.mark_spent(id, &nullifier).unwrap());
    }

    #[test]
    fn test_sync_state_per_subwallet() {
        let mut store = InMemoryShieldedStore::new();
        let a = test_id(0);
        let b = test_id(1);

        assert_eq!(store.last_synced_note_index(a).unwrap(), 0);
        store.set_last_synced_note_index(a, 100).unwrap();
        assert_eq!(store.last_synced_note_index(a).unwrap(), 100);
        // Different subwallet still at 0.
        assert_eq!(store.last_synced_note_index(b).unwrap(), 0);

        store.set_nullifier_checkpoint(a, 200, 1234567890).unwrap();
        assert_eq!(
            store.nullifier_checkpoint(a).unwrap(),
            Some((200, 1234567890))
        );
        assert!(store.nullifier_checkpoint(b).unwrap().is_none());
    }

    #[test]
    fn test_commitment_tree_operations() {
        let mut store = InMemoryShieldedStore::new();
        store.append_commitment(&[1u8; 32], true).unwrap();
        store.append_commitment(&[2u8; 32], false).unwrap();
        store.checkpoint_tree(1).unwrap();
        assert_eq!(store.tree_anchor().unwrap(), [0u8; 32]);
    }
}
