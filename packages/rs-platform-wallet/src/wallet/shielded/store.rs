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
    ///
    /// This is an *observed at-or-before* bound (the tip the fetch was
    /// proven at), NOT the note's inclusion height, which the proof
    /// doesn't carry. It is a grouping key only — never surface it as
    /// when the note was mined (scan-derived activity entries carry
    /// `block_height: None` for exactly this reason).
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
    /// Proven platform height of the chunk fetch that recovered the
    /// note — the same per-batch *observed at-or-before* bound (and
    /// grouping key) as [`ShieldedNote::block_height`]; NOT the height
    /// the send was mined at.
    pub block_height: u64,
}

/// A pending reservation that carries a recorded anchor:
/// `(nullifier, anchor, activity_id)`. Yielded by
/// [`ShieldedStore::stale_pending_spends`] for the sync reconcile,
/// which releases the reservation once `anchor` is no longer in
/// Platform's recorded set (the spend can then never execute).
pub type StalePendingSpend = ([u8; 32], [u8; 32], Option<[u8; 32]>);

/// A re-drivable broadcast-accepted-but-unconfirmed spend: the signed
/// transition bytes plus everything the sync-time re-drive needs to
/// resolve the ambiguity actively — re-broadcast the transition
/// ([`nullifiers`](Self::nullifiers) detect a landing, `anchor` feeds
/// the prune backstop, `activity_id` links the UI row, `attempts`
/// bounds the retries.
///
/// Armed only on the ambiguous outcome (`ShieldedSpendUnconfirmed`):
/// the broadcast was accepted but the result wait failed, so the spend
/// may or may not have executed. Re-broadcasting the byte-identical
/// transition is fund-safe — identical nullifiers cannot double-spend —
/// and converts silence into either a confirmation (next scan sees the
/// nullifiers spent) or a definitive consensus verdict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingRedrive {
    /// Activity-entry id of the spend (sha256 of visible output cmxs);
    /// the spend-level key — every nullifier of one spend shares it.
    pub activity_id: [u8; 32],
    /// Platform-recorded anchor the spend was built against.
    pub anchor: [u8; 32],
    /// Nullifiers of every note the spend consumes.
    pub nullifiers: Vec<[u8; 32]>,
    /// The signed state transition, platform-serialized byte-exact as
    /// originally broadcast.
    pub st_bytes: Vec<u8>,
    /// Re-broadcast attempts made so far.
    pub attempts: u32,
}

