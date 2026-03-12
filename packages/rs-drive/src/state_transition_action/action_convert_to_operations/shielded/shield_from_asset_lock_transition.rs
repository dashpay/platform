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
                    insert_notes(&mut ops, &v0.notes);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition_action::shielded::shield_from_asset_lock::v0::ShieldFromAssetLockTransitionActionV0;
    use crate::state_transition_action::shielded::ShieldedActionNote;
    use crate::util::batch::drive_op_batch::ShieldedPoolOperationType;
    use dpp::block::epoch::Epoch;
    use dpp::version::PlatformVersion;

    fn make_note() -> ShieldedActionNote {
        ShieldedActionNote {
            nullifier: [0x11; 32],
            cmx: [0x22; 32],
            encrypted_note: vec![1, 2, 3],
        }
    }

    fn make_action() -> ShieldFromAssetLockTransitionAction {
        ShieldFromAssetLockTransitionAction::V0(ShieldFromAssetLockTransitionActionV0 {
            asset_lock_outpoint: [0xDD; 36],
            asset_lock_value_to_be_consumed: 5000,
            signable_bytes_hasher: [0xEE; 32],
            shield_amount: 5000,
            notes: vec![make_note()],
            current_total_balance: 10000,
        })
    }

    #[test]
    fn test_produces_system_credits_asset_lock_notes_and_balance_ops() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // AddToSystemCredits + AddUsedAssetLock + InsertNote (1) + UpdateTotalBalance
        assert_eq!(ops.len(), 4);
    }

    #[test]
    fn test_first_op_adds_system_credits() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        match &ops[0] {
            DriveOperation::SystemOperation(SystemOperationType::AddToSystemCredits {
                amount,
            }) => {
                assert_eq!(*amount, 5000);
            }
            other => panic!("expected AddToSystemCredits, got {:?}", other),
        }
    }

    #[test]
    fn test_second_op_adds_used_asset_lock() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        assert!(matches!(
            &ops[1],
            DriveOperation::SystemOperation(SystemOperationType::AddUsedAssetLock { .. })
        ));
    }

    #[test]
    fn test_balance_increases_by_shield_amount() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        match ops.last().unwrap() {
            DriveOperation::ShieldedPoolOperation(ShieldedPoolOperationType::UpdateTotalBalance {
                new_total_balance,
            }) => {
                assert_eq!(*new_total_balance, 15000); // 10000 + 5000
            }
            other => panic!("expected UpdateTotalBalance, got {:?}", other),
        }
    }

    #[test]
    fn test_overflow_returns_error() {
        let action = ShieldFromAssetLockTransitionAction::V0(
            ShieldFromAssetLockTransitionActionV0 {
                asset_lock_outpoint: [0xDD; 36],
                asset_lock_value_to_be_consumed: 1000,
                signable_bytes_hasher: [0x00; 32],
                shield_amount: u64::MAX,
                notes: vec![],
                current_total_balance: 1,
            },
        );
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let result = action.into_high_level_drive_operations(&epoch, platform_version);
        assert!(result.is_err());
    }
}
