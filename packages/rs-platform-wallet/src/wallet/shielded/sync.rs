//! Shielded note synchronization with scan-based spend detection.
//!
//! The heavy lifting lives in three free functions that take a
//! flat `&[(SubwalletId, AccountViewingKeys)]` slice and drive a
//! single network-wide SDK fetch per pass:
//! - [`sync_notes_across`] — fetches encrypted notes once,
//!   trial-decrypts against the union of every subwallet's IVK,
//!   appends commitments to the shared tree exactly once per
//!   position with `marked = any subwallet decrypted it`, saves
//!   decrypted notes scoped per-`SubwalletId`, AND performs
//!   scan-based spend detection: every scanned action's
//!   `nullifier` (the nullifier of the note that action spent) is
//!   replayed against each subwallet's store via `mark_spent`, so
//!   spends ride the same note-scan watermark with no separate
//!   nullifier-sync round-trip.
//! - [`balances_across`] — pure unspent-balance read against
//!   the shared store.
//!
//! [`NetworkShieldedCoordinator::sync`] drives both in
//! sequence against the union of every registered subwallet.
//! Per-wallet `PlatformWallet` shielded methods read from the
//! same store via the coordinator handle they're handed at
//! call time (post-Phase-4d.3 — no more `ShieldedWallet`
//! wrapper).
//!
//! [`NetworkShieldedCoordinator::sync`]:
//!     super::coordinator::NetworkShieldedCoordinator::sync

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use dash_sdk::platform::shielded::{
    sync_shielded_notes_stream, try_decrypt_note, try_recover_outgoing_note,
};
use futures::StreamExt;
use grovedb_commitment_tree::{ExtractedNoteCommitment, Note as OrchardNote, PaymentAddress};
use tokio::sync::RwLock;
use tracing::{debug, info};

use super::keys::AccountViewingKeys;
use super::store::{ShieldedNote, ShieldedStore, SubwalletId};
use crate::changeset::ShieldedChangeSet;
use crate::error::PlatformWalletError;

/// On-chain MMR chunk size for the shielded notes tree — must stay
/// in lock-step with `SHIELDED_NOTES_CHUNK_POWER` in
/// `rs-drive/src/drive/shielded/paths.rs` (`1 << 11 = 2048`).
/// `start_index` aligns to this regardless of how many chunks one
/// query spans; multi-chunk fetches still have to begin on a chunk
/// boundary because that's what the server-side range proof
/// bisects against. Hardcoded rather than imported because
/// `rs-platform-wallet` doesn't depend on `drive` directly — bump
/// here and there together if the chunk power ever changes.
const CHUNK_SIZE: u64 = 2048;

/// Result of one note-sync pass.
#[derive(Debug, Clone, Default)]
pub struct SyncNotesResult {
    /// Per-account count of new notes discovered in this pass.
    pub new_notes_per_account: BTreeMap<u32, usize>,
    /// Number of **new** positions scanned this pass — i.e.
    /// commitments at positions `>= already_have` that the SDK
    /// returned. Re-scanned positions (the partial-chunk
    /// re-fetch that Platform's chunked sync semantics force on
    /// every pass while the buffer chunk is mutable) are
    /// excluded so the cumulative counter the host displays
    /// reflects "new work seen", not wire-level fetch volume.
    pub total_scanned: u64,
}

impl SyncNotesResult {
    /// Total new notes across every account.
    pub fn total_new_notes(&self) -> usize {
        self.new_notes_per_account.values().sum()
    }
}

/// Summary of a full sync (notes + nullifiers + balances).
#[derive(Debug, Clone, Default)]
pub struct ShieldedSyncSummary {
    /// Note-sync result.
    pub notes_result: SyncNotesResult,
    /// Per-account count of notes newly detected as spent.
    pub newly_spent_per_account: BTreeMap<u32, usize>,
    /// Per-account unspent balance after sync.
    pub balances: BTreeMap<u32, u64>,
    /// True when the pass was short-circuited by the
    /// caught-up cooldown (see [`super::CAUGHT_UP_COOLDOWN`]) —
    /// no SDK fetch, no trial-decrypt, no nullifier scan. The
    /// `balances` field is still populated from local state so
    /// hosts can keep balance displays current, but counters /
    /// last-sync timestamps should treat this as a no-op
    /// distinct from a genuine "ran and found nothing" pass.
    /// `false` for any pass that actually walked Platform.
    pub is_cooldown_skip: bool,
}

impl ShieldedSyncSummary {
    /// Sum of unspent balances across accounts.
    pub fn balance_total(&self) -> u64 {
        self.balances.values().copied().sum()
    }

    /// Sum of newly-spent counts across accounts.
    pub fn total_newly_spent(&self) -> usize {
        self.newly_spent_per_account.values().sum()
    }
}

/// Result of a multi-subwallet note-sync pass.
///
/// Produced by [`sync_notes_across`] and consumed by the
/// coordinator's [`NetworkShieldedCoordinator::sync`] flow.
///
/// `total_scanned` is a property of the **network fetch**, not of
/// any individual subwallet — every subwallet on the network
/// sees the same set of new positions in the same order, so
/// surfacing per-subwallet `total_scanned` would just duplicate
/// the same number `N` times.
///
/// [`NetworkShieldedCoordinator::sync`]:
///     super::coordinator::NetworkShieldedCoordinator::sync
#[derive(Debug, Clone, Default)]
pub struct MultiSyncNotesResult {
    /// Per-subwallet count of new notes discovered in this pass.
    pub per_subwallet_new_notes: BTreeMap<SubwalletId, usize>,
    /// Per-subwallet count of notes newly detected as spent in this
    /// pass via scan-based nullifier matching (each scanned action's
    /// `nullifier` replayed against the per-subwallet store). Replaces
    /// the count the removed dedicated nullifier-sync pass produced.
    pub per_subwallet_newly_spent: BTreeMap<SubwalletId, usize>,
    /// Wire-level scan volume this pass — encrypted notes pulled from
    /// Platform (decrypted + skipped), computed as `(aligned_start +
    /// total_notes_scanned).saturating_sub(already_have)`. This is the
    /// host-visible "Scanned" counter and is deliberately NOT the count
    /// of newly-appended tree positions (the tree_size append gate makes
    /// the two diverge on re-fetch). See
    /// [`SyncNotesResult::total_scanned`] for the rationale.
    pub total_scanned: u64,
    /// Accumulated persistence changeset spanning every touched
    /// subwallet. The caller decides whether to queue it on the
    /// shared `WalletPersister`.
    pub changeset: ShieldedChangeSet,
}

impl MultiSyncNotesResult {
    /// Total new notes across every subwallet.
    pub fn total_new_notes(&self) -> usize {
        self.per_subwallet_new_notes.values().sum()
    }

    /// Split out the per-account map for `wallet_id`. Useful for
    /// callers that want to feed a single wallet's slice back into
    /// the legacy per-wallet [`SyncNotesResult`] shape.
    pub fn per_account_for(
        &self,
        wallet_id: crate::wallet::platform_wallet::WalletId,
    ) -> BTreeMap<u32, usize> {
        self.per_subwallet_new_notes
            .iter()
            .filter(|(id, _)| id.wallet_id == wallet_id)
            .map(|(id, &c)| (id.account_index, c))
            .collect()
    }
}