/// The result of [`SubwalletState::mark_spent`].
///
/// `newly_spent` preserves the historical `bool` return (the
/// unspent→spent transition, which the durable store keys its
/// note-row write on). `dropped_redrives` carries the activity ids of
/// any redrive records resolved by this nullifier so the durable store
/// can mirror the SQLite deletion — even on the already-spent path,
/// where the in-memory drop happens but `newly_spent` is false.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct MarkSpentOutcome {
    pub newly_spent: bool,
    pub dropped_redrives: Vec<[u8; 32]>,
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

    /// Attach the recorded `anchor` the spend was built against, and
    /// the linked `activity_id`, to `id`'s pending reservation for
    /// `nullifier` (taken earlier by [`Self::mark_pending`]). No-op
    /// if the nullifier is no longer pending. The sync reconcile uses
    /// the stored anchor to detect a stranded spend whose anchor
    /// Platform has pruned (see [`Self::stale_pending_spends`]).
    fn set_pending_spend(
        &mut self,
        id: SubwalletId,
        nullifier: &[u8; 32],
        anchor: [u8; 32],
        activity_id: [u8; 32],
    ) -> Result<(), Self::Error>;

    /// Pending reservations for `id` that carry a recorded anchor —
    /// `(nullifier, anchor, activity_id)` per built-but-unconfirmed
    /// spend. Empty when `id` has no anchored reservations (the common
    /// case, which lets the sync reconcile skip its network round-trip).
    /// The reconcile checks each anchor against Platform's recorded set
    /// and releases the reservation via [`Self::clear_pending`] when the
    /// anchor is pruned — the spend can then never execute.
    fn stale_pending_spends(&self, id: SubwalletId) -> Result<Vec<StalePendingSpend>, Self::Error>;

    // ── Re-drivable unconfirmed spends (per-subwallet) ─────────────────

    /// Persist a re-drivable record for a broadcast-accepted spend whose
    /// result wait failed ambiguously. Keyed by `redrive.activity_id`;
    /// re-arming the same id overwrites. Unlike the bare `mark_pending`
    /// reservations, redrive records survive a restart where the backend
    /// persists them (the file store does): on reopen both the record
    /// and its note reservations are rehydrated, so the re-drive — and
    /// the linked activity row's eventual Confirmed/Failed flip —
    /// continue across relaunches.
    fn arm_redrive(&mut self, id: SubwalletId, redrive: PendingRedrive) -> Result<(), Self::Error>;

    /// Every armed redrive record for `id`.
    fn pending_redrives(&self, id: SubwalletId) -> Result<Vec<PendingRedrive>, Self::Error>;

    /// Increment the attempt counter on `id`'s redrive keyed by
    /// `activity_id`, returning the new count (`0` when no such record
    /// exists).
    fn bump_redrive_attempts(
        &mut self,
        id: SubwalletId,
        activity_id: &[u8; 32],
    ) -> Result<u32, Self::Error>;

    /// Drop the redrive record keyed by `activity_id` — the spend
    /// resolved (landed, definitively rejected, or released by the
    /// anchor-prune backstop). Implementations also drop the record
    /// implicitly when [`Self::mark_spent`] or [`Self::clear_pending`]
    /// resolves one of its nullifiers, since a transition lands or dies
    /// atomically for all of its nullifiers.
    fn clear_redrive(&mut self, id: SubwalletId, activity_id: &[u8; 32])
        -> Result<(), Self::Error>;

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

    /// Drop the per-subwallet state (and any durable redrive rows)
    /// for exactly ONE subwallet, leaving every other subwallet of
    /// the same wallet — and the shared commitment tree — intact.
    ///
    /// The account-scoped sibling of [`Self::purge_wallet`]. Used by
    /// the coordinator when a re-bind changes a wallet's account set:
    /// only the accounts that were dropped (or whose viewing key
    /// changed — their stored notes belong to the old key) are
    /// purged, so the accounts that remain bound keep their fresh
    /// in-memory notes and watermarks across the re-bind.
    fn purge_subwallet(&mut self, id: SubwalletId) -> Result<(), Self::Error>;

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

    // ── Lifecycle admission (store-level, cross-instance) ──────────────
    //
    // See the [`LifecycleAdmission`] module docs for the protocol and its
    // correctness argument. These five methods exist on the STORE, not on the
    // coordinator, because the store is the only object two coordinators —
    // or two processes — sharing the same backing state actually have in
    // common (`dashpay/platform#4313`).

    /// Admit a one-time-key claim for `wallet_id`, or refuse it because a
    /// destructive lifecycle operation holds admission over that scope.
    ///
    /// On `Ok(true)` a claim lease keyed by `token` is durable and live until
    /// `now_ms + lease_ms`; the caller owns it until it calls
    /// [`Self::end_claim_admission`]. On `Ok(false)` nothing was written and
    /// the caller must not touch the claim record.
    ///
    /// Implementations MUST make the barrier check and the lease insert one
    /// atomic step against every other admission operation on the same
    /// underlying state.
    fn begin_claim_admission(
        &mut self,
        wallet_id: WalletId,
        token: AdmissionToken,
        now_ms: u64,
        lease_ms: u64,
    ) -> Result<bool, Self::Error>;

    /// Arm `redrive` **only if** the claim lease `token` is still live,
    /// re-stamping that lease to `now_ms + lease_ms` in the same atomic step.
    ///
    /// Returns `Ok(false)` — with nothing written — when the lease has expired
    /// or was already released. Callers fail the claim closed on `false`: the
    /// record is the only handle that recovers a padded single-note claim, so
    /// broadcasting without it is unrecoverable.
    ///
    /// Arming under the lease rather than next to it is what closes the gap
    /// between "the claim checked that it was admitted" and "the claim wrote
    /// the record": the two are one transaction, so a destructive operation
    /// cannot slot in between them.
    fn arm_redrive_under_claim(
        &mut self,
        id: SubwalletId,
        redrive: PendingRedrive,
        token: AdmissionToken,
        now_ms: u64,
        lease_ms: u64,
    ) -> Result<bool, Self::Error>;

    /// Re-stamp a live claim lease to `now_ms + lease_ms`, WITHOUT touching the
    /// pending record.
    ///
    /// [`Self::arm_redrive_under_claim`] stamps the lease once, so the window
    /// protecting a claim ran for a fixed [`CLAIM_LEASE_MS`] from the arm — not
    /// for as long as the claim actually took. A broadcast plus confirmation
    /// that outran it (a slow or retrying DAPI node is enough) let the row be
    /// reaped, at which point a purge counted zero live claims and destroyed
    /// state the in-flight claim still needed (#4313 review finding 161a517fce36).
    ///
    /// Returns `false` when the lease is gone — already lapsed and reaped, or
    /// displaced by a destructive barrier. The caller cannot un-send a
    /// transition, so a `false` is a loud diagnostic, not an abort.
    fn renew_claim_admission(
        &mut self,
        token: AdmissionToken,
        now_ms: u64,
        lease_ms: u64,
    ) -> Result<bool, Self::Error>;

    /// Release the claim lease `token`. Idempotent; unknown tokens are a
    /// no-op (a lease that already expired was reaped).
    fn end_claim_admission(&mut self, token: AdmissionToken) -> Result<(), Self::Error>;

    /// Take (or refresh) destructive admission over `scope` and report how
    /// many claim leases are still live inside it.
    ///
    /// `scope` is `None` for a whole-store operation (`purge_all_subwallets`)
    /// and `Some(wallet_id)` for a single wallet (`purge_wallet`). The barrier
    /// is installed **and** the live-lease count taken in one atomic step, so
    /// a claim is either counted here or refused by
    /// [`Self::begin_claim_admission`] — never both, never neither.
    ///
    /// A non-zero count means the caller must wait and call again; it must not
    /// purge. The barrier carries its own expiry so a crashed holder cannot
    /// block claims forever, and refreshing it is exactly re-calling this.
    fn begin_destructive_admission(
        &mut self,
        scope: Option<WalletId>,
        token: AdmissionToken,
        now_ms: u64,
        barrier_ms: u64,
    ) -> Result<usize, Self::Error>;

    /// Drop the destructive barrier `token`, whether the operation went ahead
    /// or gave up. Idempotent.
    fn end_destructive_admission(&mut self, token: AdmissionToken) -> Result<(), Self::Error>;
}

