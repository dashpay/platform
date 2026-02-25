mod shield_from_asset_lock_transition;
mod shield_transition;
mod shielded_transfer_transition;
mod shielded_withdrawal_transition;
mod unshield_transition;

use crate::util::batch::drive_op_batch::ShieldedPoolOperationType;
use crate::util::batch::DriveOperation;

/// Insert each nullifier (InsertOnly to prevent double-spend).
pub(super) fn insert_nullifiers<'a>(ops: &mut Vec<DriveOperation<'a>>, nullifiers: &[[u8; 32]]) {
    for nullifier in nullifiers.iter() {
        ops.push(DriveOperation::ShieldedPoolOperation(
            ShieldedPoolOperationType::InsertNullifier {
                nullifier: *nullifier,
            },
        ));
    }
}

/// Insert notes into the CommitmentTree (appends cmx to frontier + stores nullifier||cmx||encrypted_note).
///
/// Each action's nullifier is stored alongside the note so light clients can derive
/// Rho for trial decryption.
pub(super) fn insert_notes<'a>(
    ops: &mut Vec<DriveOperation<'a>>,
    nullifiers: &[[u8; 32]],
    note_commitments: &[[u8; 32]],
    encrypted_notes: &[Vec<u8>],
) {
    for ((nullifier, cmx), encrypted_note) in nullifiers
        .iter()
        .zip(note_commitments.iter())
        .zip(encrypted_notes.iter())
    {
        ops.push(DriveOperation::ShieldedPoolOperation(
            ShieldedPoolOperationType::InsertNote {
                nullifier: *nullifier,
                cmx: *cmx,
                encrypted_note: encrypted_note.clone(),
            },
        ));
    }
}

/// Update pool total balance.
pub(super) fn update_balance<'a>(ops: &mut Vec<DriveOperation<'a>>, new_total_balance: u64) {
    ops.push(DriveOperation::ShieldedPoolOperation(
        ShieldedPoolOperationType::UpdateTotalBalance { new_total_balance },
    ));
}

/// Store nullifiers to recent block storage for catch-up sync RPCs.
pub(super) fn store_nullifiers_for_block<'a>(
    ops: &mut Vec<DriveOperation<'a>>,
    nullifiers: &[[u8; 32]],
) {
    if !nullifiers.is_empty() {
        ops.push(DriveOperation::ShieldedPoolOperation(
            ShieldedPoolOperationType::StoreNullifiersForBlock {
                nullifiers: nullifiers.to_vec(),
            },
        ));
    }
}