/// Single-fetch, multi-IVK trial-decrypt across an arbitrary set
/// of registered subwallets — the Phase 2b primitive that
/// collapses N per-wallet SDK calls into one.
///
/// The first subwallet's IVK drives the SDK call; the SDK's
/// `result.all_notes` is then locally trial-decrypted against
/// every other subwallet's IVK. Commitments are appended to the
/// shared tree exactly once per global position with `marked =
/// (any subwallet decrypted this position)` — so accounts that
/// belong to different wallets but share the same network all
/// see their notes' authentication paths retained.
///
/// `subwallets` must be non-empty; otherwise the function
/// returns an empty result without contacting Platform.
///
/// Privilege boundary: only the viewing-key half is required
/// (FVK for nullifier derivation, IVK for trial decryption).
/// No `SpendAuthorizingKey` is needed by sync — the spend
/// surface re-attaches it at call time.
pub(super) async fn sync_notes_across<S: ShieldedStore>(
    sdk: &Arc<dash_sdk::Sdk>,
    store: &Arc<RwLock<S>>,
    subwallets: &[(SubwalletId, AccountViewingKeys)],
    on_progress: Option<&super::coordinator::ShieldedProgressCallback>,
    on_tree_progress: Option<&super::coordinator::ShieldedTreeProgressCallback>,
) -> Result<MultiSyncNotesResult, PlatformWalletError> {
    if subwallets.is_empty() {
        return Ok(MultiSyncNotesResult::default());
    }

    // Snapshot each subwallet's watermark and the shared tree's
    // current leaf count. Two distinct quantities drive the rest
    // of this pass:
    //   * `already_have` = the LOWEST per-subwallet watermark.
    //     Drives the SDK fetch start so a lagging (e.g. late-bound)
    //     subwallet's notes are re-scanned. A caught-up subwallet
    //     shares the same watermark, so in the common case this is
    //     just that one value.
    //   * `tree_size` = the number of commitments already in the
    //     shared tree. Drives the append gate (below) so the
    //     re-fetch from a chunk boundary doesn't double-append
    //     positions the tree already holds.
    // The per-subwallet `watermarks` map gates note saving so a
    // caught-up subwallet doesn't re-derive nullifiers for notes
    // it already stored, while a lagging one still saves from its
    // own start.
    let (watermarks, already_have, tree_size) = {
        let store = store.read().await;
        let mut watermarks: BTreeMap<SubwalletId, u64> = BTreeMap::new();
        let mut min_idx: Option<u64> = None;
        for (id, _) in subwallets {
            let idx = store
                .last_synced_note_index(*id)
                .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
            watermarks.insert(*id, idx);
            min_idx = Some(min_idx.map_or(idx, |m| m.min(idx)));
        }
        let tree_size = store
            .tree_size()
            .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
        (watermarks, min_idx.unwrap_or(0), tree_size)
    };
    let aligned_start = (already_have / CHUNK_SIZE) * CHUNK_SIZE;

    info!(
        subwallets = subwallets.len(),
        already_have, aligned_start, tree_size, "Starting multi-subwallet shielded note sync"
    );

    // Denominator for the tree-progress ("checked") bar: the on-chain
    // total leaf count of the shielded `CommitmentTree`. This now comes
    // from the note-fetch proofs themselves — every streamed batch carries
    // `total_count`, extracted from the SAME proof that delivers the notes
    // (the parent CommitmentTree element is always present in it). That
    // removes the separate up-front `GetShieldedNotesCount` RPC: the
    // denominator works against currently-deployed nodes with no dependency
    // on that RPC being deployed, and costs no extra round-trip.
    //
    // Seeded from the local `tree_size`, not 0: the on-chain count is
    // append-only, so the real total can never legitimately fall below what
    // we have already committed locally, and both numerators are floored at
    // `tree_size` (the download clamp below and `leaves_committed`'s seed).
    // Starting at 0 would let a first batch proven by a lagging node report
    // `total_count < tree_size`, making the numerators briefly read above the
    // denominator (progress > 100%) until a fresher chunk arrives. It is then
    // raised to `batch.total_count` as batches land. On a cold sync
    // `tree_size == 0`, so this preserves the host's indeterminate handling
    // (total 0) until the first batch lands.
    let mut total_target: u64 = tree_size;

    // Drive the FIRST subwallet's IVK as the streaming driver. Its hits
    // come back per batch as `batch.decrypted`; every other subwallet's
    // are produced by local trial-decryption against `batch.notes`.
    let (driver_id, driver_views) = &subwallets[0];
    let driver_ivk = driver_views.prepared_ivk.clone();
    // Network-only config carrying the caller's "downloaded" progress
    // callback; the SDK fires it once per completed network chunk inside
    // the stream. The tree-progress ("checked") callback is owned by
    // this consumer and fired below — it never travels through the SDK
    // config (the SDK doesn't append to a tree).
    // Fetch up to 16 chunks concurrently (default is 4) to parallelize
    // across the network's ~13 nodes. The per-chunk cost is dominated by
    // server-side proof generation, so more in-flight requests is the main
    // client-side lever; the pull-based stream still caps in-flight fetches
    // at this bound and keeps memory bounded.
    let mut sync_config = dash_sdk::platform::shielded::notes_sync::types::ShieldedSyncConfig {
        max_concurrent: 16,
        ..Default::default()
    };
    if let Some(cb) = on_progress {
        // Clamp the SDK's "downloaded" value up to the pre-stream tree leaf
        // count before forwarding it to the host. The SDK reports downloaded
        // as `aligned_start + scanned`, where `aligned_start` is the rewound
        // MIN watermark across subwallets and can sit far below `tree_size`
        // (a second subwallet binding at watermark 0 collapses it to 0; a
        // partial-chunk-tail resume rewinds it below the tail). The "checked"
        // signal is the absolute tree size, so over the gate-skipped re-scan
        // region (`global_pos < tree_size`, no new appends) the unclamped
        // download value reads *below* checked and breaks the advertised
        // `Checked ≤ Downloaded ≤ total` invariant. Everything already in the
        // tree was necessarily downloaded on a prior pass, so counting it as
        // downloaded is accurate; the clamp is a no-op once the stream
        // advances past `tree_size`, and the cold-sync case (tree_size == 0)
        // is entirely unaffected.
        let cb = cb.clone();
        let tree_baseline = tree_size;
        sync_config.on_chunk_completed = Some(Arc::new(move |downloaded: u64, height: u64| {
            cb(downloaded.max(tree_baseline), height);
        }));
    }

    // Acquire the store write lock for the whole interleaved consume.
    // This is the only writer during a pass, so holding it across
    // `stream.next().await` is safe: the stream's producer side is the
    // SDK's network fetch loop, which never touches the store, so there
    // is no lock-ordering deadlock. Backpressure is pull-based — a slow
    // append simply polls the stream less often, capping in-flight
    // network fetches at `max_concurrent`.
    let mut store = store.write().await;

    let stream = sync_shielded_notes_stream(sdk, &driver_ivk, aligned_start, Some(sync_config));
    futures::pin_mut!(stream);

    // Route decryptions to the subwallet that owns the IVK, accumulated
    // across every batch.
    let mut decrypted_by_subwallet: BTreeMap<SubwalletId, Vec<DiscoveredNote>> = BTreeMap::new();

    // Route OVK-recovered outgoing (SENT) notes to the subwallet whose
    // OVK opened `out_ciphertext`, accumulated across every batch.
    // A note only recovers under the OVK of the wallet that SENT it, so
    // this captures exactly the wallet's own send history. Recorded into
    // the store + changeset after this pass's incoming receipts persist,
    // mirroring the spend-detection ordering. `cmx`-idempotent at the
    // store, so collecting across re-fetched (gate-skipped) positions is
    // harmless.
    let mut recovered_outgoing_by_subwallet: BTreeMap<SubwalletId, Vec<RecoveredOutgoing>> =
        BTreeMap::new();

    // Cumulative tree-append bookkeeping, interleaved with the fetch.
    //
    // Append every commitment to the shared tree exactly once per
    // position, ALWAYS retained (`marked = true`). Skip positions
    // already in the tree (`global_pos < tree_size`) — the SDK
    // re-fetches from a chunk boundary every pass while the buffer
    // chunk is mutable, and a lagging subwallet rewinds that start
    // even further, so the streamed notes routinely overlap positions
    // the tree already holds. Gating on the tree's snapshot leaf count
    // captured BEFORE the stream (NOT a per-subwallet watermark, and
    // NOT the live tree size which we are actively growing) is what
    // makes the append idempotent: re-appending an existing position
    // duplicates a leaf, corrupts shardtree's internal nodes, and makes
    // per-position witnesses resolve against roots Platform never
    // recorded ("Anchor not found in the recorded anchors tree").
    //
    // Why mark every position rather than only owned ones: the
    // commitment tree is a single chain-wide structure shared by
    // every wallet on the network (the whole point of the
    // coordinator refactor — one SQLite handle, not N). Ownership
    // is decided per-pass by trial-decryption, but wallets bind at
    // *different* times. If wallet A syncs first (driver IVK owns
    // nothing) the positions get appended; when wallet B binds
    // later and discovers its note at one of those positions,
    // shardtree has no way to retroactively mark it — the auth
    // path was already discarded as `Ephemeral`, and the note
    // becomes permanently unwitnessable (balance shows, spend
    // fails with "Merkle witness unavailable"). Marking every
    // position makes the shared tree witness-complete regardless
    // of bind ordering; per-wallet ownership is tracked
    // separately in the per-`SubwalletId` notes store, so privacy
    // / accounting is unaffected. The cost is retained auth paths
    // for non-owned positions (O(commitments) storage); acceptable
    // for correctness, and the shielded pool is small. A future
    // optimization can prune auth paths for positions no live
    // subwallet owns once all wallets have caught past them.
    let mut appended = 0u32;
    // Cumulative leaves committed to the tree this pass — the
    // tree-progress ("checked") signal numerator. Includes only
    // positions we actually appended (gate-skipped re-fetched
    // positions are already in the tree, so they don't advance the
    // commit count beyond `tree_size`).
    let mut leaves_committed: u64 = tree_size;
    // Cumulative notes scanned across every batch (decrypted + skipped)
    // — drives the watermark advance and the host-visible scan volume,
    // exactly as `result.total_notes_scanned` did in the one-shot path.
    let mut total_notes_scanned: u64 = 0;
    // Track the last non-empty batch's `(start_index, is_partial)` —
    // diagnostic only. The one-shot path used this to rewind its resume
    // point to the mutable buffer chunk's start; the streaming path's
    // watermark is `aligned_start + total_notes_scanned` (the partial
    // chunk is re-fetched via the chunk-boundary realignment at the
    // next pass's start instead), so this value never feeds the
    // watermark — it is surfaced in the completion log for parity with
    // the SDK's `next_start_index` semantics.
    let mut last_nonempty: Option<(u64, bool)> = None;

    // Scan-based spend detection (replaces the dedicated
    // nullifier-sync pass). EVERY scanned action carries the
    // `nullifier` of the note it SPENT (in Orchard the output note's
    // rho == the input note's nullifier), so the union of every
    // batch's note nullifiers is exactly the set of nullifiers that
    // went on-chain across the scanned range. We accumulate them
    // here and, after this pass's freshly-decrypted receipts are
    // persisted below, replay them against each subwallet via
    // `store.mark_spent` (a no-op for nullifiers the wallet doesn't
    // own). Receipt-before-spend ordering holds: a note must be
    // received (and stored, with its nullifier indexed) before its
    // spend can match, and tree/block order always places receipt
    // ahead of spend — within this pass the receipt save runs first,
    // and across passes the receipt was persisted on an earlier pass,
    // so `mark_spent`'s by-nullifier lookup still resolves.
    let mut scanned_nullifiers: Vec<[u8; 32]> = Vec::new();

    while let Some(item) = stream.next().await {
        let batch = item.map_err(|e| PlatformWalletError::ShieldedSyncFailed(e.to_string()))?;
        // The denominator arrives with the batch (extracted from the
        // note-fetch proof). It is stable across a sync; take the max-seen
        // so a late chunk proven at a slightly higher block never lowers
        // it. Stays 0 (indeterminate) only if no batch is ever produced.
        total_target = total_target.max(batch.total_count);
        total_notes_scanned += batch.notes.len() as u64;
        if !batch.notes.is_empty() {
            last_nonempty = Some((batch.start_index, batch.is_partial));
        }

        // 1. Append commitments for THIS batch, applying the same
        //    idempotency gate against the pre-stream `tree_size`
        //    snapshot. `batch.start_index + i` is the global tree
        //    position. Each action's `nullifier` is the nullifier of
        //    the note that action SPENT — collect every one across the
        //    whole scan for the spend-match replay after receipts are
        //    persisted. These are collected for ALL positions (even
        //    those gate-skipped for tree append: a re-fetched chunk can
        //    still surface a spend whose receipt only persisted on a
        //    later pass), since `mark_spent` is an idempotent no-op for
        //    nullifiers the wallet doesn't own or already marked spent.
        for (i, raw_note) in batch.notes.iter().enumerate() {
            // A malformed nullifier length means the proven note item is corrupt;
            // fail fast (consistent with the cmx check below) rather than silently
            // dropping it — a dropped nullifier would leave a spent note marked
            // unspent.
            let nf_bytes = <[u8; 32]>::try_from(raw_note.nullifier.as_slice()).map_err(|_| {
                PlatformWalletError::ShieldedSyncFailed("Invalid nullifier length".into())
            })?;
            scanned_nullifiers.push(nf_bytes);
            let global_pos = batch.start_index + i as u64;
            if global_pos < tree_size {
                continue;
            }
            let cmx_bytes: [u8; 32] = raw_note.cmx.as_slice().try_into().map_err(|_| {
                PlatformWalletError::ShieldedSyncFailed("Invalid cmx length".into())
            })?;
            store
                .append_commitment(&cmx_bytes, true)
                .map_err(|e| PlatformWalletError::ShieldedTreeUpdateFailed(e.to_string()))?;
            appended += 1;
            leaves_committed += 1;
        }

        // 2. Fire the tree-progress ("checked") callback once per batch
        //    (already coarse at ~8192-note batches). `total_target` is
        //    sourced from this batch's proof above; it is set by the first
        //    batch and stable thereafter. It is 0 (indeterminate on the
        //    host) only before any batch lands — which can't happen inside
        //    this loop since we're holding a batch.
        if let Some(cb) = on_tree_progress {
            cb(leaves_committed, total_target);
        }

        // 3. Trial-decrypt THIS batch. Driver hits come pre-decrypted
        //    in `batch.decrypted`; other subwallets via local
        //    trial-decryption over `batch.notes`.
        for dn in batch.decrypted {
            decrypted_by_subwallet
                .entry(*driver_id)
                .or_default()
                .push(DiscoveredNote {
                    position: dn.position,
                    cmx: dn.cmx,
                    note: dn.note,
                    block_height: batch.block_height,
                });
        }
        for (id, views) in subwallets.iter().skip(1) {
            for (i, raw_note) in batch.notes.iter().enumerate() {
                let position = batch.start_index + i as u64;
                if let Some((note, _addr)) = try_decrypt_note(&views.prepared_ivk, raw_note) {
                    let cmx_bytes: [u8; 32] = match raw_note.cmx.as_slice().try_into() {
                        Ok(b) => b,
                        Err(_) => continue,
                    };
                    decrypted_by_subwallet
                        .entry(*id)
                        .or_default()
                        .push(DiscoveredNote {
                            position,
                            cmx: cmx_bytes,
                            note,
                            block_height: batch.block_height,
                        });
                }
            }
        }

        // 4. OVK recovery (outgoing/SENT notes). Attempt to open each
        //    scanned note's `out_ciphertext` with EVERY subwallet's OVK
        //    (driver included — unlike IVK trial-decrypt, the driver's
        //    sent notes are NOT pre-recovered by the SDK). A note only
        //    recovers under the OVK of the wallet that sent it, so this
        //    is precisely each subwallet's own send history; a received
        //    note (or another wallet's send, or a dummy output) yields
        //    `None` and is skipped. Recovered notes are accumulated and
        //    recorded after this pass's incoming receipts persist (the
        //    record is `cmx`-idempotent at the store).
        for (id, views) in subwallets.iter() {
            for raw_note in batch.notes.iter() {
                if let Some((note, recipient, memo)) =
                    try_recover_outgoing_note(&views.outgoing_viewing_key, raw_note)
                {
                    recovered_outgoing_by_subwallet
                        .entry(*id)
                        .or_default()
                        .push(RecoveredOutgoing {
                            note,
                            recipient,
                            memo,
                            block_height: batch.block_height,
                        });
                }
            }
        }
    }

    // Diagnostic mirror of the SDK's one-shot `next_start_index`
    // (rewinds to the last partial chunk's start). NOT the watermark:
    // the streaming path persists `aligned_start + total_notes_scanned`
    // below, and re-covers the partial buffer chunk through the
    // chunk-boundary realignment at the next pass's start. The old
    // "next_start_index is 0 — next sync will rescan from the
    // beginning" warning keyed off this value was therefore wrong for
    // any single-partial-batch cold scan (a from-scratch rescan always
    // tripped it while the watermark advanced correctly), and sent a
    // real field investigation chasing the wrong mechanism — dropped.
    let next_start_index = match last_nonempty {
        Some((s, true)) => s,
        _ => aligned_start + total_notes_scanned,
    };
    info!(
        total_scanned = total_notes_scanned,
        decrypted_for_driver = decrypted_by_subwallet
            .get(driver_id)
            .map(|v| v.len())
            .unwrap_or(0),
        next_start_index,
        "SDK stream consumed"
    );

    if appended > 0 {
        // Checkpoint at the tree's true post-append leaf count. The
        // tree only ever grows, so this id is strictly monotonic
        // and collision-free across consecutive syncs — unlike
        // `result.next_start_index` (rewinds to the last partial
        // chunk's start) or `aligned_start + total_notes_scanned`
        // (can repeat when a lagging subwallet rewinds the fetch).
        // shardtree's `checkpoint(id)` silently dedups duplicate
        // ids; a non-monotonic id pins depth-0 at the first
        // checkpoint while later appends extend the tree past it,
        // so the depth-0 witness then reflects a state Platform
        // never recorded.
        let new_tree_size = store
            .tree_size()
            .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
        // Hard-fail rather than saturate at u32::MAX: a saturated id
        // would reintroduce shardtree's silent-dedup (every checkpoint
        // past the ceiling pins to the same id) — the exact corruption
        // this monotonic-id scheme exists to avoid. Unreachable today
        // (>4.29B notes scanned) but fail loudly before proving.
        let checkpoint_id: u32 = new_tree_size.try_into().map_err(|_| {
            PlatformWalletError::ShieldedTreeUpdateFailed(format!(
                "commitment tree size {new_tree_size} exceeds u32 checkpoint-id range"
            ))
        })?;
        store
            .checkpoint_tree(checkpoint_id)
            .map_err(|e| PlatformWalletError::ShieldedTreeUpdateFailed(e.to_string()))?;
    }

    // Save decrypted notes per subwallet; count new notes per
    // subwallet; build a single consolidated changeset.
    let mut per_subwallet_new_notes: BTreeMap<SubwalletId, usize> = BTreeMap::new();
    let mut changeset = ShieldedChangeSet::default();
    for (id, discovered) in &decrypted_by_subwallet {
        // Look up the FVK for nullifier derivation. The id is
        // guaranteed to come from `subwallets` (we keyed off the
        // same set above), so the find is infallible — but be
        // defensive in case caller passed a malformed slice.
        let Some((_, views)) = subwallets.iter().find(|(s, _)| s == id) else {
            continue;
        };
        // Gate on THIS subwallet's own watermark (not the network
        // min): a caught-up subwallet skips re-deriving nullifiers
        // for notes it already stored, while a lagging one still
        // saves everything from its own start. `save_note` is an
        // idempotent overwrite-by-nullifier, so a stray re-save is
        // harmless — but gating per-subwallet keeps the
        // `per_subwallet_new_notes` count honest.
        let sub_watermark = watermarks.get(id).copied().unwrap_or(0);
        for d in discovered {
            if d.position < sub_watermark {
                continue;
            }
            let nullifier = d.note.nullifier(&views.full_viewing_key);
            let value = d.note.value().inner();
            debug!(
                wallet_id = %hex::encode(id.wallet_id),
                account = id.account_index,
                position = d.position,
                value,
                "Note DECRYPTED"
            );
            let note_data = serialize_note(&d.note);
            // Stamp the note with ITS chunk's proven height — the same
            // per-batch height OVK-recovered outgoing notes get below —
            // never a pass-wide max. The activity deriver clusters
            // incoming and outgoing events by this height, so a bundle's
            // change note and its OVK-recovered send must carry the same
            // value or the bundle splits across two clusters.
            let shielded_note = super::store::ShieldedNote {
                note_data,
                position: d.position,
                cmx: d.cmx,
                nullifier: nullifier.to_bytes(),
                block_height: d.block_height,
                is_spent: false,
                value,
            };
            store
                .save_note(*id, &shielded_note)
                .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
            changeset.record_note(*id, shielded_note);
            *per_subwallet_new_notes.entry(*id).or_default() += 1;
        }
    }

    // Record OVK-recovered outgoing (sent) notes. `record_outgoing_note`
    // is idempotent by `cmx` (returns `false` for a note already on
    // file), so only genuinely new recoveries are pushed onto the
    // changeset — a re-scan that re-recovers the same sent note adds
    // nothing. Unlike incoming receipts there is no nullifier / spend
    // bookkeeping: a sent note is send history, not a spendable note.
    let mut per_subwallet_new_outgoing: BTreeMap<SubwalletId, usize> = BTreeMap::new();
    for (id, recovered) in &recovered_outgoing_by_subwallet {
        for r in recovered {
            let outgoing_note = super::store::ShieldedOutgoingNote {
                cmx: ExtractedNoteCommitment::from(r.note.commitment()).to_bytes(),
                recipient: r.recipient.to_raw_address_bytes().to_vec(),
                value: r.note.value().inner(),
                memo: r.memo.to_vec(),
                block_height: r.block_height,
            };
            let newly = store
                .record_outgoing_note(*id, &outgoing_note)
                .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
            if newly {
                debug!(
                    wallet_id = %hex::encode(id.wallet_id),
                    account = id.account_index,
                    value = outgoing_note.value,
                    "Outgoing note RECOVERED via OVK"
                );
                changeset.record_outgoing_note(*id, outgoing_note);
                *per_subwallet_new_outgoing.entry(*id).or_default() += 1;
            }
        }
    }
    if !per_subwallet_new_outgoing.is_empty() {
        info!(
            new_outgoing_total = per_subwallet_new_outgoing.values().sum::<usize>(),
            "OVK-recovered outgoing notes this pass"
        );
    }

    // Scan-based spend detection. Now that THIS pass's receipts are
    // persisted (above), replay every scanned action's nullifier
    // against each subwallet — see [`apply_scanned_nullifier_spends`]
    // for the ordering rationale.
    let subwallet_ids: Vec<SubwalletId> = subwallets.iter().map(|(id, _)| *id).collect();
    let newly_spent_per_subwallet = apply_scanned_nullifier_spends(
        &mut *store,
        &subwallet_ids,
        &scanned_nullifiers,
        &mut changeset,
    )
    .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;

    // Advance every subwallet's watermark to the same global
    // tree position so the next sync resumes coherently across
    // the union. `total_notes_scanned` is accumulated from every
    // streamed batch's note count — identical to the one-shot path's
    // `result.total_notes_scanned`.
    let new_index = aligned_start + total_notes_scanned;
    for (id, _) in subwallets {
        store
            .set_last_synced_note_index(*id, new_index)
            .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
        changeset.record_synced_index(*id, new_index);
    }
    // Drop the write lock before returning so the caller's
    // persister queue (which may take its own synchronous
    // mutex) doesn't nest under our store lock.
    drop(store);

    info!(
        new_notes_total = per_subwallet_new_notes.values().sum::<usize>(),
        new_index, "Multi-subwallet shielded sync finished"
    );

    // `total_scanned` keeps its host-visible meaning: wire-level scan
    // volume this pass (encrypted notes pulled — decrypted + skipped),
    // matching the exported FFI/Swift/UI "Scanned" contract. This is
    // intentionally NOT `appended`: the tree_size append gate makes the
    // two diverge whenever the SDK re-fetches positions the tree
    // already holds (chunk-boundary realignment, lagging-subwallet
    // rewind), and the host counter is documented as scan throughput,
    // not tree growth.
    let scanned_volume = (aligned_start + total_notes_scanned).saturating_sub(already_have);
    Ok(MultiSyncNotesResult {
        per_subwallet_new_notes,
        per_subwallet_newly_spent: newly_spent_per_subwallet,
        total_scanned: scanned_volume,
        changeset,
    })
}

