use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::util::batch::DriveOperation::{DocumentOperation, IdentityOperation, SystemOperation};
use crate::util::batch::{
    DocumentOperationType, DriveOperation, IdentityOperationType, SystemOperationType,
};
use crate::util::object_size_info::{DocumentInfo, OwnedDocumentInfo};
use dpp::block::epoch::Epoch;

use crate::error::drive::DriveError;
use crate::state_transition_action::identity::identity_credit_withdrawal::IdentityCreditWithdrawalTransitionAction;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for IdentityCreditWithdrawalTransitionAction {
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
            .identity_credit_withdrawal_transition
        {
            0 => {
                let identity_id = self.identity_id();
                let nonce = self.nonce();
                let balance_to_remove = self.amount();
                let prepared_withdrawal_document = self.prepared_withdrawal_document_owned();

                let drive_operations = vec![
                    IdentityOperation(IdentityOperationType::RemoveFromIdentityBalance {
                        identity_id: identity_id.to_buffer(),
                        balance_to_remove,
                    }),
                    IdentityOperation(IdentityOperationType::UpdateIdentityNonce {
                        identity_id: identity_id.into_buffer(),
                        nonce,
                    }),
                    DocumentOperation(DocumentOperationType::AddWithdrawalDocument {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentInfo::DocumentOwnedInfo((
                                prepared_withdrawal_document,
                                None,
                            )),
                            owner_id: None,
                        },
                    }),
                    SystemOperation(SystemOperationType::RemoveFromSystemCredits {
                        amount: balance_to_remove,
                    }),
                ];

                Ok(drive_operations)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method:
                    "IdentityCreditWithdrawalTransitionAction::into_high_level_drive_operations"
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
    use crate::state_transition_action::identity::identity_credit_withdrawal::v0::IdentityCreditWithdrawalTransitionActionV0;
    use dpp::block::epoch::Epoch;
    use dpp::document::{Document, DocumentV0};
    use dpp::platform_value::Identifier;
    use dpp::version::PlatformVersion;

    fn make_document() -> Document {
        Document::V0(DocumentV0 {
            id: Identifier::from([0xDD; 32]),
            owner_id: Identifier::from([0xAA; 32]),
            properties: Default::default(),
            revision: Some(1),
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        })
    }

    fn make_action() -> IdentityCreditWithdrawalTransitionAction {
        IdentityCreditWithdrawalTransitionAction::V0(
            IdentityCreditWithdrawalTransitionActionV0 {
                identity_id: Identifier::from([0xAA; 32]),
                nonce: 10,
                prepared_withdrawal_document: make_document(),
                amount: 7500,
                user_fee_increase: 0,
            },
        )
    }

    #[test]
    fn test_produces_four_operations() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // RemoveFromIdentityBalance + UpdateIdentityNonce + AddWithdrawalDocument + RemoveFromSystemCredits
        assert_eq!(ops.len(), 4);
    }

    #[test]
    fn test_first_op_is_remove_from_identity_balance() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        match &ops[0] {
            DriveOperation::IdentityOperation(
                IdentityOperationType::RemoveFromIdentityBalance {
                    identity_id,
                    balance_to_remove,
                },
            ) => {
                assert_eq!(*identity_id, [0xAA; 32]);
                assert_eq!(*balance_to_remove, 7500);
            }
            other => panic!("expected RemoveFromIdentityBalance, got {:?}", other),
        }
    }

    #[test]
    fn test_second_op_is_update_identity_nonce() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        match &ops[1] {
            DriveOperation::IdentityOperation(IdentityOperationType::UpdateIdentityNonce {
                identity_id,
                nonce,
            }) => {
                assert_eq!(*identity_id, [0xAA; 32]);
                assert_eq!(*nonce, 10);
            }
            other => panic!("expected UpdateIdentityNonce, got {:?}", other),
        }
    }

    #[test]
    fn test_third_op_is_add_withdrawal_document() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        assert!(matches!(
            &ops[2],
            DocumentOperation(DocumentOperationType::AddWithdrawalDocument { .. })
        ));
    }

    #[test]
    fn test_fourth_op_is_remove_from_system_credits() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        match &ops[3] {
            SystemOperation(SystemOperationType::RemoveFromSystemCredits { amount }) => {
                assert_eq!(*amount, 7500);
            }
            other => panic!("expected RemoveFromSystemCredits, got {:?}", other),
        }
    }
}
