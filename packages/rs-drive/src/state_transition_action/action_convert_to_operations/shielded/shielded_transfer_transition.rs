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

                    // 1. Insert each nullifier (known to not exist after validation)
                    insert_nullifiers(&mut ops, &v0.notes);

                    // 2. Insert notes into CommitmentTree
                    insert_notes(&mut ops, &v0.notes);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition_action::shielded::shielded_transfer::v0::ShieldedTransferTransitionActionV0;
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

    fn make_action() -> ShieldedTransferTransitionAction {
        ShieldedTransferTransitionAction::V0(ShieldedTransferTransitionActionV0 {
            notes: vec![make_note()],
            anchor: [0xAA; 32],
            fee_amount: 500,
            current_total_balance: 10000,
        })
    }

    #[test]
    fn test_produces_nullifiers_notes_and_balance_update() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // InsertNullifiers + InsertNote (1 note) + UpdateTotalBalance
        assert_eq!(ops.len(), 3);
    }

    #[test]
    fn test_first_op_is_insert_nullifiers() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        match &ops[0] {
            DriveOperation::ShieldedPoolOperation(ShieldedPoolOperationType::InsertNullifiers {
                nullifiers,
            }) => {
                assert_eq!(nullifiers.len(), 1);
                assert_eq!(nullifiers[0], [0x11; 32]);
            }
            other => panic!("expected InsertNullifiers, got {:?}", other),
        }
    }

    #[test]
    fn test_balance_decreases_by_fee_amount() {
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
                assert_eq!(*new_total_balance, 9500); // 10000 - 500
            }
            other => panic!("expected UpdateTotalBalance, got {:?}", other),
        }
    }

    #[test]
    fn test_underflow_returns_error() {
        let action =
            ShieldedTransferTransitionAction::V0(ShieldedTransferTransitionActionV0 {
                notes: vec![],
                anchor: [0x00; 32],
                fee_amount: 10001,
                current_total_balance: 10000,
            });
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let result = action.into_high_level_drive_operations(&epoch, platform_version);
        assert!(result.is_err());
    }
}