/// Scan-based spend detection: replay the set of nullifiers seen on
/// scanned actions against each subwallet's note store, marking any
/// owned note whose nullifier appears as spent.
///
/// This is the whole spend-detection mechanism — it replaces the
/// removed dedicated nullifier-sync pass. In Orchard each action
/// reveals the nullifier of the note it SPENT (the output note's rho
/// equals the input note's nullifier), so `scanned_nullifiers` —
/// gathered from every action across the note scan — is exactly the
/// set of nullifiers that went on-chain over the scanned range.
/// [`ShieldedStore::mark_spent`] looks each one up in the
/// per-subwallet nullifier index and flips the matching note's
/// `is_spent`; it returns `false` (a no-op) when the wallet doesn't
/// own that nullifier or already marked the note spent, so
/// dummy/padding actions and other wallets' spends pass through
/// harmlessly.
///
/// # Ordering
///
/// The caller MUST persist this scan's freshly-decrypted receipts
/// *before* invoking this function. A note has to be received (stored,
/// with its nullifier indexed) before its spend can match, and
/// tree/block order always places the receipt ahead of the spend:
/// - **Same-scan** spend (note received and spent within one pass):
///   the receipt was `save_note`d earlier in the same pass, so the
///   index lookup here resolves.
/// - **Cross-scan** spend (note received on an earlier pass, spent in
///   a later action): the receipt was persisted on that earlier pass,
///   so the lookup against the persisted store still resolves even
///   though the note isn't in this pass's decrypted set.
///
/// Returns the per-subwallet count of notes newly flipped to spent and
/// records each match on `changeset` via
/// [`ShieldedChangeSet::record_nullifier_spent`].
fn apply_scanned_nullifier_spends<S: ShieldedStore>(
    store: &mut S,
    subwallet_ids: &[SubwalletId],
    scanned_nullifiers: &[[u8; 32]],
    changeset: &mut ShieldedChangeSet,
) -> Result<BTreeMap<SubwalletId, usize>, S::Error> {
    let mut newly_spent_per_subwallet: BTreeMap<SubwalletId, usize> = BTreeMap::new();
    if scanned_nullifiers.is_empty() {
        return Ok(newly_spent_per_subwallet);
    }
    for id in subwallet_ids {
        let mut spent_count = 0usize;
        for nf in scanned_nullifiers {
            if store.mark_spent(*id, nf)? {
                changeset.record_nullifier_spent(*id, *nf);
                spent_count += 1;
            }
        }
        if spent_count > 0 {
            newly_spent_per_subwallet.insert(*id, spent_count);
            info!(
                wallet_id = %hex::encode(id.wallet_id),
                account = id.account_index,
                spent_count,
                "Notes newly detected as spent (scan-based)"
            );
        }
    }
    Ok(newly_spent_per_subwallet)
}

