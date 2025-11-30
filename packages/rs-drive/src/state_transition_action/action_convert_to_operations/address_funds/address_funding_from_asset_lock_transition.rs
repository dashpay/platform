use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::address_funds::address_funding_from_asset_lock::AddressFundingFromAssetLockTransitionAction;
use crate::util::batch::drive_op_batch::AddressFundsOperationType;
use crate::util::batch::DriveOperation;
use crate::util::batch::DriveOperation::AddressFundsOperation;
use dpp::block::epoch::Epoch;
use platform_version::version::PlatformVersion;

impl DriveHighLevelOperationConverter for AddressFundingFromAssetLockTransitionAction {
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
            .address_funding_from_asset_lock_transition
        {
            0 => {
                let (inputs, outputs) = self.inputs_with_remaining_balance_and_outputs_owned();
                let mut drive_operations = vec![];

                // Set remaining balances for all inputs (if any existing addresses were used)
                for (address, (nonce, remaining_balance)) in inputs {
                    drive_operations.push(AddressFundsOperation(
                        AddressFundsOperationType::SetBalanceToAddress {
                            address,
                            nonce,
                            balance: remaining_balance,
                        },
                    ));
                }

                // Add balance to outputs
                for (address, balance_to_add) in outputs {
                    drive_operations.push(AddressFundsOperation(
                        AddressFundsOperationType::AddBalanceToAddress {
                            address,
                            balance_to_add,
                        },
                    ));
                }

                Ok(drive_operations)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method:
                    "AddressFundingFromAssetLockTransitionAction::into_high_level_drive_operations"
                        .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
