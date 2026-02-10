use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::address_funds::address_funding_from_asset_lock::AddressFundingFromAssetLockTransitionAction;
use crate::util::batch::drive_op_batch::AddressFundsOperationType;
use crate::util::batch::DriveOperation::{AddressFundsOperation, SystemOperation};
use crate::util::batch::{DriveOperation, SystemOperationType};
use dpp::asset_lock::reduced_asset_lock_value::{AssetLockValueGettersV0, AssetLockValueSettersV0};
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
                let asset_lock_outpoint = self.asset_lock_outpoint();

                let (inputs, outputs, mut asset_lock_value, input_contributions_total) =
                    self.inputs_with_remaining_balance_outputs_and_asset_lock_value_owned();

                let initial_balance = asset_lock_value.remaining_credit_value();

                asset_lock_value.set_remaining_credit_value(0); // We are using the entire value

                let mut drive_operations = vec![
                    SystemOperation(SystemOperationType::AddToSystemCredits {
                        amount: initial_balance,
                    }),
                    SystemOperation(SystemOperationType::AddUsedAssetLock {
                        asset_lock_outpoint,
                        asset_lock_value,
                    }),
                ];

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

                // Calculate the sum of explicit outputs
                let explicit_outputs_sum: u64 = outputs.values().flatten().sum();

                // Total available for outputs = asset lock + input contributions
                // (input credits are already in the system — we're moving them, not creating them)
                let total_available = initial_balance + input_contributions_total;

                // Calculate remainder: total_available - explicit_outputs_sum
                let remainder_balance = total_available.saturating_sub(explicit_outputs_sum);

                // Add balance to outputs
                for (address, balance_option) in outputs {
                    let balance_to_add = match balance_option {
                        Some(explicit_amount) => explicit_amount,
                        None => remainder_balance, // This address receives the remainder
                    };
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
