use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::identity::identity_topup_from_addresses::IdentityTopUpFromAddressesTransitionAction;
use crate::util::batch::drive_op_batch::AddressFundsOperationType;
use crate::util::batch::DriveOperation::{AddressFundsOperation, IdentityOperation};
use crate::util::batch::{DriveOperation, IdentityOperationType};
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for IdentityTopUpFromAddressesTransitionAction {
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
            .identity_top_up_from_addresses_transition
        {
            0 => {
                let identity_id = self.identity_id();
                let topup_amount = self.topup_amount();
                let output = self.output();
                let inputs = self.inputs_with_remaining_balance_owned();

                let mut drive_operations = vec![IdentityOperation(
                    IdentityOperationType::AddToIdentityBalance {
                        identity_id: identity_id.to_buffer(),
                        added_balance: topup_amount,
                    },
                )];

                // Update input address balances
                for (address, (nonce, remaining_balance)) in inputs {
                    drive_operations.push(AddressFundsOperation(
                        AddressFundsOperationType::SetBalanceToAddress {
                            address,
                            nonce,
                            balance: remaining_balance,
                        },
                    ));
                }

                // Handle output address if present
                if let Some((output_address, output_balance)) = output {
                    drive_operations.push(AddressFundsOperation(
                        AddressFundsOperationType::AddBalanceToAddress {
                            address: output_address,
                            balance_to_add: output_balance,
                        },
                    ));
                }

                Ok(drive_operations)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method:
                    "IdentityTopUpFromAddressesTransitionAction::into_high_level_drive_operations"
                        .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
