use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::address_funds::address_credit_withdrawal::AddressCreditWithdrawalTransitionAction;
use crate::util::batch::drive_op_batch::AddressFundsOperationType;
use crate::util::batch::DriveOperation::{
    AddressFundsOperation, DocumentOperation, SystemOperation,
};
use crate::util::batch::{DocumentOperationType, DriveOperation, SystemOperationType};
use crate::util::object_size_info::{DocumentInfo, OwnedDocumentInfo};
use dpp::block::epoch::Epoch;
use platform_version::version::PlatformVersion;

impl DriveHighLevelOperationConverter for AddressCreditWithdrawalTransitionAction {
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
            .address_credit_withdrawal_transition
        {
            0 => {
                let mut drive_operations = vec![];

                // Set remaining balances for all inputs
                for (address, (nonce, remaining_balance)) in self.inputs_with_remaining_balance() {
                    drive_operations.push(AddressFundsOperation(
                        AddressFundsOperationType::SetBalanceToAddress {
                            address: *address,
                            nonce: *nonce,
                            balance: *remaining_balance,
                        },
                    ));
                }

                // Add balance to change output if present
                if let Some((address, balance_to_add)) = self.output() {
                    drive_operations.push(AddressFundsOperation(
                        AddressFundsOperationType::AddBalanceToAddress {
                            address,
                            balance_to_add,
                        },
                    ));
                }

                let withdrawal_amount = self.amount();
                let prepared_withdrawal_document = self.prepared_withdrawal_document_owned();

                // Add the withdrawal document
                drive_operations.push(DocumentOperation(
                    DocumentOperationType::AddWithdrawalDocument {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentInfo::DocumentOwnedInfo((
                                prepared_withdrawal_document,
                                None,
                            )),
                            owner_id: None,
                        },
                    },
                ));

                // Remove from system credits
                drive_operations.push(SystemOperation(
                    SystemOperationType::RemoveFromSystemCredits {
                        amount: withdrawal_amount,
                    },
                ));

                Ok(drive_operations)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "AddressCreditWithdrawalTransitionAction::into_high_level_drive_operations"
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
    use crate::state_transition_action::address_funds::address_credit_withdrawal::v0::AddressCreditWithdrawalTransitionActionV0;
    use dpp::address_funds::PlatformAddress;
    use dpp::block::epoch::Epoch;
    use dpp::document::{Document, DocumentV0};
    use dpp::prelude::Identifier;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    fn make_document() -> Document {
        Document::V0(DocumentV0 {
            id: Identifier::from([0x11; 32]),
            owner_id: Identifier::from([0x22; 32]),
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

    fn make_action_with_output() -> AddressCreditWithdrawalTransitionAction {
        let addr_a = PlatformAddress::P2pkh([0xAA; 20]);
        let addr_b = PlatformAddress::P2sh([0xCC; 20]);

        let mut inputs = BTreeMap::new();
        inputs.insert(addr_a, (5_u32, 10000_u64));

        AddressCreditWithdrawalTransitionAction::V0(AddressCreditWithdrawalTransitionActionV0 {
            inputs_with_remaining_balance: inputs,
            output: Some((addr_b, 2000)),
            fee_strategy: vec![],
            user_fee_increase: 0,
            prepared_withdrawal_document: make_document(),
            amount: 8000,
        })
    }

    fn make_action_no_output() -> AddressCreditWithdrawalTransitionAction {
        let addr_a = PlatformAddress::P2pkh([0xAA; 20]);

        let mut inputs = BTreeMap::new();
        inputs.insert(addr_a, (5_u32, 2000_u64));

        AddressCreditWithdrawalTransitionAction::V0(AddressCreditWithdrawalTransitionActionV0 {
            inputs_with_remaining_balance: inputs,
            output: None,
            fee_strategy: vec![],
            user_fee_increase: 0,
            prepared_withdrawal_document: make_document(),
            amount: 8000,
        })
    }

    #[test]
    fn test_with_output_produces_correct_operation_count() {
        let action = make_action_with_output();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // SetBalanceToAddress (1 input) + AddBalanceToAddress (output)
        // + AddWithdrawalDocument + RemoveFromSystemCredits
        assert_eq!(ops.len(), 4);
    }

    #[test]
    fn test_without_output_produces_correct_operation_count() {
        let action = make_action_no_output();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // SetBalanceToAddress (1 input) + AddWithdrawalDocument + RemoveFromSystemCredits
        assert_eq!(ops.len(), 3);
    }

    #[test]
    fn test_input_balances_are_set() {
        let action = make_action_with_output();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        match &ops[0] {
            AddressFundsOperation(AddressFundsOperationType::SetBalanceToAddress {
                address,
                nonce,
                balance,
            }) => {
                assert_eq!(*address, PlatformAddress::P2pkh([0xAA; 20]));
                assert_eq!(*nonce, 5);
                assert_eq!(*balance, 10000);
            }
            other => panic!("expected SetBalanceToAddress, got {:?}", other),
        }
    }

    #[test]
    fn test_system_credits_removal_matches_amount() {
        let action = make_action_with_output();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        let last = ops.last().unwrap();
        match last {
            SystemOperation(SystemOperationType::RemoveFromSystemCredits { amount }) => {
                assert_eq!(*amount, 8000);
            }
            other => panic!("expected RemoveFromSystemCredits, got {:?}", other),
        }
    }

    #[test]
    fn test_has_withdrawal_document_operation() {
        let action = make_action_with_output();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        let has_withdrawal_doc = ops.iter().any(|op| {
            matches!(
                op,
                DocumentOperation(DocumentOperationType::AddWithdrawalDocument { .. })
            )
        });
        assert!(has_withdrawal_doc, "expected AddWithdrawalDocument operation");
    }
}
