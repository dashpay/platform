//! Shielded note + nullifier synchronization (multi-account).
//!
//! Implements sync methods on `ShieldedWallet<S>`:
//! - `sync_notes()` — fetch encrypted notes once, trial-decrypt
//!   with every bound account's IVK, append commitments to the
//!   shared tree once with `marked = any account decrypted the
//!   position`, save decrypted notes per-subwallet.
//! - `check_nullifiers()` — privacy-preserving nullifier scan,
//!   marks spent notes per-subwallet.
//! - `sync()` — full pass: notes + nullifiers + per-account
//!   balance summary.

use std::collections::{BTreeMap, BTreeSet};

use dash_sdk::platform::shielded::nullifier_sync::{NullifierSyncCheckpoint, NullifierSyncConfig};
use dash_sdk::platform::shielded::{sync_shielded_notes, try_decrypt_note};
use grovedb_commitment_tree::{Note as OrchardNote, PaymentAddress, PreparedIncomingViewingKey};
use tracing::{debug, info, warn};

use super::store::{ShieldedStore, SubwalletId};
use super::ShieldedWallet;
use crate::error::PlatformWalletError;

/// Server-enforced chunk size — start_index must be a multiple of this.
const CHUNK_SIZE: u64 = 2048;

/// Result of one note-sync pass.
#[derive(Debug, Clone, Default)]
pub struct SyncNotesResult {
    /// Per-account count of new notes discovered in this pass.
    pub new_notes_per_account: BTreeMap<u32, usize>,
    /// Total encrypted notes scanned.
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

impl<S: ShieldedStore> ShieldedWallet<S> {
    /// Sync encrypted notes from Platform across every bound account.
    ///
    /// Fetches raw chunks once via the SDK (using account 0's IVK
    /// as the trial-decrypt key for the SDK call), then locally
    /// trial-decrypts the same chunks against every other
    /// account's IVK. Commitments are appended to the shared
    /// tree exactly once per global position with `marked =
    /// (any bound account owns this position)`. Decrypted notes
    /// land in the store under the discovering account's
    /// [`SubwalletId`].
    pub async fn sync_notes(&self) -> Result<SyncNotesResult, PlatformWalletError> {
        // Snapshot accounts + their prepared IVKs. The IVKs are
        // owned `PreparedIncomingViewingKey` values so we can hold
        // them across the await without borrowing `self`.
        let account_indices: Vec<u32> = self.account_indices();
        if account_indices.is_empty() {
            return Ok(SyncNotesResult::default());
        }
        let prepared: Vec<(u32, PreparedIncomingViewingKey)> = account_indices
            .iter()
            .map(|&a| Ok((a, self.keys_for(a)?.prepared_ivk())))
            .collect::<Result<_, PlatformWalletError>>()?;

        // Use the lowest per-account watermark as the canonical
        // tree-fetch start. Today we wipe-and-re-sync when an
        // account is added, so all accounts share the same
        // watermark in practice — this `min` is just defensive.
        let already_have = {
            let store = self.store.read().await;
            let mut min_idx: Option<u64> = None;
            for &account in &account_indices {
                let id = self.subwallet_id(account);
                let idx = store
                    .last_synced_note_index(id)
                    .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
                min_idx = Some(min_idx.map_or(idx, |m| m.min(idx)));
            }
            min_idx.unwrap_or(0)
        };
        let aligned_start = (already_have / CHUNK_SIZE) * CHUNK_SIZE;

        info!(
            accounts = account_indices.len(),
            already_have, aligned_start, "Starting shielded note sync"
        );

        // Fetch + trial-decrypt with the FIRST bound account's
        // IVK in one SDK call. We also reuse the returned
        // `all_notes` for local trial-decryption with every other
        // account's IVK below.
        let (driver_account, driver_ivk) = &prepared[0];
        let result = sync_shielded_notes(&self.sdk, driver_ivk, aligned_start, None)
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

        // Index decryptions by `(account, position) → DecryptedNote`.
        // The driver account's hits come from the SDK call;
        // every other account's are produced by local
        // trial-decryption against `result.all_notes`.
        let mut decrypted_by_account: BTreeMap<u32, Vec<DiscoveredNote>> = BTreeMap::new();
        for dn in &result.decrypted_notes {
            decrypted_by_account
                .entry(*driver_account)
                .or_default()
                .push(DiscoveredNote {
                    position: dn.position,
                    cmx: dn.cmx,
                    note: dn.note,
                });
        }

        for (account, ivk) in prepared.iter().skip(1) {
            for (i, raw_note) in result.all_notes.iter().enumerate() {
                let position = aligned_start + i as u64;
                if let Some((note, _addr)) = try_decrypt_note(ivk, raw_note) {
                    let cmx_bytes: [u8; 32] = match raw_note.cmx.as_slice().try_into() {
                        Ok(b) => b,
                        Err(_) => continue,
                    };
                    decrypted_by_account
                        .entry(*account)
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
        let owned_positions: BTreeSet<u64> = decrypted_by_account
            .values()
            .flat_map(|v| v.iter().map(|n| n.position))
            .collect();

        let mut store = self.store.write().await;

        // Append every commitment to the shared tree exactly
        // once per position. Skip positions already in the tree
        // (re-scan after a partial chunk advance).
        let mut appended = 0u32;
        for (i, raw_note) in result.all_notes.iter().enumerate() {
            let global_pos = aligned_start + i as u64;
            if global_pos < already_have {
                continue;
            }
            let cmx_bytes: [u8; 32] = raw_note.cmx.as_slice().try_into().map_err(|_| {
                PlatformWalletError::ShieldedSyncFailed("Invalid cmx length".into())
            })?;
            let is_ours = owned_positions.contains(&global_pos);
            store
                .append_commitment(&cmx_bytes, is_ours)
                .map_err(|e| PlatformWalletError::ShieldedTreeUpdateFailed(e.to_string()))?;
            appended += 1;
        }

        if appended > 0 {
            let checkpoint_id = result.next_start_index as u32;
            store
                .checkpoint_tree(checkpoint_id)
                .map_err(|e| PlatformWalletError::ShieldedTreeUpdateFailed(e.to_string()))?;
        }

        // Save decrypted notes scoped per subwallet, and
        // count new notes per account.
        let mut new_notes_per_account: BTreeMap<u32, usize> = BTreeMap::new();
        for (account, discovered) in &decrypted_by_account {
            let fvk = &self.keys_for(*account)?.full_viewing_key;
            let id = self.subwallet_id(*account);
            for d in discovered {
                if d.position < already_have {
                    continue;
                }
                let nullifier = d.note.nullifier(fvk);
                let value = d.note.value().inner();
                debug!(
                    account = account,
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
                    .save_note(id, &shielded_note)
                    .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
                *new_notes_per_account.entry(*account).or_default() += 1;
            }
        }

        // Update every account's watermark to the same global
        // tree position so the next sync resumes coherently.
        let new_index = aligned_start + result.total_notes_scanned;
        for &account in &account_indices {
            let id = self.subwallet_id(account);
            store
                .set_last_synced_note_index(id, new_index)
                .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
        }

        info!(
            new_notes_total = new_notes_per_account.values().sum::<usize>(),
            new_index, "Shielded sync finished"
        );

        Ok(SyncNotesResult {
            new_notes_per_account,
            total_scanned: result.total_notes_scanned,
        })
    }

    /// Check nullifier status for unspent notes across every bound
    /// account. Spent notes are marked per-subwallet.
    pub async fn check_nullifiers(&self) -> Result<BTreeMap<u32, usize>, PlatformWalletError> {
        let account_indices = self.account_indices();
        if account_indices.is_empty() {
            return Ok(BTreeMap::new());
        }

        // Aggregate unspent nullifiers across accounts so we hit
        // the SDK once, then route the `found` results back to
        // the right subwallet via a position lookup.
        struct AccountUnspent {
            id: SubwalletId,
            nullifiers: Vec<[u8; 32]>,
            checkpoint: Option<NullifierSyncCheckpoint>,
        }

        let per_account: Vec<(u32, AccountUnspent)> = {
            let store = self.store.read().await;
            let mut out = Vec::with_capacity(account_indices.len());
            for &account in &account_indices {
                let id = self.subwallet_id(account);
                let unspent = store
                    .get_unspent_notes(id)
                    .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
                let nullifiers: Vec<[u8; 32]> = unspent.iter().map(|n| n.nullifier).collect();
                let checkpoint = store
                    .nullifier_checkpoint(id)
                    .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?
                    .map(|(height, timestamp)| NullifierSyncCheckpoint { height, timestamp });
                out.push((
                    account,
                    AccountUnspent {
                        id,
                        nullifiers,
                        checkpoint,
                    },
                ));
            }
            out
        };

        let mut newly_spent: BTreeMap<u32, usize> = BTreeMap::new();
        for (
            account,
            AccountUnspent {
                id,
                nullifiers,
                checkpoint,
            },
        ) in per_account
        {
            if nullifiers.is_empty() {
                continue;
            }
            debug!(
                account,
                checking = nullifiers.len(),
                ?checkpoint,
                "Checking nullifiers"
            );
            let result = self
                .sdk
                .sync_nullifiers(&nullifiers, None::<NullifierSyncConfig>, checkpoint)
                .await
                .map_err(|e| PlatformWalletError::ShieldedNullifierSyncFailed(e.to_string()))?;

            let mut store = self.store.write().await;
            let mut spent_count = 0usize;
            for nf_bytes in &result.found {
                if store
                    .mark_spent(id, nf_bytes)
                    .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?
                {
                    spent_count += 1;
                }
            }
            store
                .set_nullifier_checkpoint(id, result.new_sync_height, result.new_sync_timestamp)
                .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;

            if spent_count > 0 {
                newly_spent.insert(account, spent_count);
                info!(account, spent_count, "Notes newly detected as spent");
            }
        }

        Ok(newly_spent)
    }

    /// Full sync: notes + nullifiers + per-account balance summary.
    pub async fn sync(&self) -> Result<ShieldedSyncSummary, PlatformWalletError> {
        let notes_result = self.sync_notes().await?;
        let newly_spent_per_account = self.check_nullifiers().await?;
        let balances = self.balances().await?;
        Ok(ShieldedSyncSummary {
            notes_result,
            newly_spent_per_account,
            balances,
        })
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
