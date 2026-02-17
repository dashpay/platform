use super::{insert_notes, insert_nullifiers, update_balance};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::shielded::shielded_transfer::ShieldedTransferTransitionAction;
use crate::util::batch::DriveOperation;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for ShieldedTransferTransitionAction {
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
            .shielded_transfer_transition
        {
            0 => match self {
                ShieldedTransferTransitionAction::V0(v0) => {
                    let mut ops: Vec<DriveOperation<'a>> = Vec::new();

                    // 1. Insert each nullifier (InsertOnly to prevent double-spend)
                    insert_nullifiers(&mut ops, &v0.nullifiers);

                    // 2. Insert notes into CommitmentTree
                    insert_notes(
                        &mut ops,
                        &v0.nullifiers,
                        &v0.note_commitments,
                        &v0.encrypted_notes,
                    );

                    // 3. Update total balance (pool decreases by fee_amount)
                    let new_total_balance = v0
                        .current_total_balance
                        .checked_sub(v0.fee_amount)
                        .ok_or_else(|| {
                            Error::Drive(DriveError::CorruptedDriveState(
                                "shielded pool total balance underflow when subtracting fee_amount"
                                    .to_string(),
                            ))
                        })?;
                    update_balance(&mut ops, new_total_balance);

                    Ok(ops)
                }
            },
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "ShieldedTransferTransitionAction::into_high_level_drive_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