/// How long a one-time-claim lease stays live without being re-stamped.
///
/// It has to comfortably exceed the longest single phase a claim spends between
/// two store touches — the transient full-history scan of a cold wallet, or a
/// Halo 2 proof on a slow phone — because a lease that lapses mid-claim makes
/// the record arming fail closed (safe, but a wasted attempt). It also bounds
/// how long a claim that was CANCELLED without releasing (a dropped JNI call)
/// can block wallet removal, so it must not be unbounded either. Five minutes
/// sits well above both phases and well below a user's patience for "try
/// removing the wallet again".
///
/// The lease is re-stamped when the record is armed
/// ([`ShieldedStore::arm_redrive_under_claim`]), so the window that actually
/// protects the durable record runs from the arm — not from the start of the
/// claim — and covers the broadcast and confirmation wait that follow it.
pub(crate) const CLAIM_LEASE_MS: u64 = 5 * 60 * 1_000;

/// How often an in-flight claim re-stamps its lease
/// ([`ShieldedStore::renew_claim_admission`]).
///
/// A third of [`CLAIM_LEASE_MS`]: two consecutive renewals may be missed — a
/// stalled executor, a long blocking store write — before the lease can lapse,
/// while the tick stays far cheaper than the network phases it runs alongside.
/// Renewing is a single indexed UPDATE, so the cost is noise next to a
/// broadcast.
pub(crate) const CLAIM_LEASE_RENEW_INTERVAL: std::time::Duration =
    std::time::Duration::from_millis(CLAIM_LEASE_MS / 3);

/// How long a destructive barrier survives without being refreshed.
///
/// Only has to outlive one drain wait (it is refreshed on every poll), plus
/// margin. Kept short so a purge whose process died cannot keep refusing
/// claims for long.
pub(crate) const DESTRUCTIVE_BARRIER_MS: u64 = 60 * 1_000;

/// How long a destructive lifecycle operation waits for in-flight claims to
/// drain before giving up.
///
/// Giving up means REFUSING to purge, not purging anyway: deleting a record
/// under a live claim is the unrecoverable outcome this whole mechanism
/// exists to prevent, while a refused purge is a retry.
pub(crate) const DESTRUCTIVE_DRAIN_TIMEOUT: std::time::Duration =
    std::time::Duration::from_secs(30);

/// Poll interval while waiting for claim leases to drain. Each poll also
/// refreshes the barrier, so no new claim slips in during the wait.
pub(crate) const DESTRUCTIVE_DRAIN_POLL: std::time::Duration =
    std::time::Duration::from_millis(250);

/// Opaque owner token for one lifecycle admission — a claim lease or a
/// destructive barrier.
///
/// 16 random bytes from the OS CSPRNG rather than a counter: admissions are
/// compared across independent store instances and, for the file-backed store,
/// across PROCESSES sharing one SQLite file, so a per-process counter could
/// collide and let one holder release another's admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AdmissionToken(pub [u8; 16]);

impl AdmissionToken {
    /// A fresh token from the OS CSPRNG.
    pub fn new() -> Self {
        use rand::{rngs::OsRng, RngCore};

        let mut bytes = [0u8; 16];
        OsRng.fill_bytes(&mut bytes);
        Self(bytes)
    }
}

impl Default for AdmissionToken {
    fn default() -> Self {
        Self::new()
    }
}

/// Wall-clock milliseconds since the Unix epoch — the one clock every
/// admission lease and barrier is stamped and judged against.
///
/// Wall clock rather than a monotonic instant because the leases are compared
/// across processes, which share no monotonic origin. Both holders read the
/// same system clock on the same machine, which is what the comparison needs.
pub fn admission_now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// One row of the lifecycle-admission table.
///
/// # The protocol, and why it is at the store
///
/// A one-time-key claim and a destructive lifecycle operation (`clear`,
/// `unregister_wallet`, and `remove_wallet` through it) both act on the same
/// durable pending-claim record. `clear` and `unregister_wallet` serialize
/// against each other on the coordinator's `lifecycle` mutex, but a claim never
/// takes it, and the per-FVK single-flight guards are owned by ONE
/// `NetworkShieldedCoordinator` — so a purge could delete an armed record while
/// its transition was still broadcasting. A coordinator-local `tokio` mutex
/// cannot fix that: `FileBackedShieldedStore::open_path` opens independent
/// SQLite connections to the same file, so two coordinators (or two processes)
/// share the state but not the mutex (`dashpay/platform#4313`).
///
/// Admission therefore lives at the only thing they do share — the store:
///
/// 1. A claim takes a **lease** ([`ShieldedStore::begin_claim_admission`]),
///    refused if a barrier already covers its wallet.
/// 2. A destructive operation installs a **barrier**
///    ([`ShieldedStore::begin_destructive_admission`]), which blocks new leases
///    and reports the leases already live in scope. It waits for that count to
///    reach zero and refuses to purge if it does not.
/// 3. The claim arms its record *under* its lease
///    ([`ShieldedStore::arm_redrive_under_claim`]), which re-checks and
///    re-stamps the lease in the same atomic step.
///
/// # Why there is no residual race
///
/// Step 1 and step 2 are each ONE atomic step against the shared state. They
/// therefore have a total order, and both orders are safe:
///
/// * lease commits first → the barrier's count sees it → the purge waits.
/// * barrier commits first → the lease's check sees it → the claim is refused.
///
/// For [`FileBackedShieldedStore`](super::file_store::FileBackedShieldedStore)
/// that atomicity is a `BEGIN IMMEDIATE` SQLite transaction: SQLite admits one
/// writer at a time across every connection **and every process** on the file,
/// so the total order holds exactly where a process-local mutex does not. For
/// [`InMemoryShieldedStore`] the shared object *is* the store, reached through
/// the same `RwLock`, so the write guard supplies the same total order.
///
/// No admission call holds a write transaction across scanning, proof
/// construction, broadcast, or a confirmation wait: each is a handful of
/// statements, and the long phases run between them holding only the lease row.
///
/// # Expiry
///
/// Both kinds carry `expires_at`, because a holder can die (process kill,
/// cancelled coroutine) with no chance to release. Expiry is a liveness
/// backstop only — it never lets a purge delete a record under a *live* claim,
/// it only bounds how long a dead one can block wallet removal, and how long a
/// dead purge can block claims.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LifecycleAdmission {
    /// Owner token.
    pub token: AdmissionToken,
    /// `true` for a destructive barrier, `false` for a claim lease.
    pub destructive: bool,
    /// Scope: `None` is store-wide, `Some(id)` is one wallet.
    pub wallet_id: Option<WalletId>,
    /// Unix millis after which this admission is dead and reapable.
    pub expires_at: u64,
}

