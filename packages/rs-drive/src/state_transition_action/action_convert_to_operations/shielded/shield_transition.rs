use super::{insert_notes, update_balance};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::shielded::shield::ShieldTransitionAction;
use crate::util::batch::drive_op_batch::AddressFundsOperationType;
use crate::util::batch::DriveOperation;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for ShieldTransitionAction {
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
            .shield_transition
        {
            0 => match self {
                ShieldTransitionAction::V0(v0) => {
                    let mut ops: Vec<DriveOperation<'a>> = Vec::new();

                    // 1. Debit each input address: set remaining balance
                    for (address, (nonce, remaining_balance)) in v0.inputs_with_remaining_balance {
                        ops.push(DriveOperation::AddressFundsOperation(
                            AddressFundsOperationType::SetBalanceToAddress {
                                address,
                                nonce,
                                balance: remaining_balance,
                            },
                        ));
                    }

                    // 2. Insert notes into CommitmentTree
                    insert_notes(&mut ops, &v0.notes);

                    // 3. Update total balance
                    let new_total_balance = v0
                        .current_total_balance
                        .checked_add(v0.shield_amount)
                        .ok_or_else(|| {
                            Error::Drive(DriveError::CorruptedDriveState(
                                "shielded pool total balance overflow when adding shield amount"
                                    .to_string(),
                            ))
                        })?;
                    update_balance(&mut ops, new_total_balance);

                    Ok(ops)
                }
            },
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "ShieldTransitionAction::into_high_level_drive_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition_action::shielded::shield::v0::ShieldTransitionActionV0;
    use crate::state_transition_action::shielded::ShieldedActionNote;
    use crate::util::batch::drive_op_batch::ShieldedPoolOperationType;
    use dpp::address_funds::PlatformAddress;
    use dpp::block::epoch::Epoch;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    fn make_note() -> ShieldedActionNote {
        ShieldedActionNote {
            nullifier: [0x11; 32],
            cmx: [0x22; 32],
            encrypted_note: vec![1, 2, 3],
        }
    }

    fn make_action() -> ShieldTransitionAction {
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([0xAA; 20]), (1_u32, 5000_u64));

        ShieldTransitionAction::V0(ShieldTransitionActionV0 {
            inputs_with_remaining_balance: inputs,
            shield_amount: 3000,
            notes: vec![make_note()],
            fee_strategy: vec![],
            user_fee_increase: 0,
            current_total_balance: 10000,
        })
    }

    #[test]
    fn test_produces_set_balance_insert_note_and_update_balance() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // SetBalanceToAddress (1 input) + InsertNote (1 note) + UpdateTotalBalance
        assert_eq!(ops.len(), 3);
    }

    #[test]
    fn test_first_op_is_set_balance_to_address() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        match &ops[0] {
            DriveOperation::AddressFundsOperation(AddressFundsOperationType::SetBalanceToAddress {
                address,
                nonce,
                balance,
            }) => {
                assert_eq!(*address, PlatformAddress::P2pkh([0xAA; 20]));
                assert_eq!(*nonce, 1);
                assert_eq!(*balance, 5000);
            }
            other => panic!("expected SetBalanceToAddress, got {:?}", other),
        }
    }

    #[test]
    fn test_last_op_is_update_total_balance() {
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
                assert_eq!(*new_total_balance, 13000); // 10000 + 3000
            }
            other => panic!("expected UpdateTotalBalance, got {:?}", other),
        }
    }

    #[test]
    fn test_overflow_returns_error() {
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([0xAA; 20]), (1_u32, 5000_u64));

        let action = ShieldTransitionAction::V0(ShieldTransitionActionV0 {
            inputs_with_remaining_balance: inputs,
            shield_amount: u64::MAX,
            notes: vec![],
            fee_strategy: vec![],
            user_fee_increase: 0,
            current_total_balance: 1,
        });
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let result = action.into_high_level_drive_operations(&epoch, platform_version);
        assert!(result.is_err());
    }
}