/// Multi-subwallet unspent-balance snapshot. Pure read against
/// the shared store — does not trigger a sync.
pub(crate) async fn balances_across<S: ShieldedStore>(
    store: &Arc<RwLock<S>>,
    subwallets: &[(SubwalletId, AccountViewingKeys)],
) -> Result<BTreeMap<SubwalletId, u64>, PlatformWalletError> {
    let store = store.read().await;
    let mut out: BTreeMap<SubwalletId, u64> = BTreeMap::new();
    for (id, _) in subwallets {
        let notes = store
            .get_unspent_notes(*id)
            .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
        out.insert(*id, notes.iter().map(|n| n.value).sum());
    }
    Ok(out)
}

/// Resume checkpoint for one foreign-key transient scan.
///
/// [`scan_notes_for_foreign_key`] has no subwallet store to persist a sync
/// watermark into, so without a checkpoint every call restarts the
/// proof-verified note stream at position zero — and a syntactically valid but
/// UNFUNDED invitation key (attacker-controlled input) turns every retry into
/// a full-history rescan (#4313 review finding d19c5cf84a9f). The checkpoint
/// bounds the repeat: within one cache, tree positions below
/// `resume_position` are streamed and trial-decrypted at most once per key, so
/// an unfunded key costs one full-history scan per cache lifetime, after which
/// each retry only covers new tree growth plus the mutable buffer chunk.
///
/// Funds-safety: the commitment tree is append-only and every full chunk is
/// immutable, so nothing below `resume_position` can change after it was
/// scanned; only the final (partial) buffer chunk can still receive notes, and
/// `resume_position` is never advanced past a partial chunk's `start_index` —
/// the same resume rule the subwallet sync applies (see
/// `ShieldedChunkBatch::is_partial`). A resumed scan therefore can never miss
/// a note a from-zero scan would have found. Deliberately in-memory only (no
/// persistence): a fresh process re-pays one full scan, which keeps this a
/// pure work bound with no stored state to invalidate.
#[derive(Clone)]
struct ForeignScanCheckpoint {
    /// First tree position the next scan must cover; every position strictly
    /// below it has already been streamed and trial-decrypted for this key.
    /// Always a full-chunk boundary (and re-aligned down on use).
    resume_position: u64,
    /// Notes that decrypted under the key at positions strictly below
    /// `resume_position`. Positions at/above it are re-derived on resume, so
    /// buffer-chunk notes are never carried here (no duplicates on rescan).
    notes: Vec<ShieldedNote>,
}

