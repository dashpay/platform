//! Shielded note and nullifier synchronization.
//!
//! Implements the sync methods on `ShieldedWallet<S>`:
//! - `sync_notes()` -- fetch and trial-decrypt encrypted notes from Platform
//! - `check_nullifiers()` -- privacy-preserving nullifier status check
//! - `sync()` -- full sync (notes + nullifiers + balance)

use super::store::ShieldedStore;
use super::ShieldedWallet;
use crate::error::PlatformWalletError;

use dash_sdk::platform::shielded::nullifier_sync::{
    NullifierSyncCheckpoint, NullifierSyncConfig,
};
use dash_sdk::platform::shielded::sync_shielded_notes;
use tracing::{debug, info, warn};

/// Server-enforced chunk size -- start_index must be a multiple of this.
const CHUNK_SIZE: u64 = 2048;

/// Result of a note sync operation.
#[derive(Debug, Clone)]
pub struct SyncNotesResult {
    /// Number of new notes found (decrypted for this wallet).
    pub new_notes: usize,
    /// Total encrypted notes scanned in this sync.
    pub total_scanned: u64,
}

/// Summary of a full sync (notes + nullifiers + balance).
#[derive(Debug, Clone)]
pub struct ShieldedSyncSummary {
    /// Results from note sync.
    pub notes_result: SyncNotesResult,
    /// Number of notes newly detected as spent.
    pub newly_spent: usize,
    /// Current unspent balance after sync.
    pub balance: u64,
}

impl<S: ShieldedStore> ShieldedWallet<S> {
    /// Sync encrypted notes from Platform.
    ///
    /// Performs the following steps:
    /// 1. Read `last_synced_note_index` from store and align to chunk boundary
    /// 2. Fetch and trial-decrypt all new encrypted notes via SDK
    /// 3. Append each note's commitment to the store's tree (marked if decrypted)
    /// 4. Checkpoint the commitment tree
    /// 5. Save each decrypted note to store
    /// 6. Update `last_synced_note_index`
    ///
    /// # Returns
    ///
    /// `SyncNotesResult` with the count of new notes found and total scanned.
    pub async fn sync_notes(&self) -> Result<SyncNotesResult, PlatformWalletError> {
        let prepared_ivk = self.keys.prepared_ivk();

        // Step 1: Get last synced index and align to chunk boundary
        let already_have = {
            let store = self.store.read().await;
            store
                .last_synced_note_index()
                .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?
        };
        let aligned_start = (already_have / CHUNK_SIZE) * CHUNK_SIZE;

        info!(
            "Starting shielded note sync: last_synced={}, aligned_start={}",
            already_have, aligned_start,
        );

        // Step 2: Fetch and trial-decrypt via SDK
        let result = sync_shielded_notes(&self.sdk, &prepared_ivk, aligned_start, None)
            .await
            .map_err(|e| PlatformWalletError::ShieldedSyncFailed(e.to_string()))?;

        info!(
            "Sync complete: total_scanned={}, decrypted={}, next_start_index={}",
            result.total_notes_scanned,
            result.decrypted_notes.len(),
            result.next_start_index,
        );

        if result.next_start_index == 0 && result.total_notes_scanned > 0 {
            warn!(
                "Shielded sync: next_start_index is 0 after scanning {} notes -- \
                 next sync will rescan everything from the beginning",
                result.total_notes_scanned,
            );
        }

        let mut store = self.store.write().await;

        // Step 3: Append commitments to the tree, skipping positions already present
        let mut appended = 0u32;
        for (i, raw_note) in result.all_notes.iter().enumerate() {
            let global_pos = aligned_start + i as u64;
            if global_pos < already_have {
                continue; // already appended in a previous sync
            }

            let cmx_bytes: [u8; 32] = raw_note
                .cmx
                .as_slice()
                .try_into()
                .map_err(|_| PlatformWalletError::ShieldedSyncFailed("Invalid cmx length".into()))?;

            let is_ours = result
                .decrypted_notes
                .iter()
                .any(|dn| dn.position == global_pos);

            store
                .append_commitment(&cmx_bytes, is_ours)
                .map_err(|e| PlatformWalletError::ShieldedTreeUpdateFailed(e.to_string()))?;

            appended += 1;
        }

        // Step 4: Checkpoint tree
        if appended > 0 {
            let checkpoint_id = result.next_start_index as u32;
            store
                .checkpoint_tree(checkpoint_id)
                .map_err(|e| PlatformWalletError::ShieldedTreeUpdateFailed(e.to_string()))?;
        }

        // Step 5: Save decrypted notes
        let mut new_note_count = 0usize;
        for dn in &result.decrypted_notes {
            if dn.position < already_have {
                continue; // already stored in a previous sync
            }

            // Compute the spending nullifier from our FVK.
            // dn.nullifier is the rho/nf from the compact action, not the spending nullifier.
            let nullifier = dn.note.nullifier(&self.keys.full_viewing_key);
            let value = dn.note.value().inner();

            debug!(
                "Note[{}]: DECRYPTED, value={} credits",
                dn.position, value,
            );

            // Serialize the note for storage.
            let note_data = serialize_note(&dn.note);

            let shielded_note = super::store::ShieldedNote {
                note_data,
                position: dn.position,
                cmx: dn.cmx,
                nullifier: nullifier.to_bytes(),
                block_height: result.block_height,
                is_spent: false,
                value,
            };

            store
                .save_note(&shielded_note)
                .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;

            new_note_count += 1;
        }

        // Step 6: Update last synced index
        let new_index = aligned_start + result.total_notes_scanned;
        store
            .set_last_synced_note_index(new_index)
            .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;

        info!(
            "Shielded sync finished: {} new note(s), last_synced_index={}",
            new_note_count, new_index,
        );

        Ok(SyncNotesResult {
            new_notes: new_note_count,
            total_scanned: result.total_notes_scanned,
        })
    }

