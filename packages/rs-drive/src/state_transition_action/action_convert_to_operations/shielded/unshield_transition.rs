use super::{
    append_note_commitments, insert_encrypted_notes, insert_nullifiers,
    update_balance_and_record_anchor,
};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::shielded::unshield::UnshieldTransitionAction;
use crate::util::batch::drive_op_batch::AddressFundsOperationType;
use crate::util::batch::DriveOperation;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for UnshieldTransitionAction {
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
            .unshield_transition
        {
            0 => match self {
                UnshieldTransitionAction::V0(v0) => {
                    let mut ops: Vec<DriveOperation<'a>> = Vec::new();

                    // 1. Insert each nullifier (InsertOnly to prevent double-spend)
                    insert_nullifiers(&mut ops, &v0.nullifiers);

                    // 2. Credit the output address with the unshielded amount
                    ops.push(DriveOperation::AddressFundsOperation(
                        AddressFundsOperationType::AddBalanceToAddress {
                            address: v0.output_address,
                            balance_to_add: v0.amount,
                        },
                    ));

                    // 3. Append each note commitment (change outputs) to the commitment tree
                    append_note_commitments(&mut ops, &v0.note_commitments);

                    // 4. Insert encrypted notes with auto-incremented keys in count tree
                    insert_encrypted_notes(&mut ops, &v0.note_commitments, &v0.encrypted_notes);

                    // 5. Update total balance and record anchor
                    let new_total_balance =
                        v0.current_total_balance
                            .checked_sub(v0.amount)
                            .ok_or_else(|| {
                                Error::Drive(DriveError::CorruptedDriveState(
                                "shielded pool total balance underflow when subtracting unshield amount"
                                    .to_string(),
                            ))
                            })?;
                    update_balance_and_record_anchor(&mut ops, new_total_balance);

                    Ok(ops)
                }
            },
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "UnshieldTransitionAction::into_high_level_drive_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