/// Bounded, most-recently-used-last checkpoint list keyed by
/// [`foreign_scan_checkpoint_key`]. A `Vec` with linear search: the cap is
/// tiny, and eviction order (front = least recently used) falls out for free.
type ForeignScanCheckpoints = Vec<([u8; 32], ForeignScanCheckpoint)>;

/// Coordinator-owned cache of [`ForeignScanCheckpoint`]s.
///
/// Owned by `NetworkShieldedCoordinator` — NOT process-global — so a
/// checkpoint can never leak across chains (#4313 review findings
/// 6118148e4547 / cr-4d2aa8ce): each coordinator is pinned to one network AND
/// one on-disk tree store, so two devnets that both answer to
/// `Network::Devnet` still get distinct caches, and a resume position
/// computed against one chain's tree can never skip a funded note at an
/// earlier position on another chain's tree. Dropping the coordinator drops
/// its cache — no allocation-address aliasing is possible.
///
/// Concurrency: entries are read with [`load`](Self::load) (clone, NOT
/// remove) and written with [`save`](Self::save), which only advances a
/// key's `resume_position` monotonically. A caller cancelled between the two
/// therefore leaves the previous checkpoint intact instead of destroying it
/// (#4313 review finding cr-4808dde4: the old take-then-put-back scheme lost
/// the entry if the taker's future was dropped mid-scan). Same-key callers
/// are additionally serialized end-to-end by the claim-lifecycle guard
/// (`operations::ForeignClaimGuards`); the internal mutex is sync-only and
/// never held across an await.
#[derive(Default)]
pub struct ForeignScanCheckpointCache {
    entries: Mutex<ForeignScanCheckpoints>,
}

