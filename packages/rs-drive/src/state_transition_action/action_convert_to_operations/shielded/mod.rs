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

/// Insert notes into the CommitmentTree (appends cmx to frontier + stores cmx||encrypted_note).
pub(super) fn insert_notes<'a>(
    ops: &mut Vec<DriveOperation<'a>>,
    note_commitments: &[[u8; 32]],
    encrypted_notes: &[Vec<u8>],
) {
    for (cmx, encrypted_note) in note_commitments.iter().zip(encrypted_notes.iter()) {
        ops.push(DriveOperation::ShieldedPoolOperation(
            ShieldedPoolOperationType::InsertNote {
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
