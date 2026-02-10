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
