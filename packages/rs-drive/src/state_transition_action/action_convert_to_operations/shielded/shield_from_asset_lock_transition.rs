use super::{insert_notes, update_balance};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::shielded::shield_from_asset_lock::ShieldFromAssetLockTransitionAction;
use crate::util::batch::drive_op_batch::SystemOperationType;
use crate::util::batch::DriveOperation;
use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for ShieldFromAssetLockTransitionAction {
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
            .shield_from_asset_lock_transition
        {
            0 => match self {
                ShieldFromAssetLockTransitionAction::V0(v0) => {
                    let mut ops: Vec<DriveOperation<'a>> = Vec::new();

                    // 1. Add credits to system from the asset lock
                    ops.push(DriveOperation::SystemOperation(
                        SystemOperationType::AddToSystemCredits {
                            amount: v0.asset_lock_value_to_be_consumed,
                        },
                    ));

                    // 2. Record asset lock as consumed (prevent replay)
                    let asset_lock_value = AssetLockValue::new(
                        v0.asset_lock_value_to_be_consumed,
                        vec![], // tx_out_script not needed for shielded
                        0,      // remaining_credit_value = 0 (fully consumed)
                        vec![], // no used tags for shielded
                        platform_version,
                    )
                    .map_err(|e| Error::Protocol(Box::new(e)))?;
                    ops.push(DriveOperation::SystemOperation(
                        SystemOperationType::AddUsedAssetLock {
                            asset_lock_outpoint: v0.asset_lock_outpoint.into(),
                            asset_lock_value,
                        },
                    ));

                    // 3. Insert notes into CommitmentTree
                    insert_notes(
                        &mut ops,
                        &v0.nullifiers,
                        &v0.note_commitments,
                        &v0.encrypted_notes,
                    );

                    // 4. Update total balance
                    let new_total_balance =
                        v0.current_total_balance
                            .checked_add(v0.shield_amount)
                            .ok_or_else(|| {
                                Error::Drive(DriveError::CorruptedDriveState(
                                "shielded pool total balance overflow when adding shield_from_asset_lock amount"
                                    .to_string(),
                            ))
                            })?;
                    update_balance(&mut ops, new_total_balance);

                    Ok(ops)
                }
            },
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "ShieldFromAssetLockTransitionAction::into_high_level_drive_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
