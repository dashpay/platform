use super::{insert_notes, insert_nullifiers, update_balance};
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition_action::shielded::unshield::v0::UnshieldTransitionActionV0;
    use crate::state_transition_action::shielded::ShieldedActionNote;
    use crate::util::batch::drive_op_batch::ShieldedPoolOperationType;
    use dpp::address_funds::PlatformAddress;
    use dpp::block::epoch::Epoch;
    use dpp::version::PlatformVersion;

    fn make_note() -> ShieldedActionNote {
        ShieldedActionNote {
            nullifier: [0x11; 32],
            cmx: [0x22; 32],
            encrypted_note: vec![1, 2, 3],
        }
    }

    fn make_action() -> UnshieldTransitionAction {
        UnshieldTransitionAction::V0(UnshieldTransitionActionV0 {
            output_address: PlatformAddress::P2pkh([0xBB; 20]),
            amount: 3000,
            notes: vec![make_note()],
            anchor: [0xAA; 32],
            fee_amount: 500,
            current_total_balance: 10000,
        })
    }

    #[test]
    fn test_produces_nullifiers_add_balance_notes_and_update_balance() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // InsertNullifiers + AddBalanceToAddress + InsertNote (1) + UpdateTotalBalance
        assert_eq!(ops.len(), 4);
    }

    #[test]
    fn test_add_balance_to_output_address() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        match &ops[1] {
            DriveOperation::AddressFundsOperation(
                AddressFundsOperationType::AddBalanceToAddress {
                    address,
                    balance_to_add,
                },
            ) => {
                assert_eq!(*address, PlatformAddress::P2pkh([0xBB; 20]));
                assert_eq!(*balance_to_add, 3000);
            }
            other => panic!("expected AddBalanceToAddress, got {:?}", other),
        }
    }

    #[test]
    fn test_balance_decreases_by_amount_plus_fee() {
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
                assert_eq!(*new_total_balance, 6500); // 10000 - 3000 - 500
            }
            other => panic!("expected UpdateTotalBalance, got {:?}", other),
        }
    }

    #[test]
    fn test_amount_plus_fee_overflow_returns_error() {
        let action = UnshieldTransitionAction::V0(UnshieldTransitionActionV0 {
            output_address: PlatformAddress::P2pkh([0xBB; 20]),
            amount: u64::MAX,
            notes: vec![],
            anchor: [0x00; 32],
            fee_amount: 1,
            current_total_balance: u64::MAX,
        });
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let result = action.into_high_level_drive_operations(&epoch, platform_version);
        assert!(result.is_err());
    }

    #[test]
    fn test_balance_underflow_returns_error() {
        let action = UnshieldTransitionAction::V0(UnshieldTransitionActionV0 {
            output_address: PlatformAddress::P2pkh([0xBB; 20]),
            amount: 5000,
            notes: vec![],
            anchor: [0x00; 32],
            fee_amount: 500,
            current_total_balance: 5000, // 5000 < 5000 + 500
        });
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let result = action.into_high_level_drive_operations(&epoch, platform_version);
        assert!(result.is_err());
    }
}