/// At most this many foreign keys keep a checkpoint. One claim flow touches
/// one key, so this covers concurrent/retried claims while capping what
/// hostile key churn can pin in memory (a one-time key funds 1–2 notes, so
/// each entry is small; churn also cannot force rescans of OTHER keys — an
/// evicted key merely re-pays its own full scan).
const FOREIGN_SCAN_CHECKPOINT_CAP: usize = 8;

impl ForeignScanCheckpointCache {
    /// Clone the checkpoint for `key`, if present, marking it most recently
    /// used. The entry stays in the cache — see the type-level concurrency
    /// note.
    fn load(&self, key: &[u8; 32]) -> Option<ForeignScanCheckpoint> {
        let mut map = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        map.iter().position(|(k, _)| k == key).map(|i| {
            let entry = map.remove(i);
            let checkpoint = entry.1.clone();
            map.push(entry);
            checkpoint
        })
    }

    /// Insert/replace the checkpoint for `key` as most recently used,
    /// evicting the least recently used entry beyond
    /// [`FOREIGN_SCAN_CHECKPOINT_CAP`]. Monotonic: an existing entry is only
    /// replaced by one whose `resume_position` is at least as far along, so
    /// no writer can rewind another's progress.
    fn save(&self, key: [u8; 32], checkpoint: ForeignScanCheckpoint) {
        let mut map = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(i) = map.iter().position(|(k, _)| k == &key) {
            if map[i].1.resume_position > checkpoint.resume_position {
                return;
            }
            map.remove(i);
        }
        while map.len() >= FOREIGN_SCAN_CHECKPOINT_CAP {
            map.remove(0);
        }
        map.push((key, checkpoint));
    }
}

/// Deterministic checkpoint key for a foreign one-time key. Domain-separated
/// from `one_time_claim_record_key` (operations.rs) so the two keyspaces can
/// never alias, and hashed so the raw FVK bytes are not retained in the map.
fn foreign_scan_checkpoint_key(fvk: &grovedb_commitment_tree::FullViewingKey) -> [u8; 32] {
    use dashcore::hashes::{sha256, Hash};

    let mut preimage = Vec::with_capacity(96 + 44);
    preimage.extend_from_slice(b"platform-wallet:foreign-scan-checkpoint:v1");
    preimage.extend_from_slice(&fvk.to_bytes());
    sha256::Hash::hash(&preimage).to_byte_array()
}

/// Build the checkpoint to persist after covering the tree through
/// `scanned_through`: only notes on immutable, fully-consumed chunks
/// (position strictly below the resume point) are carried — notes inside the
/// mutable buffer chunk are re-derived on the next pass.
fn foreign_scan_checkpoint_below(
    scanned_through: u64,
    found: &[ShieldedNote],
) -> ForeignScanCheckpoint {
    ForeignScanCheckpoint {
        resume_position: scanned_through,
        notes: found
            .iter()
            .filter(|n| n.position < scanned_through)
            .cloned()
            .collect(),
    }
}

/// Transiently scan the shielded-note set for a FOREIGN Orchard key (the
/// L2-invitation *claim* path).
///
/// Streams the on-chain encrypted notes with `ivk` as the driver key and
/// collects every note that decrypts under it into a store [`ShieldedNote`]
/// (position, cmx, per-`fvk` nullifier, value, and the 115-byte serialized
/// note). Unlike the regular sync path this touches NO store: the notes belong
/// to a one-time invitation spending key that is not tracked in any subwallet,
/// so they are re-derived from the network on demand and never persisted here.
///
/// The scan stops early as soon as the accumulated value reaches
/// `stop_at_value` — a one-time invitation key holds exactly its funding, so
/// there is no reason to keep streaming past the note(s) that fund it. If the
/// key's value never reaches `stop_at_value`, the tree is scanned to the tip
/// and whatever was found is returned; the caller's note selection then
/// surfaces the typed insufficient-value error.
///
/// Note: shielded notes are indexed by tree POSITION and this tree exposes no
/// height→position oracle (a chunk's `block_height` is the proof-tip height, not
/// a per-note inclusion height — see [`ShieldedChunkBatch`]), so a caller's
/// birth-height hint cannot seed the scan start. The rescan bound is instead a
/// coordinator-owned [`ForeignScanCheckpoint`] (in `checkpoints` — see
/// [`ForeignScanCheckpointCache`] for the chain-isolation and
/// cancellation-safety contract): the first scan for a key covers the
/// full history from position 0 (never risking a missed note), and every later
/// scan for the SAME key resumes past the immutable chunks it already covered —
/// so a valid-but-unfunded invitation key costs one full-history scan per
/// coordinator, not one per attempt (#4313 review finding d19c5cf84a9f).
/// Progress is checkpointed even when the stream errors mid-scan, so an
/// interrupted retry resumes rather than restarting. Same-key calls are
/// serialized by the caller's per-FVK claim guard, so two scans never
/// interleave on one key.
///
/// [`ShieldedChunkBatch`]: dash_sdk::platform::shielded::notes_sync::types::ShieldedChunkBatch
pub(crate) async fn scan_notes_for_foreign_key(
    sdk: &Arc<dash_sdk::Sdk>,
    checkpoints: &ForeignScanCheckpointCache,
    fvk: &grovedb_commitment_tree::FullViewingKey,
    ivk: &grovedb_commitment_tree::IncomingViewingKey,
    stop_at_value: u64,
) -> Result<Vec<ShieldedNote>, PlatformWalletError> {
    use grovedb_commitment_tree::PreparedIncomingViewingKey;

    let checkpoint_key = foreign_scan_checkpoint_key(fvk);
    let (mut found, resume_position) = match checkpoints.load(&checkpoint_key) {
        Some(cp) => (cp.notes, cp.resume_position),
        None => (Vec::new(), 0),
    };

    // The stream start must sit on an on-chain MMR chunk boundary; align DOWN
    // so a resume can only over-scan, never skip. Checkpointed notes at/above
    // the aligned start would be re-found by the rescan below — drop them so
    // they cannot duplicate (defensive: persisted resume positions are already
    // chunk-aligned and their notes strictly below).
    let aligned_start = (resume_position / CHUNK_SIZE) * CHUNK_SIZE;
    found.retain(|n| n.position < aligned_start);
    let mut total: u64 = found
        .iter()
        .fold(0u64, |acc, n| acc.saturating_add(n.value));

    if aligned_start > 0 {
        debug!(
            aligned_start,
            checkpointed_notes = found.len(),
            checkpointed_value = total,
            "Foreign-key scan resuming from coordinator-owned checkpoint"
        );
    }

    // Checkpointed notes already cover the requested value: no network work.
    // Safe because note contents at a scanned position are immutable
    // (append-only tree) and spent-ness is not decided here — the caller's
    // selection/preflight re-verifies nullifier status against the chain,
    // exactly as it does for freshly scanned notes.
    if total >= stop_at_value && !found.is_empty() {
        checkpoints.save(
            checkpoint_key,
            foreign_scan_checkpoint_below(aligned_start, &found),
        );
        return Ok(found);
    }

    let prepared = PreparedIncomingViewingKey::new(ivk);
    let stream = sync_shielded_notes_stream(sdk, &prepared, aligned_start, None);
    futures::pin_mut!(stream);

    // How far this pass has FULLY covered the tree: advanced past the end of
    // every immutable full chunk consumed, held AT a partial (buffer) chunk's
    // `start_index` because that chunk may still receive notes.
    let mut scanned_through = aligned_start;
    while let Some(batch) = stream.next().await {
        let batch = match batch {
            Ok(batch) => batch,
            Err(e) => {
                // Persist partial progress: the retry that follows this error
                // resumes here instead of re-paying the whole scan.
                checkpoints.save(
                    checkpoint_key,
                    foreign_scan_checkpoint_below(scanned_through, &found),
                );
                return Err(PlatformWalletError::ShieldedSyncFailed(e.to_string()));
            }
        };
        scanned_through = if batch.is_partial {
            batch.start_index
        } else {
            batch.start_index + batch.notes.len() as u64
        };
        for dn in batch.decrypted {
            let value = dn.note.value().inner();
            let nullifier = dn.note.nullifier(fvk).to_bytes();
            found.push(ShieldedNote {
                position: dn.position,
                cmx: dn.cmx,
                nullifier,
                block_height: batch.block_height,
                is_spent: false,
                value,
                note_data: serialize_note(&dn.note),
            });
            total = total.saturating_add(value);
        }
        // A one-time key holds exactly its funding — stop once it's covered.
        if total >= stop_at_value {
            break;
        }
    }

    checkpoints.save(
        checkpoint_key,
        foreign_scan_checkpoint_below(scanned_through, &found),
    );
    Ok(found)
}

