mod shield_from_asset_lock_transition;
mod shield_transition;
mod shielded_transfer_transition;
mod shielded_withdrawal_transition;
mod unshield_transition;

use crate::state_transition_action::shielded::ShieldedActionNote;
use crate::util::batch::drive_op_batch::ShieldedPoolOperationType;
use crate::util::batch::DriveOperation;

/// Insert each nullifier (InsertOnly to prevent double-spend).
pub(super) fn insert_nullifiers<'a>(
    ops: &mut Vec<DriveOperation<'a>>,
    notes: &[ShieldedActionNote],
) {
    for note in notes {
        ops.push(DriveOperation::ShieldedPoolOperation(
            ShieldedPoolOperationType::InsertNullifier {
                nullifier: note.nullifier,
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
    notes: &[ShieldedActionNote],
) {
    for note in notes {
        ops.push(DriveOperation::ShieldedPoolOperation(
            ShieldedPoolOperationType::InsertNote {
                nullifier: note.nullifier,
                cmx: note.cmx,
                encrypted_note: note.encrypted_note.clone(),
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
    notes: &[ShieldedActionNote],
) {
    if !notes.is_empty() {
        ops.push(DriveOperation::ShieldedPoolOperation(
            ShieldedPoolOperationType::StoreNullifiersForBlock {
                nullifiers: notes.iter().map(|n| n.nullifier).collect(),
            },
        ));
    }
}
