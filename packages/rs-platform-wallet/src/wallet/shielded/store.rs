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

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error as StdError;
use std::fmt;

use crate::wallet::platform_wallet::WalletId;

/// Identifies a single shielded "subwallet" — one Orchard account
/// within one wallet. Used to scope notes, nullifier indices, and
/// sync watermarks inside a [`ShieldedStore`] so a single store
/// can hold state for many wallets/accounts without leakage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ShieldedNote {
    /// Global position in the commitment tree.
    pub position: u64,
    /// Extracted note commitment (32 bytes).
    pub cmx: [u8; 32],
    /// Nullifier for detecting when spent (32 bytes).
    pub nullifier: [u8; 32],
    /// Proven platform height of the chunk fetch that surfaced the note.
    /// Stamped per-batch — the SAME height OVK-recovered outgoing notes
    /// from that chunk get — so the activity deriver can cluster one
    /// bundle's incoming change and outgoing send together by height.
    pub block_height: u64,
    /// Whether the nullifier was seen on-chain (spent).
    pub is_spent: bool,
    /// Note value in credits.
    pub value: u64,
    /// Serialized `orchard::Note` bytes (115 bytes).
    pub note_data: Vec<u8>,
}

/// A note this subwallet **sent**, recovered during the note scan via
/// the wallet's Outgoing Viewing Key (the Zcash outgoing-transaction-
/// history mechanism).
///
/// Unlike [`ShieldedNote`] (which is a note the wallet *received* and
/// can later spend), this is a minimal record of an *outgoing*
/// payment — who the wallet paid, how much, and with what memo —
/// kept purely for send-history display. It carries no spend
/// bookkeeping (no nullifier / position / witness) because the wallet
/// cannot spend a note it sent to someone else.
///
/// Keyed by `cmx` (the recovered output note's commitment), which is
/// globally unique on-chain, so recording the same recovered note
/// twice (a re-scan of the same chunk) is idempotent.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ShieldedOutgoingNote {
    /// Extracted note commitment (32 bytes) of the note that was sent.
    /// Primary key — unique per on-chain output.
    pub cmx: [u8; 32],
    /// Recipient's raw Orchard address (43 bytes,
    /// `Address::to_raw_address_bytes`). The wallet paid this address.
    /// Stored as a `Vec` rather than `[u8; 43]` because `serde`'s derive
    /// only covers fixed arrays up to length 32; always 43 bytes for a
    /// recovered note.
    pub recipient: Vec<u8>,
    /// Value sent, in credits.
    pub value: u64,
    /// Raw Dash memo bytes (`DashMemo`, 36 bytes). Stored as a `Vec`
    /// rather than `[u8; 36]` so the persisted shape stays flexible if
    /// the memo size ever changes; always 36 bytes for a recovered note.
    pub memo: Vec<u8>,
    /// Block height at which the sent note appeared on-chain.
    pub block_height: u64,
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

    /// Reserve `id`'s note with `nullifier` against an in-flight
    /// spend so concurrent callers can't pick the same note.
    /// Returns `true` if the nullifier was newly added to the
    /// pending set, `false` if it was already pending.
    ///
    /// Pending state is **in-memory only** — it does not survive
    /// a process restart. The crash-during-broadcast case is
    /// reconciled by the next note-scan pass after the transition
    /// lands (scan-based spend detection marks the spent note via
    /// its nullifier), or, on rejection, leaves the notes
    /// observable as unspent again on the next launch.
    ///
    /// `unspent_notes` skips notes whose nullifier is in the
    /// pending set, so a successful `mark_pending` immediately
    /// removes the note from selection candidates.
    fn mark_pending(&mut self, id: SubwalletId, nullifier: &[u8; 32]) -> Result<bool, Self::Error>;

    /// Release the reservation taken by [`Self::mark_pending`].
    /// Returns `true` if the nullifier was actually pending and
    /// got removed; `false` is a no-op (paired clear from the
    /// rollback path on a transition that never marked pending,
    /// or a stale clear after the spend already promoted to
    /// `mark_spent`).
    fn clear_pending(&mut self, id: SubwalletId, nullifier: &[u8; 32])
        -> Result<bool, Self::Error>;

    // ── Outgoing history (per-subwallet) ───────────────────────────────

    /// Record an outgoing (sent) note recovered via OVK for `id`.
    ///
    /// Idempotent by `note.cmx`: re-recording a note already on file
    /// (a re-scan of the same chunk) is a no-op and returns `false`;
    /// a genuinely new outgoing note is stored and returns `true`.
    /// Outgoing notes are append-only send history — there is no
    /// "mark spent" / mutation path for them.
    fn record_outgoing_note(
        &mut self,
        id: SubwalletId,
        note: &ShieldedOutgoingNote,
    ) -> Result<bool, Self::Error>;

    /// Return every outgoing (sent) note recovered for `id`, in the
    /// order they were recorded.
    fn get_outgoing_notes(&self, id: SubwalletId)
        -> Result<Vec<ShieldedOutgoingNote>, Self::Error>;

    // ── Derived activity log (per-subwallet) ───────────────────────────

    /// Upsert a derived [`ShieldedActivityEntry`] for `id`, keyed by
    /// `entry.id` (the sha256 of the visible output cmxs — see
    /// [`crate::wallet::shielded::activity`]).
    ///
    /// Re-saving an entry with the same `entry.id` overwrites the
    /// existing one in place. This is what lets a coarse scan-derived
    /// `ShieldedSpend` be upgraded to a specific kind when a later live
    /// entry (or correlation pass) re-emits the same id, and what lets a
    /// `Pending` entry flip to `Confirmed`/`Failed`.
    fn save_activity(
        &mut self,
        id: SubwalletId,
        entry: &super::activity::ShieldedActivityEntry,
    ) -> Result<(), Self::Error>;

    /// Return a page of derived activity for `id`, sorted for display
    /// (pendings first, then by `block_height` desc, tiebreak by
    /// `created_at_ms` then `id`), sliced by `[offset, offset+limit)`.
    /// `limit == 0` returns an empty page.
    fn get_activity(
        &self,
        id: SubwalletId,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<super::activity::ShieldedActivityEntry>, Self::Error>;

    /// Look up a single activity entry by its `entry.id`. `None` if no
    /// entry with that id exists for `id`.
    fn get_activity_by_entry_id(
        &self,
        id: SubwalletId,
        entry_id: &[u8; 32],
    ) -> Result<Option<super::activity::ShieldedActivityEntry>, Self::Error>;

    /// Return the set of all entry ids already recorded for `id`. Used by
    /// the scan deriver to skip clusters a live entry already owns.
    fn get_activity_ids(&self, id: SubwalletId) -> Result<BTreeSet<[u8; 32]>, Self::Error>;

    // ── Commitment tree (network-shared) ───────────────────────────────

    /// Append a note commitment to the shared tree.
    ///
    /// `marked` controls whether shardtree retains the
    /// authentication path for this position (Marked) or prunes it
    /// (Ephemeral). The sync path passes `true` for **every**
    /// position: the tree is a chain-wide structure shared by all
    /// wallets, and wallets bind at different times, so deciding
    /// retention from "is this position owned right now" loses the
    /// ability to witness a note whose owner binds later
    /// (shardtree can't retroactively mark). Per-wallet ownership
    /// is tracked separately in the per-`SubwalletId` note store.
    /// The `marked` parameter is kept for store-level flexibility
    /// and tests.
    fn append_commitment(&mut self, cmx: &[u8; 32], marked: bool) -> Result<(), Self::Error>;

    /// Create a tree checkpoint at the given identifier.
    fn checkpoint_tree(&mut self, checkpoint_id: u32) -> Result<(), Self::Error>;

    /// Return the current tree root (Sinsemilla anchor, 32 bytes).
    fn tree_anchor(&self) -> Result<[u8; 32], Self::Error>;

    /// Generate a Merkle authentication path for `position` as of the
    /// checkpoint at `depth` (0 = current tree state, 1 = the previous
    /// checkpoint, and so on). Returns `Ok(None)` when no witness is
    /// available at that depth — the position is unmarked/pruned, the
    /// requested checkpoint depth doesn't exist, or the position was
    /// appended after that checkpoint.
    ///
    /// Building a spend against an older-but-recorded checkpoint is how the
    /// wallet keeps its anchor consistent with a root Platform actually
    /// recorded: Platform records one anchor per block while an index-chunk
    /// sync routinely leaves the tree mid-block, so the depth-0 root is often
    /// one Platform never recorded.
    fn witness_at_depth(
        &self,
        position: u64,
        depth: usize,
    ) -> Result<Option<grovedb_commitment_tree::MerklePath>, Self::Error>;

    /// Generate a Merkle authentication path for `position` against the
    /// current tree state. Returns `Ok(None)` if no witness is available
    /// (position not marked, or pruned).
    ///
    /// Delegates to [`Self::witness_at_depth`] at depth 0.
    fn witness(
        &self,
        position: u64,
    ) -> Result<Option<grovedb_commitment_tree::MerklePath>, Self::Error> {
        self.witness_at_depth(position, 0)
    }

    /// Number of leaves currently in the shared commitment tree
    /// (= highest appended position + 1, or 0 when empty).
    ///
    /// This is the append watermark for the tree itself, distinct
    /// from any per-subwallet `last_synced_note_index`. The sync
    /// path gates [`Self::append_commitment`] on this value — never
    /// on a per-subwallet watermark — so a re-fetch from a chunk
    /// boundary (forced when a lagging subwallet rewinds the fetch
    /// start) re-appends nothing already in the tree. Double-append
    /// corrupts the shardtree's internal nodes and makes per-position
    /// witnesses resolve against inconsistent roots ("Anchor not
    /// found in the recorded anchors tree" at spend time).
    fn tree_size(&self) -> Result<u64, Self::Error>;

    // ── Sync state (per-subwallet) ─────────────────────────────────────

    /// Sync watermark for `id`: the count of note positions already
    /// scanned, i.e. the next global commitment-tree index to scan.
    /// `0` means nothing scanned yet; `N` means positions `0..N` are
    /// done. Exclusive upper bound — *not* the last index scanned
    /// (scanning through position `N-1` sets this to `N`).
    fn last_synced_note_index(&self, id: SubwalletId) -> Result<u64, Self::Error>;

    /// Persist the sync watermark (next index to scan) for `id`.
    fn set_last_synced_note_index(
        &mut self,
        id: SubwalletId,
        index: u64,
    ) -> Result<(), Self::Error>;

    // ── Per-subwallet lifecycle ────────────────────────────────────────

    /// Drop ALL in-memory per-subwallet state (decrypted notes,
    /// spent marks, `last_synced_note_index`, nullifier
    /// checkpoints, pending reservations) for every subwallet
    /// belonging to `wallet_id`. The shared commitment tree is
    /// left untouched — it's a chain-wide structure, not
    /// per-wallet. Used when a wallet is removed or its shielded
    /// binding is cleared so a later re-bind resyncs from index 0
    /// rather than resuming behind the stale watermark.
    fn purge_wallet(&mut self, wallet_id: WalletId) -> Result<(), Self::Error>;

    /// Drop ALL in-memory per-subwallet state for every subwallet
    /// of every wallet. The shared commitment tree is left
    /// untouched. Used by `NetworkShieldedCoordinator::clear()`.
    fn purge_all_subwallets(&mut self) -> Result<(), Self::Error>;

    /// Empty the shared commitment tree back to zero leaves.
    ///
    /// After this returns, [`Self::tree_size`] reports `0` and the
    /// next [`Self::append_commitment`] starts at position `0`. The
    /// per-subwallet watermarks ([`Self::last_synced_note_index`])
    /// are *not* touched here — callers that want a cold rebuild
    /// pair this with [`Self::purge_all_subwallets`] so the
    /// re-download watermark and the tree reset together.
    ///
    /// Used by `NetworkShieldedCoordinator::clear()` so the host's
    /// "Clear" action is a true cold reset rather than a watermark
    /// rewind into an already-full tree. Without it, Clear leaves
    /// the tree at its full size while the watermark drops to 0, so
    /// every re-fetched position is gate-skipped (`global_pos <
    /// tree_size`) and the "Checked" progress bar stays pinned at
    /// the stale leaf count while "Downloaded" climbs from 0.
    fn reset_commitment_tree(&mut self) -> Result<(), Self::Error>;
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
    /// Sync watermark: count of note positions scanned = the next
    /// global index to scan (exclusive). `0` = nothing scanned yet.
    pub last_synced_index: u64,
    /// Nullifiers of notes currently being spent in an in-flight
    /// transition. Excluded from `unspent_notes()` so concurrent
    /// callers can't double-select. In-memory only — never
    /// persisted; the next sync after a crash reconciles state.
    pub pending_nullifiers: BTreeSet<[u8; 32]>,
    /// Notes this subwallet SENT, recovered via OVK during the scan.
    /// Append-only send history in recording order.
    pub outgoing_notes: Vec<ShieldedOutgoingNote>,
    /// `cmx` set of recorded outgoing notes, for O(log n) idempotency
    /// on `record_outgoing_note` (Orchard cmx is globally unique).
    pub outgoing_cmx_index: BTreeSet<[u8; 32]>,
    /// Derived activity entries keyed by `entry.id` (sha256 of the
    /// visible output cmxs). Upsert-by-id: a later live entry or a
    /// correlation pass that re-emits the same id overwrites the row.
    pub activity: BTreeMap<[u8; 32], super::activity::ShieldedActivityEntry>,
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
        self.notes
            .iter()
            .filter(|n| !n.is_spent && !self.pending_nullifiers.contains(&n.nullifier))
            .cloned()
            .collect()
    }

    pub(super) fn all_notes(&self) -> Vec<ShieldedNote> {
        self.notes.clone()
    }

    pub(super) fn mark_spent(&mut self, nullifier: &[u8; 32]) -> bool {
        if let Some(&idx) = self.nullifier_index.get(nullifier) {
            if !self.notes[idx].is_spent {
                self.notes[idx].is_spent = true;
                // Promotion implies the spend confirmed; drop any
                // matching pending reservation. Idempotent — the
                // common path already cleared pending in the
                // spend-flow finalizer.
                self.pending_nullifiers.remove(nullifier);
                return true;
            }
        }
        false
    }

    /// Reserve `nullifier` against an in-flight spend. Returns
    /// `true` if newly added.
    pub(super) fn mark_pending(&mut self, nullifier: &[u8; 32]) -> bool {
        self.pending_nullifiers.insert(*nullifier)
    }

    /// Release a reservation previously taken via `mark_pending`.
    /// Returns `true` if a matching reservation was actually
    /// removed.
    pub(super) fn clear_pending(&mut self, nullifier: &[u8; 32]) -> bool {
        self.pending_nullifiers.remove(nullifier)
    }

    /// Record an outgoing (sent) note. Idempotent by `cmx`: returns
    /// `true` if newly recorded, `false` if a note with that `cmx`
    /// was already present.
    pub(super) fn record_outgoing_note(&mut self, note: &ShieldedOutgoingNote) -> bool {
        if !self.outgoing_cmx_index.insert(note.cmx) {
            return false;
        }
        self.outgoing_notes.push(note.clone());
        true
    }

    pub(super) fn outgoing_notes(&self) -> Vec<ShieldedOutgoingNote> {
        self.outgoing_notes.clone()
    }

    /// Upsert a derived activity entry by `entry.id`.
    pub(super) fn save_activity(&mut self, entry: &super::activity::ShieldedActivityEntry) {
        self.activity.insert(entry.id, entry.clone());
    }

    /// Display-sorted page of activity entries.
    pub(super) fn activity_page(
        &self,
        offset: usize,
        limit: usize,
    ) -> Vec<super::activity::ShieldedActivityEntry> {
        if limit == 0 {
            return Vec::new();
        }
        let mut all: Vec<super::activity::ShieldedActivityEntry> =
            self.activity.values().cloned().collect();
        super::activity::sort_activity_for_display(&mut all);
        all.into_iter().skip(offset).take(limit).collect()
    }

    pub(super) fn activity_by_id(
        &self,
        entry_id: &[u8; 32],
    ) -> Option<super::activity::ShieldedActivityEntry> {
        self.activity.get(entry_id).cloned()
    }

    pub(super) fn activity_ids(&self) -> BTreeSet<[u8; 32]> {
        self.activity.keys().copied().collect()
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

    fn mark_pending(&mut self, id: SubwalletId, nullifier: &[u8; 32]) -> Result<bool, Self::Error> {
        Ok(self
            .subwallets
            .entry(id)
            .or_default()
            .mark_pending(nullifier))
    }

    fn clear_pending(
        &mut self,
        id: SubwalletId,
        nullifier: &[u8; 32],
    ) -> Result<bool, Self::Error> {
        Ok(self
            .subwallets
            .get_mut(&id)
            .map(|sw| sw.clear_pending(nullifier))
            .unwrap_or(false))
    }

    fn record_outgoing_note(
        &mut self,
        id: SubwalletId,
        note: &ShieldedOutgoingNote,
    ) -> Result<bool, Self::Error> {
        Ok(self
            .subwallets
            .entry(id)
            .or_default()
            .record_outgoing_note(note))
    }

    fn get_outgoing_notes(
        &self,
        id: SubwalletId,
    ) -> Result<Vec<ShieldedOutgoingNote>, Self::Error> {
        Ok(self
            .subwallets
            .get(&id)
            .map(SubwalletState::outgoing_notes)
            .unwrap_or_default())
    }

    fn save_activity(
        &mut self,
        id: SubwalletId,
        entry: &super::activity::ShieldedActivityEntry,
    ) -> Result<(), Self::Error> {
        self.subwallets.entry(id).or_default().save_activity(entry);
        Ok(())
    }

    fn get_activity(
        &self,
        id: SubwalletId,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<super::activity::ShieldedActivityEntry>, Self::Error> {
        Ok(self
            .subwallets
            .get(&id)
            .map(|sw| sw.activity_page(offset, limit))
            .unwrap_or_default())
    }

    fn get_activity_by_entry_id(
        &self,
        id: SubwalletId,
        entry_id: &[u8; 32],
    ) -> Result<Option<super::activity::ShieldedActivityEntry>, Self::Error> {
        Ok(self
            .subwallets
            .get(&id)
            .and_then(|sw| sw.activity_by_id(entry_id)))
    }

    fn get_activity_ids(&self, id: SubwalletId) -> Result<BTreeSet<[u8; 32]>, Self::Error> {
        Ok(self
            .subwallets
            .get(&id)
            .map(SubwalletState::activity_ids)
            .unwrap_or_default())
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

    fn witness_at_depth(
        &self,
        _position: u64,
        _depth: usize,
    ) -> Result<Option<grovedb_commitment_tree::MerklePath>, Self::Error> {
        Err(InMemoryStoreError(
            "Merkle witness not supported in in-memory store".into(),
        ))
    }

    fn tree_size(&self) -> Result<u64, Self::Error> {
        Ok(self.commitments.len() as u64)
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

    fn purge_wallet(&mut self, wallet_id: WalletId) -> Result<(), Self::Error> {
        self.subwallets.retain(|id, _| id.wallet_id != wallet_id);
        Ok(())
    }

    fn purge_all_subwallets(&mut self) -> Result<(), Self::Error> {
        self.subwallets.clear();
        Ok(())
    }

    fn reset_commitment_tree(&mut self) -> Result<(), Self::Error> {
        // Flat-list backing: clearing the commitment / mark /
        // checkpoint vectors drops `tree_size()` to 0 and makes the
        // next append start at position 0, matching the file-backed
        // store's reset contract.
        self.commitments.clear();
        self.marked_positions.clear();
        self.checkpoints.clear();
        self.anchor = [0u8; 32];
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
    }

    #[test]
    fn test_commitment_tree_operations() {
        let mut store = InMemoryShieldedStore::new();
        store.append_commitment(&[1u8; 32], true).unwrap();
        store.append_commitment(&[2u8; 32], false).unwrap();
        store.checkpoint_tree(1).unwrap();
        assert_eq!(store.tree_anchor().unwrap(), [0u8; 32]);
    }

    #[test]
    fn test_save_activity_upserts_and_paginates() {
        use super::super::activity::{
            ShieldedActivityEntry, ShieldedActivityKind, ShieldedActivityStatus, ShieldedDirection,
        };

        fn entry(
            id: u8,
            height: Option<u64>,
            status: ShieldedActivityStatus,
        ) -> ShieldedActivityEntry {
            ShieldedActivityEntry {
                id: [id; 32],
                kind: ShieldedActivityKind::Sent,
                direction: ShieldedDirection::Out,
                amount: 1,
                fee: None,
                counterparty: None,
                memo: None,
                block_height: height,
                status,
                created_at_ms: 0,
                note_cmxs: vec![[id; 32]],
                spent_nullifiers: vec![],
            }
        }

        let mut store = InMemoryShieldedStore::new();
        let id = test_id(0);

        // Two confirmed at different heights + one pending.
        store
            .save_activity(id, &entry(1, Some(10), ShieldedActivityStatus::Confirmed))
            .unwrap();
        store
            .save_activity(id, &entry(2, Some(20), ShieldedActivityStatus::Confirmed))
            .unwrap();
        store
            .save_activity(id, &entry(3, None, ShieldedActivityStatus::Pending))
            .unwrap();

        // Display order: pending first, then height desc.
        let page = store.get_activity(id, 0, 10).unwrap();
        assert_eq!(page.len(), 3);
        assert_eq!(page[0].id, [3u8; 32], "pending floats to top");
        assert_eq!(page[1].id, [2u8; 32], "height 20 before 10");
        assert_eq!(page[2].id, [1u8; 32]);

        // Upsert by id: re-saving entry 1 as Pending replaces it in place
        // (still 3 entries, but now 1 is pending and floats up).
        store
            .save_activity(id, &entry(1, None, ShieldedActivityStatus::Pending))
            .unwrap();
        let page = store.get_activity(id, 0, 10).unwrap();
        assert_eq!(page.len(), 3, "upsert by id, not append");
        assert_eq!(
            store.get_activity_ids(id).unwrap().len(),
            3,
            "still exactly three distinct ids"
        );

        // Pagination: offset/limit slice the display-sorted list.
        let first_two = store.get_activity(id, 0, 2).unwrap();
        assert_eq!(first_two.len(), 2);
        let last_one = store.get_activity(id, 2, 10).unwrap();
        assert_eq!(last_one.len(), 1);
        // limit 0 => empty page.
        assert!(store.get_activity(id, 0, 0).unwrap().is_empty());

        // Lookup by entry id.
        assert!(store
            .get_activity_by_entry_id(id, &[2u8; 32])
            .unwrap()
            .is_some());
        assert!(store
            .get_activity_by_entry_id(id, &[9u8; 32])
            .unwrap()
            .is_none());
    }

    #[test]
    fn test_reset_commitment_tree_empties_and_reappends_from_zero() {
        let mut store = InMemoryShieldedStore::new();
        store.append_commitment(&[1u8; 32], true).unwrap();
        store.append_commitment(&[2u8; 32], true).unwrap();
        store.checkpoint_tree(2).unwrap();
        assert_eq!(store.tree_size().unwrap(), 2);

        store.reset_commitment_tree().unwrap();
        assert_eq!(
            store.tree_size().unwrap(),
            0,
            "tree_size must be 0 after reset"
        );

        // Re-append starts from position 0 again.
        store.append_commitment(&[3u8; 32], true).unwrap();
        assert_eq!(store.tree_size().unwrap(), 1);
    }
}