    /// Check nullifier status for unspent notes.
    ///
    /// Uses the SDK's privacy-preserving trunk/branch tree scan with incremental
    /// catch-up. Marks spent notes in the store.
    ///
    /// # Returns
    ///
    /// The number of notes newly detected as spent.
    pub async fn check_nullifiers(&self) -> Result<usize, PlatformWalletError> {
        // Step 1: Collect unspent nullifiers from store
        let (unspent_nullifiers, last_checkpoint) = {
            let store = self.store.read().await;
            let unspent = store
                .get_unspent_notes()
                .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
            let nullifiers: Vec<[u8; 32]> = unspent.iter().map(|n| n.nullifier).collect();
            let checkpoint = store
                .nullifier_checkpoint()
                .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?
                .map(|(height, timestamp)| NullifierSyncCheckpoint { height, timestamp });
            (nullifiers, checkpoint)
        };

        if unspent_nullifiers.is_empty() {
            return Ok(0);
        }

        debug!(
            "Checking {} nullifiers (checkpoint: {:?})",
            unspent_nullifiers.len(),
            last_checkpoint,
        );

        // Step 2: Call SDK sync_nullifiers
        let result = self
            .sdk
            .sync_nullifiers(&unspent_nullifiers, None::<NullifierSyncConfig>, last_checkpoint)
            .await
            .map_err(|e| PlatformWalletError::ShieldedNullifierSyncFailed(e.to_string()))?;

        // Step 3: Mark found (spent) nullifiers in store
        let mut store = self.store.write().await;

        let mut spent_count = 0usize;
        for nf_bytes in &result.found {
            let was_unspent = store
                .mark_spent(nf_bytes)
                .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;
            if was_unspent {
                spent_count += 1;
            }
        }

        // Step 4: Update nullifier checkpoint
        store
            .set_nullifier_checkpoint(result.new_sync_height, result.new_sync_timestamp)
            .map_err(|e| PlatformWalletError::ShieldedStoreError(e.to_string()))?;

        if spent_count > 0 {
            info!("{} note(s) newly detected as spent", spent_count);
        }

        Ok(spent_count)
    }

    /// Full sync: notes + nullifiers + balance.
    ///
    /// Performs note sync first to discover new notes, then checks nullifiers
    /// to detect spent notes, and finally computes the current balance.
    pub async fn sync(&self) -> Result<ShieldedSyncSummary, PlatformWalletError> {
        // Sync notes first
        let notes_result = self.sync_notes().await?;

        // Then check nullifiers
        let newly_spent = self.check_nullifiers().await?;

        // Compute balance
        let balance = self.balance().await?;

        Ok(ShieldedSyncSummary {
            notes_result,
            newly_spent,
            balance,
        })
    }
}

/// Serialize an Orchard note to bytes for storage.
///
/// Format: `recipient(43) || value(8 LE) || rho(32) || rseed(32)` = 115 bytes.
///
/// Must be kept in sync with `deserialize_note()` in operations.rs.
fn serialize_note(note: &grovedb_commitment_tree::Note) -> Vec<u8> {
    let mut data = Vec::with_capacity(115);
    data.extend_from_slice(&note.recipient().to_raw_address_bytes());
    data.extend_from_slice(&note.value().inner().to_le_bytes());
    data.extend_from_slice(&note.rho().to_bytes());
    data.extend_from_slice(note.rseed().as_bytes());
    data
}