impl LifecycleAdmission {
    /// Whether this admission's scope covers `wallet_id`. A store-wide entry
    /// covers every wallet; a wallet-scoped one covers only its own.
    pub fn covers(&self, wallet_id: WalletId) -> bool {
        self.wallet_id.is_none_or(|scoped| scoped == wallet_id)
    }

    /// Whether this admission's scope overlaps `scope` (the same containment
    /// relation as [`Self::covers`], in whichever direction applies).
    pub fn overlaps(&self, scope: Option<WalletId>) -> bool {
        match (self.wallet_id, scope) {
            (None, _) | (_, None) => true,
            (Some(mine), Some(theirs)) => mine == theirs,
        }
    }
}

// ── Per-subwallet bookkeeping ──────────────────────────────────────────

/// In-flight-spend bookkeeping for a single reserved nullifier.
///
/// A reservation starts life bare (both fields `None`) the moment
/// [`SubwalletState::mark_pending`] excludes the note from selection.
/// Once the spend is actually built, [`SubwalletState::set_pending_spend`]
/// records the Platform-**recorded** anchor it was built against and the
/// linked activity-log entry, so the sync reconcile can later detect a
/// stranded (broadcast-accepted-but-never-landed) spend: once that anchor
/// is pruned from Platform's recorded set the spend can never execute, and
/// with the nullifier still unspent the reservation is provably dead and
/// its note can be freed. All in-memory only — a restart drops every
/// reservation regardless.
#[derive(Debug, Clone, Default)]
pub(super) struct PendingSpend {
    /// The recorded anchor the spend was built against, once known.
    /// `None` while the note is reserved but the spend isn't built yet.
    pub anchor: Option<[u8; 32]>,
    /// The linked activity-log entry id, so a released reservation can
    /// flip its "Pending" row to "Failed" instead of stranding it.
    pub activity_id: Option<[u8; 32]>,
}

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
    /// transition, mapped to the [`PendingSpend`] bookkeeping the
    /// sync reconcile needs. Excluded from `unspent_notes()` so
    /// concurrent callers can't double-select. Held in memory; a
    /// pre-broadcast reservation dies with the process, but a
    /// reservation belonging to an armed [`PendingRedrive`] is
    /// rehydrated on file-store open (the redrive row carries its
    /// nullifiers), so an unconfirmed broadcast keeps its notes
    /// reserved across restarts.
    pub pending_nullifiers: BTreeMap<[u8; 32], PendingSpend>,
    /// Armed re-drivable unconfirmed spends, keyed by activity id.
    /// See [`PendingRedrive`]. Kept consistent with
    /// `pending_nullifiers`: resolving any of a redrive's nullifiers
    /// (mark_spent / clear_pending) drops the whole record — a
    /// transition lands or dies atomically for all its nullifiers.
    pub redrives: BTreeMap<[u8; 32], PendingRedrive>,
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
            .filter(|n| !n.is_spent && !self.pending_nullifiers.contains_key(&n.nullifier))
            .cloned()
            .collect()
    }

    pub(super) fn all_notes(&self) -> Vec<ShieldedNote> {
        self.notes.clone()
    }

    pub(super) fn mark_spent(&mut self, nullifier: &[u8; 32]) -> MarkSpentOutcome {
        let Some(&idx) = self.nullifier_index.get(nullifier) else {
            return MarkSpentOutcome::default();
        };
        let newly_spent = !self.notes[idx].is_spent;
        self.notes[idx].is_spent = true;
        // Resolve the reservation + redrive record whenever the
        // nullifier is KNOWN, not only on the first unspent→spent
        // transition. A note restored from disk already `is_spent`
        // (its owning transition landed in a prior session) paired with
        // a rehydrated redrive row would otherwise keep a ghost
        // reservation alive and re-broadcast a transition that already
        // executed. Removing a pending reservation on a spent note is
        // always safe — a spent note can't be re-spent. `dropped` is
        // returned so the durable store can mirror the deletion even
        // when `newly_spent` is false (the in-memory drop still
        // happened here).
        self.pending_nullifiers.remove(nullifier);
        let dropped = self.drop_redrives_containing(nullifier);
        MarkSpentOutcome {
            newly_spent,
            dropped_redrives: dropped,
        }
    }

    /// Drop every redrive record that carries `nullifier`, returning
    /// the dropped activity ids (the file store mirrors the deletions
    /// to SQLite). A transition lands or dies atomically for all of
    /// its nullifiers, so resolving one resolves the record.
    pub(super) fn drop_redrives_containing(&mut self, nullifier: &[u8; 32]) -> Vec<[u8; 32]> {
        let dropped: Vec<[u8; 32]> = self
            .redrives
            .iter()
            .filter(|(_, r)| r.nullifiers.contains(nullifier))
            .map(|(id, _)| *id)
            .collect();
        for id in &dropped {
            self.redrives.remove(id);
        }
        dropped
    }

    /// Reserve `nullifier` against an in-flight spend. Returns
    /// `true` if newly added, `false` if it was already reserved.
    /// Re-reserving is a true no-op: an already-armed entry keeps its
    /// anchor/activity link (selection excludes pending nullifiers, so
    /// this shouldn't happen — but a plain `insert` would silently
    /// wipe the release pass's only handle on a stranded spend if it
    /// ever did).
    pub(super) fn mark_pending(&mut self, nullifier: &[u8; 32]) -> bool {
        match self.pending_nullifiers.entry(*nullifier) {
            std::collections::btree_map::Entry::Vacant(e) => {
                e.insert(PendingSpend::default());
                true
            }
            std::collections::btree_map::Entry::Occupied(_) => false,
        }
    }

    /// Attach the built spend's recorded `anchor` and linked
    /// `activity_id` to an existing reservation for `nullifier`.
    /// No-op if the nullifier is no longer pending (e.g. the
    /// reservation was already cleared by a concurrent finalize).
    pub(super) fn set_pending_spend(
        &mut self,
        nullifier: &[u8; 32],
        anchor: [u8; 32],
        activity_id: [u8; 32],
    ) {
        if let Some(pending) = self.pending_nullifiers.get_mut(nullifier) {
            pending.anchor = Some(anchor);
            pending.activity_id = Some(activity_id);
        }
    }

    /// Reservations that carry a recorded anchor — i.e. built,
    /// broadcast-but-unconfirmed spends. Returns `(nullifier,
    /// anchor, activity_id)` per such entry; reservations still
    /// awaiting a built spend (`anchor: None`) are skipped. The
    /// sync reconcile checks each anchor against Platform's recorded
    /// set and releases the reservation when the anchor is pruned.
    pub(super) fn stale_pending_spends(&self) -> Vec<StalePendingSpend> {
        self.pending_nullifiers
            .iter()
            .filter_map(|(nullifier, spend)| {
                spend
                    .anchor
                    .map(|anchor| (*nullifier, anchor, spend.activity_id))
            })
            .collect()
    }

    /// Release a reservation previously taken via `mark_pending`.
    /// Returns `true` if a matching reservation was actually
    /// removed.
    pub(super) fn clear_pending(&mut self, nullifier: &[u8; 32]) -> bool {
        let removed = self.pending_nullifiers.remove(nullifier).is_some();
        if removed {
            // Releasing a reservation resolves its spend for good
            // (definitive rejection or prune backstop) — the redrive
            // record goes with it.
            self.drop_redrives_containing(nullifier);
        }
        removed
    }

    /// Arm (or overwrite by activity id) a re-drivable record.
    pub(super) fn arm_redrive(&mut self, redrive: PendingRedrive) {
        self.redrives.insert(redrive.activity_id, redrive);
    }

    pub(super) fn pending_redrives(&self) -> Vec<PendingRedrive> {
        self.redrives.values().cloned().collect()
    }

    /// Current attempt count for `activity_id`'s redrive, if armed.
    pub(super) fn redrive_attempts(&self, activity_id: &[u8; 32]) -> Option<u32> {
        self.redrives.get(activity_id).map(|r| r.attempts)
    }

    /// Bump the attempt counter; `0` when no such record exists.
    pub(super) fn bump_redrive_attempts(&mut self, activity_id: &[u8; 32]) -> u32 {
        self.redrives
            .get_mut(activity_id)
            .map(|r| {
                r.attempts += 1;
                r.attempts
            })
            .unwrap_or(0)
    }

    pub(super) fn clear_redrive(&mut self, activity_id: &[u8; 32]) {
        self.redrives.remove(activity_id);
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
    /// Live lifecycle admissions — see [`LifecycleAdmission`].
    ///
    /// For this store the shared object two coordinators would contend over is
    /// the store itself, reached through one `RwLock<S>`, so holding the table
    /// here gives the same total order between a claim lease and a destructive
    /// barrier that the file store gets from SQLite's single-writer rule.
    admissions: Vec<LifecycleAdmission>,
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
        // In-memory store: the redrive map lives in the same
        // `SubwalletState`, so `mark_spent` already dropped any resolved
        // redrives — nothing durable to mirror. Surface `newly_spent`.
        Ok(self
            .subwallets
            .get_mut(&id)
            .map(|sw| sw.mark_spent(nullifier).newly_spent)
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

    fn set_pending_spend(
        &mut self,
        id: SubwalletId,
        nullifier: &[u8; 32],
        anchor: [u8; 32],
        activity_id: [u8; 32],
    ) -> Result<(), Self::Error> {
        if let Some(sw) = self.subwallets.get_mut(&id) {
            sw.set_pending_spend(nullifier, anchor, activity_id);
        }
        Ok(())
    }

    fn stale_pending_spends(&self, id: SubwalletId) -> Result<Vec<StalePendingSpend>, Self::Error> {
        Ok(self
            .subwallets
            .get(&id)
            .map(SubwalletState::stale_pending_spends)
            .unwrap_or_default())
    }

    fn arm_redrive(&mut self, id: SubwalletId, redrive: PendingRedrive) -> Result<(), Self::Error> {
        self.subwallets.entry(id).or_default().arm_redrive(redrive);
        Ok(())
    }

    fn pending_redrives(&self, id: SubwalletId) -> Result<Vec<PendingRedrive>, Self::Error> {
        Ok(self
            .subwallets
            .get(&id)
            .map(SubwalletState::pending_redrives)
            .unwrap_or_default())
    }

    fn bump_redrive_attempts(
        &mut self,
        id: SubwalletId,
        activity_id: &[u8; 32],
    ) -> Result<u32, Self::Error> {
        Ok(self
            .subwallets
            .get_mut(&id)
            .map(|sw| sw.bump_redrive_attempts(activity_id))
            .unwrap_or(0))
    }

    fn clear_redrive(
        &mut self,
        id: SubwalletId,
        activity_id: &[u8; 32],
    ) -> Result<(), Self::Error> {
        if let Some(sw) = self.subwallets.get_mut(&id) {
            sw.clear_redrive(activity_id);
        }
        Ok(())
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

    fn purge_subwallet(&mut self, id: SubwalletId) -> Result<(), Self::Error> {
        self.subwallets.remove(&id);
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

    // ── Lifecycle admission ────────────────────────────────────────────
    //
    // Each of these is one uninterruptible `&mut self` step, and every caller
    // reaches this store through the same `RwLock<S>`, so the write guard
    // supplies exactly the total order the protocol needs — see
    // [`LifecycleAdmission`].

    fn begin_claim_admission(
        &mut self,
        wallet_id: WalletId,
        token: AdmissionToken,
        now_ms: u64,
        lease_ms: u64,
    ) -> Result<bool, Self::Error> {
        self.admissions.retain(|a| a.expires_at > now_ms);
        if self
            .admissions
            .iter()
            .any(|a| a.destructive && a.covers(wallet_id))
        {
            return Ok(false);
        }
        self.admissions.retain(|a| a.token != token);
        self.admissions.push(LifecycleAdmission {
            token,
            destructive: false,
            wallet_id: Some(wallet_id),
            expires_at: now_ms.saturating_add(lease_ms),
        });
        Ok(true)
    }

    fn arm_redrive_under_claim(
        &mut self,
        id: SubwalletId,
        redrive: PendingRedrive,
        token: AdmissionToken,
        now_ms: u64,
        lease_ms: u64,
    ) -> Result<bool, Self::Error> {
        let Some(lease) = self
            .admissions
            .iter_mut()
            .find(|a| a.token == token && !a.destructive && a.expires_at > now_ms)
        else {
            return Ok(false);
        };
        lease.expires_at = now_ms.saturating_add(lease_ms);
        self.subwallets.entry(id).or_default().arm_redrive(redrive);
        Ok(true)
    }

    fn renew_claim_admission(
        &mut self,
        token: AdmissionToken,
        now_ms: u64,
        lease_ms: u64,
    ) -> Result<bool, Self::Error> {
        // Same predicate arm_redrive_under_claim uses: a live, non-destructive
        // lease under this exact token. Deliberately does NOT resurrect an
        // expired one — a lapsed lease may already have been counted as absent
        // by a purge, and silently reviving it would hide that.
        let Some(lease) = self
            .admissions
            .iter_mut()
            .find(|a| a.token == token && !a.destructive && a.expires_at > now_ms)
        else {
            return Ok(false);
        };
        lease.expires_at = now_ms.saturating_add(lease_ms);
        Ok(true)
    }

    fn end_claim_admission(&mut self, token: AdmissionToken) -> Result<(), Self::Error> {
        self.admissions
            .retain(|a| a.destructive || a.token != token);
        Ok(())
    }

    fn begin_destructive_admission(
        &mut self,
        scope: Option<WalletId>,
        token: AdmissionToken,
        now_ms: u64,
        barrier_ms: u64,
    ) -> Result<usize, Self::Error> {
        self.admissions.retain(|a| a.expires_at > now_ms);
        self.admissions.retain(|a| a.token != token);
        self.admissions.push(LifecycleAdmission {
            token,
            destructive: true,
            wallet_id: scope,
            expires_at: now_ms.saturating_add(barrier_ms),
        });
        Ok(self
            .admissions
            .iter()
            .filter(|a| !a.destructive && a.overlaps(scope))
            .count())
    }

    fn end_destructive_admission(&mut self, token: AdmissionToken) -> Result<(), Self::Error> {
        self.admissions
            .retain(|a| !a.destructive || a.token != token);
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

    /// Resolving any of a redrive's nullifiers — landing (`mark_spent`)
    /// or release (`clear_pending`) — drops the whole record: a
    /// transition lands or dies atomically for all its nullifiers.
    #[test]
    fn resolving_a_nullifier_drops_the_redrive_record() {
        let mut store = InMemoryShieldedStore::new();
        let id = test_id(0);
        let n1 = [3u8; 32];
        let n2 = [4u8; 32];
        let note = ShieldedNote {
            position: 0,
            cmx: [1u8; 32],
            nullifier: n1,
            block_height: 50,
            is_spent: false,
            value: 500,
            note_data: vec![0u8; 115],
        };
        store.save_note(id, &note).unwrap();
        let redrive = PendingRedrive {
            activity_id: [9u8; 32],
            anchor: [8u8; 32],
            nullifiers: vec![n1, n2],
            st_bytes: vec![1, 2, 3],
            attempts: 0,
        };

        // Landing path: mark_spent on one nullifier drops the record.
        store.mark_pending(id, &n1).unwrap();
        store.arm_redrive(id, redrive.clone()).unwrap();
        assert_eq!(store.pending_redrives(id).unwrap().len(), 1);
        assert!(store.mark_spent(id, &n1).unwrap());
        assert!(
            store.pending_redrives(id).unwrap().is_empty(),
            "landed spend drops its redrive record"
        );

        // Release path: clear_pending on a nullifier drops the record.
        store.mark_pending(id, &n2).unwrap();
        store.arm_redrive(id, redrive).unwrap();
        assert!(store.clear_pending(id, &n2).unwrap());
        assert!(
            store.pending_redrives(id).unwrap().is_empty(),
            "released reservation drops its redrive record"
        );
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
                min_note_position: None,
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

    /// Build a minimal unspent note carrying `nullifier`.
    fn note_with_nullifier(nullifier: [u8; 32]) -> ShieldedNote {
        ShieldedNote {
            position: 0,
            cmx: [1u8; 32],
            nullifier,
            block_height: 1,
            is_spent: false,
            value: 1_000,
            note_data: vec![0u8; 115],
        }
    }

    /// A reservation carrying a recorded anchor surfaces via
    /// `stale_pending_spends`, and the sync reconcile's release decision
    /// (release iff the anchor is absent from the recorded set) must RETAIN
    /// it while the anchor is still recorded — a slow-but-landing spend is
    /// never freed, so its note stays out of the unspent set.
    #[test]
    fn pending_spend_with_recorded_anchor_is_retained() {
        use std::collections::HashSet;

        let mut store = InMemoryShieldedStore::new();
        let id = test_id(0);
        let nullifier = [0x11; 32];
        let anchor = [0x22; 32];
        let activity_id = [0x33; 32];

        store
            .save_note(id, &note_with_nullifier(nullifier))
            .unwrap();
        assert!(
            store.mark_pending(id, &nullifier).unwrap(),
            "newly reserved"
        );
        store
            .set_pending_spend(id, &nullifier, anchor, activity_id)
            .unwrap();

        assert_eq!(
            store.stale_pending_spends(id).unwrap(),
            vec![(nullifier, anchor, Some(activity_id))],
            "an armed reservation surfaces as an anchored (stale-checkable) spend"
        );

        // Anchor IS still recorded → retain.
        let recorded: HashSet<[u8; 32]> = [anchor].into_iter().collect();
        for (n, a, _) in store.stale_pending_spends(id).unwrap() {
            if !recorded.contains(&a) {
                store.clear_pending(id, &n).unwrap();
            }
        }
        assert_eq!(
            store.stale_pending_spends(id).unwrap().len(),
            1,
            "a still-recorded anchor must not be released"
        );
        assert!(
            store.get_unspent_notes(id).unwrap().is_empty(),
            "the retained note stays excluded from selection candidates"
        );
    }

    /// The fund-safe release: once the anchor is pruned (absent from the
    /// recorded set) the spend can never execute, so the reservation is
    /// released and the freed note becomes spendable again.
    #[test]
    fn pending_spend_with_pruned_anchor_is_released() {
        use std::collections::HashSet;

        let mut store = InMemoryShieldedStore::new();
        let id = test_id(0);
        let nullifier = [0x11; 32];
        let anchor = [0x22; 32];
        let activity_id = [0x33; 32];

        store
            .save_note(id, &note_with_nullifier(nullifier))
            .unwrap();
        store.mark_pending(id, &nullifier).unwrap();
        store
            .set_pending_spend(id, &nullifier, anchor, activity_id)
            .unwrap();
        assert!(store.get_unspent_notes(id).unwrap().is_empty());

        // Anchor is ABSENT from the recorded set (pruned) → release.
        let recorded: HashSet<[u8; 32]> = [[0xEE; 32]].into_iter().collect();
        for (n, a, _) in store.stale_pending_spends(id).unwrap() {
            if !recorded.contains(&a) {
                assert!(store.clear_pending(id, &n).unwrap());
            }
        }
        assert!(
            store.stale_pending_spends(id).unwrap().is_empty(),
            "a pruned-anchor reservation must be released"
        );
        let unspent = store.get_unspent_notes(id).unwrap();
        assert_eq!(unspent.len(), 1, "the freed note is spendable again");
        assert_eq!(unspent[0].nullifier, nullifier);
    }

    /// A reserved note is excluded from `unspent_notes` (so a concurrent
    /// spend can't re-select it) and re-included once the reservation is
    /// cleared.
    #[test]
    fn unspent_notes_excludes_pending_and_reincludes_after_clear() {
        let mut store = InMemoryShieldedStore::new();
        let id = test_id(0);
        let nullifier = [0x55; 32];
        store
            .save_note(id, &note_with_nullifier(nullifier))
            .unwrap();
        assert_eq!(store.get_unspent_notes(id).unwrap().len(), 1);

        store.mark_pending(id, &nullifier).unwrap();
        assert!(
            store.get_unspent_notes(id).unwrap().is_empty(),
            "a reserved note is excluded from selection candidates"
        );

        assert!(store.clear_pending(id, &nullifier).unwrap());
        assert_eq!(
            store.get_unspent_notes(id).unwrap().len(),
            1,
            "releasing the reservation re-includes the note"
        );
    }

    /// `stale_pending_spends` ignores a reservation that was `mark_pending`ed
    /// but never armed with an anchor (a just-reserved, not-yet-built spend):
    /// the reconcile must never release such a transient entry.
    #[test]
    fn unarmed_reservation_is_not_stale() {
        let mut store = InMemoryShieldedStore::new();
        let id = test_id(0);
        let nullifier = [0x66; 32];
        store
            .save_note(id, &note_with_nullifier(nullifier))
            .unwrap();
        store.mark_pending(id, &nullifier).unwrap();

        assert!(
            store.stale_pending_spends(id).unwrap().is_empty(),
            "a reservation with no recorded anchor must not surface as stale"
        );
    }

    // ---- claim-lease renewal (#4313 review finding 161a517fce36) ----

    fn claim_token(b: u8) -> AdmissionToken {
        AdmissionToken([b; 16])
    }

    /// THE BUG: without renewal a claim that outruns CLAIM_LEASE_MS is reaped,
    /// so a purge sees zero live claims and proceeds to destroy state the
    /// in-flight claim still needs. This is the negative control — it pins the
    /// behaviour the renewal exists to prevent.
    #[test]
    fn an_unrenewed_lease_lapses_and_stops_holding_off_a_purge() {
        let mut store = InMemoryShieldedStore::new();
        let wallet = [0xAA; 32];
        let token = claim_token(0x01);
        let t0 = 1_000_000u64;

        assert!(store
            .begin_claim_admission(wallet, token, t0, CLAIM_LEASE_MS)
            .unwrap());
        // Still inside the lease: a purge must WAIT.
        let live = store
            .begin_destructive_admission(None, claim_token(0x99), t0 + 1, DESTRUCTIVE_BARRIER_MS)
            .unwrap();
        assert_eq!(live, 1, "a live claim must hold off a purge");
        store.end_destructive_admission(claim_token(0x99)).unwrap();

        // One millisecond past the lease, with no renewal, the claim is invisible.
        let live_after = store
            .begin_destructive_admission(
                None,
                claim_token(0x98),
                t0 + CLAIM_LEASE_MS + 1,
                DESTRUCTIVE_BARRIER_MS,
            )
            .unwrap();
        assert_eq!(
            live_after, 0,
            "an unrenewed lease lapses — this is exactly what the heartbeat prevents"
        );
    }

    /// THE FIX: renewing keeps the claim counted well past the original lease.
    #[test]
    fn a_renewed_lease_keeps_holding_off_a_purge_past_the_original_window() {
        let mut store = InMemoryShieldedStore::new();
        let wallet = [0xAA; 32];
        let token = claim_token(0x02);
        let t0 = 1_000_000u64;

        assert!(store
            .begin_claim_admission(wallet, token, t0, CLAIM_LEASE_MS)
            .unwrap());

        // Three ticks at the heartbeat interval, as the claim path does.
        let tick = CLAIM_LEASE_MS / 3;
        let mut now = t0;
        for _ in 0..3 {
            now += tick;
            assert!(
                store
                    .renew_claim_admission(token, now, CLAIM_LEASE_MS)
                    .unwrap(),
                "renewal must succeed while the lease is live"
            );
        }

        // Past the ORIGINAL expiry, the claim is still counted.
        assert!(now > t0 + CLAIM_LEASE_MS - tick);
        let live = store
            .begin_destructive_admission(None, claim_token(0x97), now + 1, DESTRUCTIVE_BARRIER_MS)
            .unwrap();
        assert_eq!(live, 1, "a renewed claim must still hold off a purge");
    }

    /// Renewal must not RESURRECT a lease that already lapsed — a purge may
    /// already have counted it absent and acted on that.
    #[test]
    fn renewal_refuses_to_resurrect_a_lapsed_lease() {
        let mut store = InMemoryShieldedStore::new();
        let token = claim_token(0x03);
        let t0 = 1_000_000u64;
        assert!(store
            .begin_claim_admission([0xAA; 32], token, t0, CLAIM_LEASE_MS)
            .unwrap());
        assert!(
            !store
                .renew_claim_admission(token, t0 + CLAIM_LEASE_MS + 1, CLAIM_LEASE_MS)
                .unwrap(),
            "a lapsed lease must not come back"
        );
    }

    /// An unknown token renews nothing, rather than minting a lease.
    #[test]
    fn renewing_an_unknown_token_is_false_not_a_new_lease() {
        let mut store = InMemoryShieldedStore::new();
        assert!(!store
            .renew_claim_admission(claim_token(0x04), 1_000_000, CLAIM_LEASE_MS)
            .unwrap());
    }
}
