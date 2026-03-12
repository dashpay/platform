use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::system::penalize_shielded_pool_action::PenalizeShieldedPoolAction;
use crate::util::batch::drive_op_batch::ShieldedPoolOperationType;
use crate::util::batch::DriveOperation;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for PenalizeShieldedPoolAction {
    fn into_high_level_drive_operations<'a>(
        self,
        _epoch: &Epoch,
        _platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        match self {
            PenalizeShieldedPoolAction::V0(v0) => {
                let mut ops: Vec<DriveOperation<'a>> = Vec::new();

                // 1. Record nullifiers as spent (prevents replaying the same invalid proof)
                if !v0.nullifiers.is_empty() {
                    ops.push(DriveOperation::ShieldedPoolOperation(
                        ShieldedPoolOperationType::InsertNullifiers {
                            nullifiers: v0.nullifiers,
                        },
                    ));
                }

                // 2. Deduct penalty from pool total balance
                let new_total_balance = v0
                    .current_total_balance
                    .checked_sub(v0.penalty_amount)
                    .ok_or_else(|| {
                        Error::Drive(DriveError::CorruptedDriveState(
                            "shielded pool total balance underflow when subtracting penalty_amount"
                                .to_string(),
                        ))
                    })?;
                ops.push(DriveOperation::ShieldedPoolOperation(
                    ShieldedPoolOperationType::UpdateTotalBalance { new_total_balance },
                ));

                Ok(ops)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition_action::system::penalize_shielded_pool_action::v0::PenalizeShieldedPoolActionV0;
    use dpp::block::epoch::Epoch;
    use dpp::version::PlatformVersion;

    #[test]
    fn test_with_non_empty_nullifiers() {
        let action = PenalizeShieldedPoolAction::V0(PenalizeShieldedPoolActionV0 {
            penalty_amount: 1000,
            nullifiers: vec![[0xAA; 32], [0xBB; 32]],
            current_total_balance: 10000,
        });
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // Should produce InsertNullifiers + UpdateTotalBalance
        assert_eq!(ops.len(), 2);

        match &ops[0] {
            DriveOperation::ShieldedPoolOperation(ShieldedPoolOperationType::InsertNullifiers {
                nullifiers,
            }) => {
                assert_eq!(nullifiers.len(), 2);
                assert_eq!(nullifiers[0], [0xAA; 32]);
                assert_eq!(nullifiers[1], [0xBB; 32]);
            }
            other => panic!("expected InsertNullifiers, got {:?}", other),
        }

        match &ops[1] {
            DriveOperation::ShieldedPoolOperation(
                ShieldedPoolOperationType::UpdateTotalBalance { new_total_balance },
            ) => {
                assert_eq!(*new_total_balance, 9000); // 10000 - 1000
            }
            other => panic!("expected UpdateTotalBalance, got {:?}", other),
        }
    }

    #[test]
    fn test_with_empty_nullifiers() {
        let action = PenalizeShieldedPoolAction::V0(PenalizeShieldedPoolActionV0 {
            penalty_amount: 500,
            nullifiers: vec![],
            current_total_balance: 5000,
        });
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // Should produce only UpdateTotalBalance (no InsertNullifiers since empty)
        assert_eq!(ops.len(), 1);

        match &ops[0] {
            DriveOperation::ShieldedPoolOperation(
                ShieldedPoolOperationType::UpdateTotalBalance { new_total_balance },
            ) => {
                assert_eq!(*new_total_balance, 4500); // 5000 - 500
            }
            other => panic!("expected UpdateTotalBalance, got {:?}", other),
        }
    }

    #[test]
    fn test_underflow_returns_error() {
        let action = PenalizeShieldedPoolAction::V0(PenalizeShieldedPoolActionV0 {
            penalty_amount: 10001,
            nullifiers: vec![],
            current_total_balance: 10000,
        });
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let result = action.into_high_level_drive_operations(&epoch, platform_version);
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            Error::Drive(DriveError::CorruptedDriveState(msg)) => {
                assert!(msg.contains("underflow"));
            }
            other => panic!("expected CorruptedDriveState, got {:?}", other),
        }
    }

    #[test]
    fn test_penalty_equals_balance_results_in_zero() {
        let action = PenalizeShieldedPoolAction::V0(PenalizeShieldedPoolActionV0 {
            penalty_amount: 5000,
            nullifiers: vec![[0x11; 32]],
            current_total_balance: 5000,
        });
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        assert_eq!(ops.len(), 2);

        match &ops[1] {
            DriveOperation::ShieldedPoolOperation(
                ShieldedPoolOperationType::UpdateTotalBalance { new_total_balance },
            ) => {
                assert_eq!(*new_total_balance, 0);
            }
            other => panic!("expected UpdateTotalBalance, got {:?}", other),
        }
    }

    #[test]
    fn test_zero_penalty() {
        let action = PenalizeShieldedPoolAction::V0(PenalizeShieldedPoolActionV0 {
            penalty_amount: 0,
            nullifiers: vec![],
            current_total_balance: 10000,
        });
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        assert_eq!(ops.len(), 1);
        match &ops[0] {
            DriveOperation::ShieldedPoolOperation(
                ShieldedPoolOperationType::UpdateTotalBalance { new_total_balance },
            ) => {
                assert_eq!(*new_total_balance, 10000);
            }
            other => panic!("expected UpdateTotalBalance, got {:?}", other),
        }
    }
}
