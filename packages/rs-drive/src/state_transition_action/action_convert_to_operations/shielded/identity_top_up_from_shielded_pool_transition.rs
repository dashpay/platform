use super::{insert_notes, insert_nullifiers, update_balance};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::shielded::identity_top_up_from_shielded_pool::IdentityTopUpFromShieldedPoolTransitionAction;
use crate::util::batch::DriveOperation::IdentityOperation;
use crate::util::batch::{DriveOperation, IdentityOperationType};
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for IdentityTopUpFromShieldedPoolTransitionAction {
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
            .identity_top_up_from_shielded_pool_transition
        {
            0 => match self {
                IdentityTopUpFromShieldedPoolTransitionAction::V0(v0) => {
                    let mut ops: Vec<DriveOperation<'a>> = Vec::new();

                    insert_nullifiers(&mut ops, &v0.notes);

                    // The pool and the identity balance are both right-hand-side terms of
                    // the block conservation equation: the net amount moves between them
                    // and the fee moves to the fee pools; no system-credit adjustment.
                    let net_identity_amount =
                        v0.amount.checked_sub(v0.fee_amount).ok_or_else(|| {
                            Error::Drive(DriveError::CorruptedDriveState(
                                "identity top up fee exceeds top up amount".to_string(),
                            ))
                        })?;
                    if net_identity_amount > 0 {
                        ops.push(IdentityOperation(IdentityOperationType::AddToIdentityBalance {
                            identity_id: v0.identity_id.to_buffer(),
                            added_balance: net_identity_amount,
                        }));
                    }

                    insert_notes(&mut ops, &v0.notes);

                    let new_total_balance = v0
                        .current_total_balance
                        .checked_sub(v0.amount)
                        .ok_or_else(|| {
                            Error::Drive(DriveError::CorruptedDriveState(
                                "shielded pool total balance underflow when subtracting identity top up amount"
                                    .to_string(),
                            ))
                        })?;
                    update_balance(&mut ops, new_total_balance);

                    Ok(ops)
                }
            },
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method:
                    "IdentityTopUpFromShieldedPoolTransitionAction::into_high_level_drive_operations"
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
    use crate::state_transition_action::shielded::identity_top_up_from_shielded_pool::v0::IdentityTopUpFromShieldedPoolTransitionActionV0;
    use crate::state_transition_action::shielded::ShieldedActionNote;
    use crate::util::batch::drive_op_batch::{ShieldedPoolOperationType, SystemOperationType};
    use dpp::block::epoch::Epoch;
    use dpp::platform_value::Identifier;
    use dpp::version::PlatformVersion;

    fn make_note() -> ShieldedActionNote {
        ShieldedActionNote {
            nullifier: [0x11; 32],
            cmx: [0x22; 32],
            cv_net: [0x33; 32],
            encrypted_note: vec![1, 2, 3],
        }
    }

    fn make_action(amount: u64, fee: u64) -> IdentityTopUpFromShieldedPoolTransitionAction {
        IdentityTopUpFromShieldedPoolTransitionAction::V0(
            IdentityTopUpFromShieldedPoolTransitionActionV0 {
                identity_id: Identifier::from([0xAA; 32]),
                amount,
                notes: vec![make_note(), make_note()],
                anchor: [9; 32],
                fee_amount: fee,
                current_total_balance: 10_000,
            },
        )
    }

    fn ops(action: IdentityTopUpFromShieldedPoolTransitionAction) -> Vec<DriveOperation<'static>> {
        action
            .into_high_level_drive_operations(&Epoch::new(0).unwrap(), PlatformVersion::latest())
            .expect("expected operations")
    }

    #[test]
    fn test_nullifiers_identity_credit_notes_pool_total() {
        let ops = ops(make_action(3_000, 200));
        // InsertNullifiers + AddToIdentityBalance + 2 InsertNote + UpdateTotalBalance
        assert_eq!(ops.len(), 5);
        assert!(matches!(
            &ops[0],
            DriveOperation::ShieldedPoolOperation(
                ShieldedPoolOperationType::InsertNullifiers { .. }
            )
        ));
        assert!(matches!(
            &ops[1],
            IdentityOperation(IdentityOperationType::AddToIdentityBalance {
                added_balance: 2_800,
                ..
            })
        ));
        assert!(matches!(
            ops.last().unwrap(),
            DriveOperation::ShieldedPoolOperation(ShieldedPoolOperationType::UpdateTotalBalance {
                new_total_balance: 7_000
            })
        ));
    }

    #[test]
    fn test_no_system_credit_adjustment_is_emitted() {
        let ops = ops(make_action(3_000, 200));
        assert!(!ops.iter().any(|op| matches!(
            op,
            DriveOperation::SystemOperation(SystemOperationType::AddToSystemCredits { .. })
                | DriveOperation::SystemOperation(
                    SystemOperationType::RemoveFromSystemCredits { .. }
                )
        )));
    }

    #[test]
    fn test_fee_above_amount_is_an_error() {
        let result = make_action(100, 200)
            .into_high_level_drive_operations(&Epoch::new(0).unwrap(), PlatformVersion::latest());
        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::CorruptedDriveState(_)))
        ));
    }
}
