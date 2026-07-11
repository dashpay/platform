use super::{insert_notes, update_balance};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::shielded::shield_from_asset_lock::ShieldFromAssetLockTransitionAction;
use crate::util::batch::drive_op_batch::{AddressFundsOperationType, SystemOperationType};
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

                    // 1. Add the FULL consumed asset-lock value to system credits. It is
                    //    distributed below: `shield_amount` -> shielded pool, `surplus_amount` ->
                    //    `surplus_output` address (when set), and the remainder -> fee pools
                    //    (computed by the execution event as consumed - shield_amount - surplus).
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

                    // 3. Route the surplus to the optional platform-address output. When
                    //    `surplus_output` is `None`, `surplus_amount` is 0 and the surplus is
                    //    instead folded into the fee pools by the execution event. Conservation:
                    //    AddToSystemCredits(consumed) == shield_amount (pool) + surplus_amount
                    //    (address) + fee (pools).
                    if let Some(surplus_address) = v0.surplus_output {
                        if v0.surplus_amount > 0 {
                            ops.push(DriveOperation::AddressFundsOperation(
                                AddressFundsOperationType::AddBalanceToAddress {
                                    address: surplus_address,
                                    balance_to_add: v0.surplus_amount,
                                },
                            ));
                        }
                    }

                    // 4. Insert notes into CommitmentTree
                    insert_notes(&mut ops, &v0.notes);

                    // 5. Update total balance
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
            cv_net: [0x22; 32],
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
            surplus_output: None,
            surplus_amount: 0,
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
            DriveOperation::SystemOperation(SystemOperationType::AddToSystemCredits { amount }) => {
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
            DriveOperation::ShieldedPoolOperation(
                ShieldedPoolOperationType::UpdateTotalBalance { new_total_balance },
            ) => {
                assert_eq!(*new_total_balance, 15000); // 10000 + 5000
            }
            other => panic!("expected UpdateTotalBalance, got {:?}", other),
        }
    }

    #[test]
    fn test_overflow_returns_error() {
        let action =
            ShieldFromAssetLockTransitionAction::V0(ShieldFromAssetLockTransitionActionV0 {
                asset_lock_outpoint: [0xDD; 36],
                asset_lock_value_to_be_consumed: 1000,
                signable_bytes_hasher: [0x00; 32],
                shield_amount: u64::MAX,
                notes: vec![],
                current_total_balance: 1,
                surplus_output: None,
                surplus_amount: 0,
            });
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let result = action.into_high_level_drive_operations(&epoch, platform_version);
        assert!(result.is_err());
    }

    #[test]
    fn test_routes_surplus_to_address() {
        use dpp::address_funds::PlatformAddress;
        let surplus_addr = PlatformAddress::P2pkh([0x42; 20]);
        let action =
            ShieldFromAssetLockTransitionAction::V0(ShieldFromAssetLockTransitionActionV0 {
                asset_lock_outpoint: [0xDD; 36],
                asset_lock_value_to_be_consumed: 10_000,
                signable_bytes_hasher: [0xEE; 32],
                shield_amount: 5_000,
                notes: vec![make_note()],
                current_total_balance: 10_000,
                surplus_output: Some(surplus_addr),
                surplus_amount: 2_000,
            });
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // AddToSystemCredits + AddUsedAssetLock + AddBalanceToAddress + InsertNote + UpdateTotalBalance
        assert_eq!(ops.len(), 5);

        // The FULL consumed lock is added to system credits (not just the shield amount).
        match &ops[0] {
            DriveOperation::SystemOperation(SystemOperationType::AddToSystemCredits { amount }) => {
                assert_eq!(*amount, 10_000);
            }
            other => panic!("expected AddToSystemCredits, got {:?}", other),
        }

        // The surplus is routed to the surplus_output address.
        let has_surplus_op = ops.iter().any(|op| {
            matches!(
                op,
                DriveOperation::AddressFundsOperation(
                    AddressFundsOperationType::AddBalanceToAddress {
                        address,
                        balance_to_add,
                    },
                ) if *address == surplus_addr && *balance_to_add == 2_000
            )
        });
        assert!(
            has_surplus_op,
            "expected AddBalanceToAddress(surplus_addr, 2000)"
        );
    }

    #[test]
    fn test_without_surplus_output_emits_no_address_op() {
        // surplus_output None => no AddBalanceToAddress op; the surplus folds into the fee pools
        // at the execution-event layer instead. `make_action` has surplus_output: None.
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        assert_eq!(ops.len(), 4);
        assert!(
            !ops.iter()
                .any(|op| matches!(op, DriveOperation::AddressFundsOperation(_))),
            "no AddressFundsOperation expected when surplus_output is None"
        );
    }
}