/// One decrypted note discovered during a sync pass.
#[derive(Clone)]
struct DiscoveredNote {
    position: u64,
    cmx: [u8; 32],
    note: OrchardNote,
    /// Proven platform height of the chunk that surfaced this note —
    /// per-batch, matching [`RecoveredOutgoing::block_height`], so that
    /// one bundle's incoming change and OVK-recovered send carry the
    /// same height and cluster together in the activity deriver.
    block_height: u64,
}

/// One outgoing (sent) note recovered via OVK during a sync pass.
///
/// Holds the recovered `(note, recipient, memo)` plus the block height
/// the output appeared at; converted into a
/// [`super::store::ShieldedOutgoingNote`] when recorded (the `cmx`
/// primary key is derived from `note.commitment()` at record time).
#[derive(Clone)]
struct RecoveredOutgoing {
    note: OrchardNote,
    recipient: PaymentAddress,
    memo: [u8; dash_sdk::platform::shielded::DASH_MEMO_SIZE],
    block_height: u64,
}

// Suppress dead_code on `address` field — kept for future use
// (e.g. surfacing diversifier index per discovered note).
#[allow(dead_code)]
fn _unused_payment_address(_pa: PaymentAddress) {}

/// Serialize an Orchard note to bytes for storage.
///
/// Format: `recipient(43) || value(8 LE) || rho(32) || rseed(32)` = 115 bytes.
/// Must be kept in sync with `deserialize_note()` in operations.rs.
fn serialize_note(note: &grovedb_commitment_tree::Note) -> Vec<u8> {
    let mut data = Vec::with_capacity(115);
    data.extend_from_slice(&note.recipient().to_raw_address_bytes());
    data.extend_from_slice(&note.value().inner().to_le_bytes());
    data.extend_from_slice(&note.rho().to_bytes());
    data.extend_from_slice(note.rseed().as_bytes());
    data
}

#[cfg(test)]
mod tests {
    use super::apply_scanned_nullifier_spends;
    use crate::changeset::ShieldedChangeSet;
    use crate::wallet::shielded::store::{
        InMemoryShieldedStore, ShieldedNote, ShieldedStore, SubwalletId,
    };

    fn sub(account: u32) -> SubwalletId {
        SubwalletId::new([0xCC; 32], account)
    }

    /// Build a received (unspent) note carrying `nullifier` at `position`.
    fn received_note(nullifier: [u8; 32], position: u64) -> ShieldedNote {
        ShieldedNote {
            position,
            cmx: [0x11; 32],
            nullifier,
            block_height: 10,
            is_spent: false,
            value: 1_000,
            note_data: vec![0u8; 115],
        }
    }

    /// THE core Part-A guarantee: a note that was RECEIVED (persisted to
    /// the store) and is later SPENT — its nullifier surfacing on a
    /// scanned action in a *subsequent* chunk/pass — is marked
    /// `is_spent` purely through the note-scan spend-detection path
    /// (`apply_scanned_nullifier_spends`). No nullifier-sync, no SDK
    /// call, no checkpoint: just the persisted note's nullifier matched
    /// against the scanned-action nullifier set.
    #[test]
    fn received_then_later_spent_note_is_marked_spent_via_scan() {
        let mut store = InMemoryShieldedStore::new();
        let id = sub(0);
        let owned_nf = [0xAB; 32];

        // Earlier chunk/pass: the note is received and persisted. (In the
        // real loop this is the `save_note` of a trial-decrypted note,
        // which must run before any nullifier replay.)
        store.save_note(id, &received_note(owned_nf, 5)).unwrap();
        assert_eq!(store.get_unspent_notes(id).unwrap().len(), 1);

        // A LATER scanned chunk surfaces a batch of action nullifiers.
        // One of them is `owned_nf` (this note's spend); the rest belong
        // to dummy/padding actions or other wallets and must be no-ops.
        let scanned_nullifiers = vec![[0x01; 32], owned_nf, [0x02; 32]];
        let mut changeset = ShieldedChangeSet::default();

        let newly_spent =
            apply_scanned_nullifier_spends(&mut store, &[id], &scanned_nullifiers, &mut changeset)
                .expect("scan-based spend detection should not error");

        // The note is now spent — detected entirely via the scan path.
        assert_eq!(newly_spent.get(&id).copied(), Some(1));
        assert!(
            store.get_unspent_notes(id).unwrap().is_empty(),
            "received-then-spent note must no longer be unspent"
        );
        let all = store.get_all_notes(id).unwrap();
        assert_eq!(all.len(), 1);
        assert!(all[0].is_spent, "note must be flagged is_spent");

        // The spend is recorded on the changeset for the persister, and
        // ONLY the owned nullifier (the dummies did not match).
        let recorded = changeset.nullifiers_spent.get(&id).expect("spend recorded");
        assert_eq!(recorded.as_slice(), &[owned_nf]);

        // A re-scan that surfaces the same nullifier again is idempotent:
        // `mark_spent` returns false for the already-spent note, so no new
        // spend is reported.
        let mut changeset2 = ShieldedChangeSet::default();
        let again =
            apply_scanned_nullifier_spends(&mut store, &[id], &scanned_nullifiers, &mut changeset2)
                .unwrap();
        assert!(
            again.is_empty(),
            "re-detecting an already-spent note must be a no-op"
        );
        assert!(changeset2.nullifiers_spent.is_empty());
    }

    /// Same-scan case: a note received earlier in the SAME pass (already
    /// persisted by the time the nullifier replay runs) and spent by an
    /// action in that same scan is still caught — the replay looks the
    /// nullifier up in the persisted store, not just this pass's
    /// freshly-decrypted set.
    #[test]
    fn note_received_and_spent_in_same_scan_is_marked_spent() {
        let mut store = InMemoryShieldedStore::new();
        let id = sub(0);
        let owned_nf = [0xCD; 32];

        // Receipt persisted first (mirrors the save-loop running before
        // the replay within one `sync_notes_across` pass).
        store.save_note(id, &received_note(owned_nf, 0)).unwrap();

        // The same pass's scanned actions include this note's spend.
        let scanned = vec![owned_nf];
        let mut changeset = ShieldedChangeSet::default();
        let newly_spent =
            apply_scanned_nullifier_spends(&mut store, &[id], &scanned, &mut changeset).unwrap();

        assert_eq!(newly_spent.get(&id).copied(), Some(1));
        assert!(store.get_unspent_notes(id).unwrap().is_empty());
    }

