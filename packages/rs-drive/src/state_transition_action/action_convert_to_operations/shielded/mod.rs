mod shield_from_asset_lock_transition;
mod shield_transition;
mod shielded_transfer_transition;
mod shielded_withdrawal_transition;
mod unshield_transition;

use crate::util::batch::drive_op_batch::finalize_task::DriveOperationFinalizeTask;
use crate::util::batch::drive_op_batch::ShieldedPoolOperationType;
use crate::util::batch::DriveOperation;
use grovedb::Element;

/// Build the encrypted note items (cmx || encrypted_note) for auto-incremented insertion.
fn build_encrypted_note_items(
    note_commitments: &[[u8; 32]],
    encrypted_notes: &[Vec<u8>],
) -> Vec<Element> {
    note_commitments
        .iter()
        .zip(encrypted_notes.iter())
        .map(|(cmx, encrypted_note)| {
            let mut value = cmx.to_vec();
            value.extend_from_slice(encrypted_note);
            Element::new_item(value)
        })
        .collect()
}

/// Append each note commitment to the commitment tree.
pub(super) fn append_note_commitments<'a>(
    ops: &mut Vec<DriveOperation<'a>>,
    note_commitments: &[[u8; 32]],
) {
    for cmx in note_commitments.iter() {
        ops.push(DriveOperation::ShieldedPoolOperation(
            ShieldedPoolOperationType::AppendNoteCommitment { cmx: *cmx },
        ));
    }
}

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

/// Insert encrypted notes with auto-incremented keys in count tree.
pub(super) fn insert_encrypted_notes<'a>(
    ops: &mut Vec<DriveOperation<'a>>,
    note_commitments: &[[u8; 32]],
    encrypted_notes: &[Vec<u8>],
) {
    ops.push(DriveOperation::ShieldedPoolOperation(
        ShieldedPoolOperationType::InsertEncryptedNotes {
            items: build_encrypted_note_items(note_commitments, encrypted_notes),
        },
    ));
}

/// Update pool total balance and record shielded anchor.
pub(super) fn update_balance_and_record_anchor<'a>(
    ops: &mut Vec<DriveOperation<'a>>,
    new_total_balance: u64,
) {
    ops.push(DriveOperation::ShieldedPoolOperation(
        ShieldedPoolOperationType::UpdateTotalBalance { new_total_balance },
    ));
    ops.push(DriveOperation::FinalizeOperation(
        DriveOperationFinalizeTask::RecordShieldedAnchor,
    ));
}
