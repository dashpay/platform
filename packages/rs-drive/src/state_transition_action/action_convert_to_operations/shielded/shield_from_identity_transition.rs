use super::{insert_notes, update_balance};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::shielded::shield_from_identity::ShieldFromIdentityTransitionAction;
use crate::util::batch::DriveOperation::IdentityOperation;
use crate::util::batch::{DriveOperation, IdentityOperationType};
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for ShieldFromIdentityTransitionAction {
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
            .shield_from_identity_transition
        {
            0 => match self {
                ShieldFromIdentityTransitionAction::V0(v0) => {
                    let identity_id = v0.identity_id.to_buffer();

                    // The identity balance and the shielded pool are both right-hand-side
                    // terms of the block credit-conservation equation, so this is a move
                    // between two buckets: no system-credit adjustment is emitted.
                    let mut ops: Vec<DriveOperation<'a>> = vec![
                        IdentityOperation(IdentityOperationType::UpdateIdentityNonce {
                            identity_id,
                            nonce: v0.nonce,
                        }),
                        IdentityOperation(IdentityOperationType::RemoveFromIdentityBalance {
                            identity_id,
                            balance_to_remove: v0.shield_amount,
                        }),
                    ];

                    insert_notes(&mut ops, &v0.notes);

                    let new_total_balance = v0
                        .current_total_balance
                        .checked_add(v0.shield_amount)
                        .ok_or_else(|| {
                            Error::Drive(DriveError::CorruptedDriveState(
                                "shielded pool total balance overflow when adding shield from identity amount"
                                    .to_string(),
                            ))
                        })?;
                    update_balance(&mut ops, new_total_balance);

                    Ok(ops)
                }
            },
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "ShieldFromIdentityTransitionAction::into_high_level_drive_operations"
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
    use crate::state_transition_action::shielded::shield_from_identity::v0::ShieldFromIdentityTransitionActionV0;
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

    fn make_action(notes: usize) -> ShieldFromIdentityTransitionAction {
        ShieldFromIdentityTransitionAction::V0(ShieldFromIdentityTransitionActionV0 {
            identity_id: Identifier::from([0xAA; 32]),
            nonce: 7,
            shield_amount: 3000,
            notes: vec![make_note(); notes],
            user_fee_increase: 0,
            current_total_balance: 10000,
        })
    }

    fn ops(action: ShieldFromIdentityTransitionAction) -> Vec<DriveOperation<'static>> {
        action
            .into_high_level_drive_operations(&Epoch::new(0).unwrap(), PlatformVersion::latest())
            .expect("expected operations")
    }

    #[test]
    fn test_nonce_then_balance_removal_then_notes_then_pool_total() {
        let ops = ops(make_action(2));
        // UpdateIdentityNonce + RemoveFromIdentityBalance + 2 InsertNote + UpdateTotalBalance
        assert_eq!(ops.len(), 5);
        assert!(matches!(
            &ops[0],
            IdentityOperation(IdentityOperationType::UpdateIdentityNonce { nonce: 7, .. })
        ));
        assert!(matches!(
            &ops[1],
            IdentityOperation(IdentityOperationType::RemoveFromIdentityBalance {
                balance_to_remove: 3000,
                ..
            })
        ));
        assert!(matches!(
            &ops[2],
            DriveOperation::ShieldedPoolOperation(ShieldedPoolOperationType::InsertNote { .. })
        ));
        assert!(matches!(
            ops.last().unwrap(),
            DriveOperation::ShieldedPoolOperation(ShieldedPoolOperationType::UpdateTotalBalance {
                new_total_balance: 13000
            })
        ));
    }

    #[test]
    fn test_no_system_credit_adjustment_is_emitted() {
        let ops = ops(make_action(1));
        assert!(
            !ops.iter().any(|op| matches!(
                op,
                DriveOperation::SystemOperation(SystemOperationType::AddToSystemCredits { .. })
                    | DriveOperation::SystemOperation(
                        SystemOperationType::RemoveFromSystemCredits { .. }
                    )
            )),
            "identity to pool is a move between two balance trees; system credits must not change"
        );
    }

    #[test]
    fn test_pool_total_overflow_is_an_error() {
        let action = ShieldFromIdentityTransitionAction::V0(ShieldFromIdentityTransitionActionV0 {
            identity_id: Identifier::from([0xAA; 32]),
            nonce: 1,
            shield_amount: 10,
            notes: vec![make_note()],
            user_fee_increase: 0,
            current_total_balance: u64::MAX - 5,
        });
        let result = action
            .into_high_level_drive_operations(&Epoch::new(0).unwrap(), PlatformVersion::latest());
        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::CorruptedDriveState(_)))
        ));
    }
}