    /// Scanned nullifiers the wallet does not own (every action belongs to
    /// other wallets / dummies) never touch this subwallet's notes.
    #[test]
    fn unowned_scanned_nullifiers_are_noops() {
        let mut store = InMemoryShieldedStore::new();
        let id = sub(0);
        store.save_note(id, &received_note([0x55; 32], 0)).unwrap();

        let scanned = vec![[0x01; 32], [0x02; 32], [0x03; 32]];
        let mut changeset = ShieldedChangeSet::default();
        let newly_spent =
            apply_scanned_nullifier_spends(&mut store, &[id], &scanned, &mut changeset).unwrap();

        assert!(newly_spent.is_empty());
        assert_eq!(
            store.get_unspent_notes(id).unwrap().len(),
            1,
            "an unowned scanned nullifier must not spend our note"
        );
        assert!(changeset.nullifiers_spent.is_empty());
    }

    /// Spend detection is scoped per subwallet: two subwallets each own a
    /// note, and only the one whose nullifier appears in the scan is
    /// marked spent.
    #[test]
    fn scan_spend_detection_is_per_subwallet() {
        let mut store = InMemoryShieldedStore::new();
        let a = sub(0);
        let b = sub(1);
        let nf_a = [0xA0; 32];
        let nf_b = [0xB0; 32];
        store.save_note(a, &received_note(nf_a, 0)).unwrap();
        store.save_note(b, &received_note(nf_b, 1)).unwrap();

        // Only subwallet A's nullifier is on-chain this scan.
        let scanned = vec![nf_a];
        let mut changeset = ShieldedChangeSet::default();
        let newly_spent =
            apply_scanned_nullifier_spends(&mut store, &[a, b], &scanned, &mut changeset).unwrap();

        assert_eq!(newly_spent.get(&a).copied(), Some(1));
        assert!(!newly_spent.contains_key(&b));
        assert!(store.get_unspent_notes(a).unwrap().is_empty());
        assert_eq!(store.get_unspent_notes(b).unwrap().len(), 1);
    }

    /// Note at `position` worth `value` (checkpoint tests don't care about
    /// nullifiers).
    fn note_at(position: u64, value: u64) -> ShieldedNote {
        ShieldedNote {
            position,
            cmx: [0x22; 32],
            nullifier: [0x33; 32],
            block_height: 10,
            is_spent: false,
            value,
            note_data: vec![0u8; 115],
        }
    }

    /// The checkpoint carries only notes on immutable, fully-consumed chunks
    /// (position strictly below the resume point); buffer-chunk notes are
    /// dropped so the rescan of that chunk cannot duplicate them.
    #[test]
    fn foreign_scan_checkpoint_below_drops_buffer_chunk_notes() {
        let found = vec![note_at(5, 100), note_at(2047, 200), note_at(2048, 300)];

        let cp = super::foreign_scan_checkpoint_below(2048, &found);

        assert_eq!(cp.resume_position, 2048);
        let positions: Vec<u64> = cp.notes.iter().map(|n| n.position).collect();
        assert_eq!(
            positions,
            vec![5, 2047],
            "the note AT the resume position sits in the still-mutable buffer \
             chunk and must be re-derived next pass, not carried"
        );
    }

    /// Checkpoint cache semantics: load clones without removing, save
    /// replaces monotonically, and the least-recently-used entry is evicted
    /// beyond the cap.
    #[test]
    fn foreign_scan_checkpoint_cache_load_save_and_evict() {
        let cache = super::ForeignScanCheckpointCache::default();
        let key = |i: u8| -> [u8; 32] { [0xE0 + i; 32] };
        let cp = |resume: u64| super::ForeignScanCheckpoint {
            resume_position: resume,
            notes: vec![note_at(1, 42)],
        };

        // Missing key: nothing to load.
        assert!(cache.load(&key(0)).is_none());

        // Round-trip: save then load returns the entry WITHOUT removing it —
        // a caller cancelled after a load must leave the checkpoint intact
        // for the next attempt (review finding cr-4808dde4).
        cache.save(key(0), cp(2048));
        let got = cache.load(&key(0)).expect("saved checkpoint");
        assert_eq!(got.resume_position, 2048);
        assert_eq!(got.notes.len(), 1);
        assert!(
            cache.load(&key(0)).is_some(),
            "load must NOT remove the entry (cancellation between load and \
             save would otherwise destroy the resume progress)"
        );

        // Save for an existing key replaces rather than duplicates…
        cache.save(key(0), cp(4096));
        let got = cache.load(&key(0)).expect("replaced checkpoint");
        assert_eq!(got.resume_position, 4096, "farther save must win");
        // …but only monotonically: a stale writer cannot rewind progress.
        cache.save(key(0), cp(2048));
        let got = cache.load(&key(0)).expect("checkpoint after stale save");
        assert_eq!(
            got.resume_position, 4096,
            "an older resume position must never replace a newer one"
        );

        // Fill one past the cap with fresh keys: the oldest entry is evicted,
        // the rest live.
        let n = super::FOREIGN_SCAN_CHECKPOINT_CAP as u8 + 1;
        let cache = super::ForeignScanCheckpointCache::default();
        for i in 0..n {
            cache.save(key(i), cp(u64::from(i) * 2048));
        }
        assert!(
            cache.load(&key(0)).is_none(),
            "least-recently-used entry must be evicted beyond the cap"
        );
        for i in 1..n {
            assert!(
                cache.load(&key(i)).is_some(),
                "entry {i} must survive the eviction"
            );
        }
    }

    /// Chain isolation: the cache is an instance owned by ONE coordinator
    /// (one network, one tree store), so the same foreign key checkpointed
    /// through one coordinator must be invisible to another — a resume
    /// position computed against one chain's tree can never skip a funded
    /// note at an earlier position on a different chain (review findings
    /// 6118148e4547 / cr-4d2aa8ce; covers two devnets that share
    /// `Network::Devnet`).
    #[test]
    fn foreign_scan_checkpoints_do_not_cross_cache_instances() {
        let mainnet_like = super::ForeignScanCheckpointCache::default();
        let devnet_like = super::ForeignScanCheckpointCache::default();
        let key = [0xAB; 32];

        mainnet_like.save(
            key,
            super::ForeignScanCheckpoint {
                resume_position: 4096,
                notes: vec![note_at(1, 42)],
            },
        );

        assert!(
            devnet_like.load(&key).is_none(),
            "a checkpoint saved through one coordinator's cache must not be \
             visible through another's"
        );
        assert_eq!(
            mainnet_like
                .load(&key)
                .expect("own checkpoint stays visible")
                .resume_position,
            4096
        );
    }
}

/// OVK outgoing-note recovery: round-trip a real Orchard output
/// encrypted with the wallet's OVK back into a `ShieldedOutgoingNote`.
///
/// These tests exercise the exact client-side recovery primitive the
/// scan calls (`dash_sdk::platform::shielded::try_recover_outgoing_note`)
/// plus the store record path (`record_outgoing_note`) that
/// `sync_notes_across` drives — without standing up a live SDK / network
/// stream (the same way the Part-A tests exercise the extracted
/// `apply_scanned_nullifier_spends` helper rather than the full async
/// `sync_notes_across`).
#[cfg(test)]
mod ovk_recovery_tests;

/// Round-trip guard for the Type 15 client pair: the shield builder's
/// serialized actions must trial-decrypt under the same keyset's IVK
/// (the chain stores them verbatim, so this covers the full path).
#[cfg(test)]
mod shield_decrypt_tests;

/// Sender-side mirror of `shield_decrypt_tests`: the shield builder's
/// serialized actions must OVK-recover (recipient, value, memo) under
/// the same keyset's outgoing viewing key and persist as an outgoing
/// note — the wallet's own send history reconstructed from chain data.
#[cfg(test)]
mod ovk_builder_roundtrip_tests;

/// Round-trip guard for the shielded note memo: a `ShieldedMemo` attached
/// to an output survives encryption and comes back out of both the IVK
/// full-decryption and the OVK send-history recovery primitives.
#[cfg(test)]
mod memo_roundtrip_tests;
