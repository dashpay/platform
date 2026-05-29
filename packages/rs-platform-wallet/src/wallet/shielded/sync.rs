//! Shielded note + nullifier synchronization.
//!
//! The heavy lifting lives in three free functions that take a
//! flat `&[(SubwalletId, AccountViewingKeys)]` slice and drive a
//! single network-wide SDK fetch per pass:
//! - [`sync_notes_across`] — fetches encrypted notes once,
//!   trial-decrypts against the union of every subwallet's IVK,
//!   appends commitments to the shared tree exactly once per
//!   position with `marked = any subwallet decrypted it`, and
//!   saves decrypted notes scoped per-`SubwalletId`.
//! - [`check_nullifiers_across`] — privacy-preserving nullifier
//!   scan per subwallet (the SDK's nullifier-scan API is
//!   per-checkpoint, so it stays per-subwallet).
//! - [`balances_across`] — pure unspent-balance read against
//!   the shared store.
//!
//! [`NetworkShieldedCoordinator::sync`] drives all three in
//! sequence against the union of every registered subwallet.
//! Per-wallet `PlatformWallet` shielded methods read from the
//! same store via the coordinator handle they're handed at
//! call time (post-Phase-4d.3 — no more `ShieldedWallet`
//! wrapper).
//!
//! [`NetworkShieldedCoordinator::sync`]:
//!     super::coordinator::NetworkShieldedCoordinator::sync

use std::collections::BTreeMap;
use std::sync::Arc;

use dash_sdk::platform::shielded::nullifier_sync::{NullifierSyncCheckpoint, NullifierSyncConfig};
use dash_sdk::platform::shielded::{sync_shielded_notes_stream, try_decrypt_note};
use futures::StreamExt;
use grovedb_commitment_tree::{Note as OrchardNote, PaymentAddress};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::keys::AccountViewingKeys;
use super::store::{ShieldedStore, SubwalletId};
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
    // It is unknown until the first batch arrives, so it starts at 0
    // (indeterminate — the host's existing indeterminate handling) and is
    // set from `batch.total_count` as soon as the first batch lands.
    let mut total_target: u64 = 0;

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
    let mut sync_config =
        dash_sdk::platform::shielded::notes_sync::types::ShieldedSyncConfig::default();
    if let Some(cb) = on_progress {
        sync_config.on_chunk_completed = Some(cb.clone());
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
    // Max block height across batches and the resume point for
    // `next_start_index` semantics, accumulated as batches arrive.
    let mut max_block_height: u64 = 0;
    // Mirrors the one-shot rewind rule: track the last non-empty
    // batch's `(start_index, is_partial)` so we can warn on a
    // rewind-to-zero just like before.
    let mut last_nonempty: Option<(u64, bool)> = None;

    while let Some(item) = stream.next().await {
        let batch = item.map_err(|e| PlatformWalletError::ShieldedSyncFailed(e.to_string()))?;
        max_block_height = max_block_height.max(batch.block_height);
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
        //    position.
        for (i, raw_note) in batch.notes.iter().enumerate() {
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
                        });
                }
            }
        }
    }

    // Preserve the one-shot `next_start_index` warning: if the resume
    // point would rewind to 0 after scanning notes, the next sync
    // rescans from the beginning.
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
    if next_start_index == 0 && total_notes_scanned > 0 {
        warn!(
            "Shielded sync: next_start_index is 0 after scanning {} notes — \
             next sync will rescan from the beginning",
            total_notes_scanned,
        );
    }

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
            let shielded_note = super::store::ShieldedNote {
                note_data,
                position: d.position,
                cmx: d.cmx,
                nullifier: nullifier.to_bytes(),
                block_height: max_block_height,
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
        total_scanned: scanned_volume,
        changeset,
    })
}

/// Multi-subwallet nullifier sync. One SDK call per subwallet
/// that has unspent notes — the SDK's nullifier-scan API is
/// keyed by a checkpoint per subwallet and can't be coalesced
/// the same way `sync_shielded_notes` can. Still drops the
/// per-wallet `last_caught_up_at` and persister hop, leaving
/// the caller in charge of changeset queueing.
pub(super) async fn check_nullifiers_across<S: ShieldedStore>(
    sdk: &Arc<dash_sdk::Sdk>,
    store: &Arc<RwLock<S>>,
    subwallets: &[(SubwalletId, AccountViewingKeys)],
) -> Result<(BTreeMap<SubwalletId, usize>, ShieldedChangeSet), PlatformWalletError> {
    if subwallets.is_empty() {
        return Ok((BTreeMap::new(), ShieldedChangeSet::default()));
    }

    // Aggregate unspent nullifiers per subwallet so we hit the
    // SDK once per subwallet, then route the `found` results
    // back via a position lookup.
    struct PerSub {
        nullifiers: Vec<[u8; 32]>,
        checkpoint: Option<NullifierSyncCheckpoint>,
    }

    let per_sub: Vec<(SubwalletId, PerSub)> = {
        let store = store.read().await;
        let mut out = Vec::with_capacity(subwallets.len());
        for (id, _) in subwallets {
            let unspent = store
                .get_unspent_notes(*id)
                .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
            let nullifiers: Vec<[u8; 32]> = unspent.iter().map(|n| n.nullifier).collect();
            let checkpoint = store
                .nullifier_checkpoint(*id)
                .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?
                .map(|(height, timestamp)| NullifierSyncCheckpoint { height, timestamp });
            out.push((
                *id,
                PerSub {
                    nullifiers,
                    checkpoint,
                },
            ));
        }
        out
    };

    let mut newly_spent: BTreeMap<SubwalletId, usize> = BTreeMap::new();
    let mut changeset = ShieldedChangeSet::default();
    for (id, sub) in per_sub {
        if sub.nullifiers.is_empty() {
            continue;
        }
        debug!(
            wallet_id = %hex::encode(id.wallet_id),
            account = id.account_index,
            checking = sub.nullifiers.len(),
            ?sub.checkpoint,
            "Checking nullifiers"
        );
        let result = sdk
            .sync_nullifiers(&sub.nullifiers, None::<NullifierSyncConfig>, sub.checkpoint)
            .await
            .map_err(|e| PlatformWalletError::ShieldedNullifierSyncFailed(e.to_string()))?;

        let mut store = store.write().await;
        let mut spent_count = 0usize;
        for nf_bytes in &result.found {
            if store
                .mark_spent(id, nf_bytes)
                .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?
            {
                changeset.record_nullifier_spent(id, *nf_bytes);
                spent_count += 1;
            }
        }
        store
            .set_nullifier_checkpoint(id, result.new_sync_height, result.new_sync_timestamp)
            .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
        changeset.record_nullifier_checkpoint(
            id,
            result.new_sync_height,
            result.new_sync_timestamp,
        );

        if spent_count > 0 {
            newly_spent.insert(id, spent_count);
            info!(
                wallet_id = %hex::encode(id.wallet_id),
                account = id.account_index,
                spent_count,
                "Notes newly detected as spent"
            );
        }
    }
    Ok((newly_spent, changeset))
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

/// One decrypted note discovered during a sync pass.
#[derive(Clone)]
struct DiscoveredNote {
    position: u64,
    cmx: [u8; 32],
    note: OrchardNote,
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
