//! Shielded note + nullifier synchronization.
//!
//! Phase-2b shape: the heavy lifting lives in three free
//! functions that take a flat `&[(SubwalletId, AccountViewingKeys)]`
//! slice and drive a single network-wide SDK fetch per pass:
//! - [`sync_notes_across`] — fetches encrypted notes once,
//!   trial-decrypts against the union of every subwallet's IVK,
//!   appends commitments to the shared tree exactly once per
//!   position with `marked = any subwallet decrypted it`, and
//!   saves decrypted notes scoped per-`SubwalletId`.
//! - [`check_nullifiers_across`] — privacy-preserving nullifier
//!   scan per subwallet (the SDK's nullifier-scan API is
//!   per-checkpoint, so it stays per-subwallet, but no longer
//!   per-`ShieldedWallet`).
//! - [`balances_across`] — pure unspent-balance read against
//!   the shared store.
//!
//! Per-wallet [`ShieldedWallet`] methods (`sync_notes`,
//! `check_nullifiers`, `balances`) remain as thin delegators
//! that build the one-wallet's slice and call the free
//! functions; the full-wallet `sync` orchestrator was removed
//! in Phase 4d.2 (the coordinator's `sync` is now the only
//! entry point that runs all three in sequence). Phase 4d.3
//! removes the remaining delegators along with `ShieldedWallet`
//! itself.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use dash_sdk::platform::shielded::nullifier_sync::{NullifierSyncCheckpoint, NullifierSyncConfig};
use dash_sdk::platform::shielded::{sync_shielded_notes, try_decrypt_note};
use grovedb_commitment_tree::{Note as OrchardNote, PaymentAddress};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::keys::AccountViewingKeys;
use super::store::{ShieldedStore, SubwalletId};
use super::ShieldedWallet;
use crate::changeset::ShieldedChangeSet;
use crate::error::PlatformWalletError;

