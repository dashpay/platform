use super::{insert_notes, insert_nullifiers, store_nullifiers_for_block, update_balance};
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

                    // 1. Insert each nullifier (known to not exist after validation)
                    insert_nullifiers(&mut ops, &v0.notes);

                    // 2. Credit the output address with the unshielded amount
                    ops.push(DriveOperation::AddressFundsOperation(
                        AddressFundsOperationType::AddBalanceToAddress {
                            address: v0.output_address,
                            balance_to_add: v0.amount,
                        },
                    ));

                    // 3. Insert notes into CommitmentTree (change outputs)
                    insert_notes(&mut ops, &v0.notes);

                    // 4. Update total balance
                    // Pool decreases by amount (to output address) + fee_amount (to proposers)
                    let total_deduction =
                        v0.amount.checked_add(v0.fee_amount).ok_or_else(|| {
                            Error::Drive(DriveError::CorruptedDriveState(
                                "overflow when adding unshield amount and fee".to_string(),
                            ))
                        })?;
                    let new_total_balance =
                        v0.current_total_balance
                            .checked_sub(total_deduction)
                            .ok_or_else(|| {
                                Error::Drive(DriveError::CorruptedDriveState(
                                "shielded pool total balance underflow when subtracting unshield amount and fee"
                                    .to_string(),
                            ))
                            })?;
                    update_balance(&mut ops, new_total_balance);

                    // 5. Store nullifiers to recent block storage for catch-up sync
                    store_nullifiers_for_block(&mut ops, &v0.notes);

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
