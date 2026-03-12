use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::util::batch::DriveOperation::IdentityOperation;
use crate::util::batch::{DriveOperation, IdentityOperationType};

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::identity::identity_credit_transfer::IdentityCreditTransferTransitionAction;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for IdentityCreditTransferTransitionAction {
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
            .identity_credit_transfer_transition
        {
            0 => {
                let recipient_id = self.recipient_id();
                let identity_id = self.identity_id();
                let transfer_amount = self.transfer_amount();
                let nonce = self.nonce();

                let drive_operations = vec![
                    IdentityOperation(IdentityOperationType::UpdateIdentityNonce {
                        identity_id: identity_id.into_buffer(),
                        nonce,
                    }),
                    IdentityOperation(IdentityOperationType::RemoveFromIdentityBalance {
                        identity_id: identity_id.to_buffer(),
                        balance_to_remove: transfer_amount,
                    }),
                    IdentityOperation(IdentityOperationType::AddToIdentityBalance {
                        identity_id: recipient_id.to_buffer(),
                        added_balance: transfer_amount,
                    }),
                ];
                Ok(drive_operations)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "IdentityCreditTransferTransitionAction::into_high_level_drive_operations"
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
    use crate::state_transition_action::identity::identity_credit_transfer::v0::IdentityCreditTransferTransitionActionV0;
    use dpp::block::epoch::Epoch;
    use dpp::platform_value::Identifier;
    use dpp::version::PlatformVersion;

    fn make_action() -> IdentityCreditTransferTransitionAction {
        IdentityCreditTransferTransitionAction::V0(IdentityCreditTransferTransitionActionV0 {
            transfer_amount: 5000,
            recipient_id: Identifier::from([0xBB; 32]),
            identity_id: Identifier::from([0xAA; 32]),
            nonce: 42,
            user_fee_increase: 3,
        })
    }

    #[test]
    fn test_produces_three_operations() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // UpdateIdentityNonce, RemoveFromIdentityBalance, AddToIdentityBalance
        assert_eq!(ops.len(), 3);
    }

    #[test]
    fn test_first_op_is_update_nonce() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        match &ops[0] {
            IdentityOperation(IdentityOperationType::UpdateIdentityNonce {
                identity_id,
                nonce,
            }) => {
                assert_eq!(*identity_id, [0xAA; 32]);
                assert_eq!(*nonce, 42);
            }
            other => panic!("expected UpdateIdentityNonce, got {:?}", other),
        }
    }

    #[test]
    fn test_second_op_removes_from_sender_balance() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        match &ops[1] {
            IdentityOperation(IdentityOperationType::RemoveFromIdentityBalance {
                identity_id,
                balance_to_remove,
            }) => {
                assert_eq!(*identity_id, [0xAA; 32]);
                assert_eq!(*balance_to_remove, 5000);
            }
            other => panic!("expected RemoveFromIdentityBalance, got {:?}", other),
        }
    }

    #[test]
    fn test_third_op_adds_to_recipient_balance() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        match &ops[2] {
            IdentityOperation(IdentityOperationType::AddToIdentityBalance {
                identity_id,
                added_balance,
            }) => {
                assert_eq!(*identity_id, [0xBB; 32]);
                assert_eq!(*added_balance, 5000);
            }
            other => panic!("expected AddToIdentityBalance, got {:?}", other),
        }
    }
}