/// Server-enforced chunk size — start_index must be a multiple of this.
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
    /// New positions observed this pass — `(aligned_start +
    /// total_notes_scanned).saturating_sub(already_have)`. See
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
) -> Result<MultiSyncNotesResult, PlatformWalletError> {
    if subwallets.is_empty() {
        return Ok(MultiSyncNotesResult::default());
    }

    // Snapshot the lowest per-subwallet watermark — the canonical
    // tree-fetch start. Defensive `min` across subwallets: in
    // practice we wipe-and-re-sync on account add so every
    // subwallet shares the same watermark.
    let already_have = {
        let store = store.read().await;
        let mut min_idx: Option<u64> = None;
        for (id, _) in subwallets {
            let idx = store
                .last_synced_note_index(*id)
                .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
            min_idx = Some(min_idx.map_or(idx, |m| m.min(idx)));
        }
        min_idx.unwrap_or(0)
    };
    let aligned_start = (already_have / CHUNK_SIZE) * CHUNK_SIZE;

    info!(
        subwallets = subwallets.len(),
        already_have, aligned_start, "Starting multi-subwallet shielded note sync"
    );

    // Fetch + trial-decrypt with the FIRST subwallet's IVK in
    // one SDK call. The driver's hits come back as
    // `result.decrypted_notes`; every other subwallet's are
    // produced by local trial-decryption against
    // `result.all_notes` below.
    let (driver_id, driver_views) = &subwallets[0];
    let driver_ivk = driver_views.prepared_ivk.clone();
    let result = sync_shielded_notes(sdk, &driver_ivk, aligned_start, None)
        .await
        .map_err(|e| PlatformWalletError::ShieldedSyncFailed(e.to_string()))?;

    info!(
        total_scanned = result.total_notes_scanned,
        decrypted_for_driver = result.decrypted_notes.len(),
        next_start_index = result.next_start_index,
        "SDK sync returned"
    );

    if result.next_start_index == 0 && result.total_notes_scanned > 0 {
        warn!(
            "Shielded sync: next_start_index is 0 after scanning {} notes — \
             next sync will rescan from the beginning",
            result.total_notes_scanned,
        );
    }

    // Route decryptions to the subwallet that owns the IVK.
    let mut decrypted_by_subwallet: BTreeMap<SubwalletId, Vec<DiscoveredNote>> = BTreeMap::new();
    for dn in &result.decrypted_notes {
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
        for (i, raw_note) in result.all_notes.iter().enumerate() {
            let position = aligned_start + i as u64;
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

    // Build the union of "owned" positions for tree marking.
    let owned_positions: BTreeSet<u64> = decrypted_by_subwallet
        .values()
        .flat_map(|v| v.iter().map(|n| n.position))
        .collect();

    let mut store = store.write().await;

    // Append every commitment to the shared tree exactly once
    // per position. Skip positions already in the tree (re-scan
    // after a partial chunk advance).
    let mut appended = 0u32;
    for (i, raw_note) in result.all_notes.iter().enumerate() {
        let global_pos = aligned_start + i as u64;
        if global_pos < already_have {
            continue;
        }
        let cmx_bytes: [u8; 32] =
            raw_note.cmx.as_slice().try_into().map_err(|_| {
                PlatformWalletError::ShieldedSyncFailed("Invalid cmx length".into())
            })?;
        let is_ours = owned_positions.contains(&global_pos);
        store
            .append_commitment(&cmx_bytes, is_ours)
            .map_err(|e| PlatformWalletError::ShieldedTreeUpdateFailed(e.to_string()))?;
        appended += 1;
    }

    if appended > 0 {
        // Use the high-water position as the checkpoint id (not
        // `result.next_start_index`, which rewinds to the last
        // partial chunk's start and can therefore be the same
        // value across consecutive syncs). shardtree's
        // `checkpoint(id)` silently dedups duplicate ids; a
        // non-monotonic id leaves depth-0 pinned at the first
        // checkpoint while later appends extend the tree past
        // it, and the witness at depth 0 then reflects a state
        // Platform never recorded.
        let new_index = aligned_start + result.total_notes_scanned;
        let checkpoint_id: u32 = new_index.try_into().unwrap_or(u32::MAX);
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
        for d in discovered {
            if d.position < already_have {
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
                block_height: result.block_height,
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
    // the union.
    let new_index = aligned_start + result.total_notes_scanned;
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

    let scanned_new = (aligned_start + result.total_notes_scanned).saturating_sub(already_have);
    Ok(MultiSyncNotesResult {
        per_subwallet_new_notes,
        total_scanned: scanned_new,
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
pub(super) async fn balances_across<S: ShieldedStore>(
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

impl<S: ShieldedStore> ShieldedWallet<S> {
    /// Sync encrypted notes from Platform across every bound
    /// account of this single wallet.
    ///
    /// Delegates to [`sync_notes_across`] under the hood with
    /// the wallet's own subwallets — Phase 2b kept this surface
    /// for backward-compat with existing per-wallet callers
    /// (tests, the fallback path in `ShieldedSyncManager` when no
    /// coordinator has been configured). Phase 4 removes this
    /// method along with `ShieldedWallet` itself; the coordinator
    /// becomes the only sync entry point.
    pub async fn sync_notes(&self) -> Result<SyncNotesResult, PlatformWalletError> {
        let account_indices = self.account_indices();
        if account_indices.is_empty() {
            return Ok(SyncNotesResult::default());
        }
        let subwallets: Vec<(SubwalletId, AccountViewingKeys)> = account_indices
            .iter()
            .map(|&account| {
                let id = self.subwallet_id(account);
                let views = self.keys_for(account)?.viewing_keys();
                Ok::<_, PlatformWalletError>((id, views))
            })
            .collect::<Result<_, _>>()?;

        let result = sync_notes_across(&self.sdk, &self.store, &subwallets).await?;
        // Fold per-subwallet counts back into per-account for
        // this wallet, and queue the consolidated changeset on
        // the wallet's own persister.
        let new_notes_per_account = result.per_account_for(self.wallet_id);
        self.queue_shielded_changeset(result.changeset);
        Ok(SyncNotesResult {
            new_notes_per_account,
            total_scanned: result.total_scanned,
        })
    }

    /// Check nullifier status for unspent notes across every
    /// bound account on this single wallet. Spent notes are
    /// marked per-subwallet.
    ///
    /// Delegates to [`check_nullifiers_across`]; see
    /// [`sync_notes`](Self::sync_notes) for the Phase 4 deletion
    /// plan that removes this method along with `ShieldedWallet`.
    pub async fn check_nullifiers(&self) -> Result<BTreeMap<u32, usize>, PlatformWalletError> {
        let account_indices = self.account_indices();
        if account_indices.is_empty() {
            return Ok(BTreeMap::new());
        }
        let subwallets: Vec<(SubwalletId, AccountViewingKeys)> = account_indices
            .iter()
            .map(|&account| {
                let id = self.subwallet_id(account);
                let views = self.keys_for(account)?.viewing_keys();
                Ok::<_, PlatformWalletError>((id, views))
            })
            .collect::<Result<_, _>>()?;

        let (per_sub, changeset) =
            check_nullifiers_across(&self.sdk, &self.store, &subwallets).await?;
        self.queue_shielded_changeset(changeset);
        // Fold per-subwallet counts back into per-account for
        // this wallet's caller shape.
        Ok(per_sub
            .into_iter()
            .filter(|(id, _)| id.wallet_id == self.wallet_id)
            .map(|(id, c)| (id.account_index, c))
            .collect())
    }
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
