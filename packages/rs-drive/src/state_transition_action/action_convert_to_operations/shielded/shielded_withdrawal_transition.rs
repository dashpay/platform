use super::{
    append_note_commitments, insert_encrypted_notes, insert_nullifiers,
    update_balance_and_record_anchor,
};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::shielded::shielded_withdrawal::ShieldedWithdrawalTransitionAction;
use crate::util::batch::drive_op_batch::SystemOperationType;
use crate::util::batch::{DocumentOperationType, DriveOperation};
use crate::util::object_size_info::{DocumentInfo, OwnedDocumentInfo};
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for ShieldedWithdrawalTransitionAction {
    fn into_high_level_drive_operations<'a>(
        self,
        _epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        match platform_version
            .drive
            .methods
            .state_transitions
            .convert_to_high_level_operations
            .shielded_withdrawal_transition
        {
            0 => match self {
                ShieldedWithdrawalTransitionAction::V0(v0) => {
                    let mut ops: Vec<DriveOperation<'a>> = Vec::new();

                    // 1. Insert nullifiers (prevent double-spend)
                    insert_nullifiers(&mut ops, &v0.nullifiers);

                    // 2. Append change note commitments to commitment tree
                    append_note_commitments(&mut ops, &v0.note_commitments);

                    // 3. Insert encrypted change notes with auto-incremented keys in count tree
                    insert_encrypted_notes(&mut ops, &v0.note_commitments, &v0.encrypted_notes);

                    // 4. Update total balance: subtract withdrawal amount
                    let new_total_balance =
                        v0.current_total_balance
                            .checked_sub(v0.amount)
                            .ok_or_else(|| {
                                Error::Drive(DriveError::CorruptedDriveState(
                                "shielded pool total balance underflow when subtracting shielded_withdrawal amount"
                                    .to_string(),
                            ))
                            })?;
                    update_balance_and_record_anchor(&mut ops, new_total_balance);

                    // 5. Add withdrawal document
                    ops.push(DriveOperation::DocumentOperation(
                        DocumentOperationType::AddWithdrawalDocument {
                            owned_document_info: OwnedDocumentInfo {
                                document_info: DocumentInfo::DocumentOwnedInfo((
                                    v0.prepared_withdrawal_document,
                                    None,
                                )),
                                owner_id: None,
                            },
                        },
                    ));

                    // 6. Remove credits from system (they leave the system to Core)
                    ops.push(DriveOperation::SystemOperation(
                        SystemOperationType::RemoveFromSystemCredits { amount: v0.amount },
                    ));

                    Ok(ops)
                }
            },
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "ShieldedWithdrawalTransitionAction::into_high_level_drive_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
