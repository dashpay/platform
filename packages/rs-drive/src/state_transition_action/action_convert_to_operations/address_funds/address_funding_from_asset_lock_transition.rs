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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition_action::address_funds::address_funding_from_asset_lock::v0::AddressFundingFromAssetLockTransitionActionV0;
    use dpp::address_funds::PlatformAddress;
    use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
    use dpp::block::epoch::Epoch;
    use dpp::platform_value::Bytes36;
    use dpp::state_transition::signable_bytes_hasher::SignableBytesHasher;
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    fn make_asset_lock_value(remaining: u64) -> AssetLockValue {
        AssetLockValue::new(
            remaining + 1000,
            vec![0xDE, 0xAD],
            remaining,
            vec![],
            PlatformVersion::latest(),
        )
        .expect("should create AssetLockValue")
    }

    fn make_action() -> AddressFundingFromAssetLockTransitionAction {
        let addr_input = PlatformAddress::P2pkh([0xAA; 20]);
        let addr_explicit = PlatformAddress::P2pkh([0xBB; 20]);
        let addr_remainder = PlatformAddress::P2sh([0xCC; 20]);

        let mut inputs = BTreeMap::new();
        inputs.insert(addr_input, (1_u32, 2000_u64));

        let mut outputs = BTreeMap::new();
        outputs.insert(addr_explicit, Some(3000_u64));
        outputs.insert(addr_remainder, None);

        AddressFundingFromAssetLockTransitionAction::V0(
            AddressFundingFromAssetLockTransitionActionV0 {
                signable_bytes_hasher: SignableBytesHasher::Bytes(vec![0xFF; 8]),
                asset_lock_value_to_be_consumed: make_asset_lock_value(5000),
                asset_lock_outpoint: Bytes36([0xDD; 36]),
                inputs_with_remaining_balance: inputs,
                outputs,
                input_contributions_total: 2000,
                fee_strategy: vec![],
                user_fee_increase: 0,
            },
        )
    }

    #[test]
    fn test_produces_system_input_and_output_operations() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // AddToSystemCredits + AddUsedAssetLock + SetBalanceToAddress (1 input)
        // + AddBalanceToAddress (explicit) + AddBalanceToAddress (remainder)
        assert_eq!(ops.len(), 5);
    }

    #[test]
    fn test_first_op_is_add_to_system_credits() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        match &ops[0] {
            DriveOperation::SystemOperation(SystemOperationType::AddToSystemCredits { amount }) => {
                assert_eq!(*amount, 5000); // remaining asset lock value
            }
            other => panic!("expected AddToSystemCredits, got {:?}", other),
        }
    }

    #[test]
    fn test_second_op_is_add_used_asset_lock() {
        let action = make_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        match &ops[1] {
            DriveOperation::SystemOperation(SystemOperationType::AddUsedAssetLock {
                asset_lock_outpoint,
                asset_lock_value,
            }) => {
                assert_eq!(*asset_lock_outpoint, Bytes36([0xDD; 36]));
                // remaining should be 0 after consumption
                use dpp::asset_lock::reduced_asset_lock_value::AssetLockValueGettersV0;
                assert_eq!(asset_lock_value.remaining_credit_value(), 0);
            }
            other => panic!("expected AddUsedAssetLock, got {:?}", other),
        }
    }

    #[test]
    fn test_no_inputs_produces_only_system_and_output_ops() {
        let addr_output = PlatformAddress::P2pkh([0xBB; 20]);
        let mut outputs = BTreeMap::new();
        outputs.insert(addr_output, Some(5000_u64));

        let action = AddressFundingFromAssetLockTransitionAction::V0(
            AddressFundingFromAssetLockTransitionActionV0 {
                signable_bytes_hasher: SignableBytesHasher::Bytes(vec![]),
                asset_lock_value_to_be_consumed: make_asset_lock_value(5000),
                asset_lock_outpoint: Bytes36([0xDD; 36]),
                inputs_with_remaining_balance: BTreeMap::new(),
                outputs,
                input_contributions_total: 0,
                fee_strategy: vec![],
                user_fee_increase: 0,
            },
        );
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // AddToSystemCredits + AddUsedAssetLock + AddBalanceToAddress
        assert_eq!(ops.len(), 3);
    }
}
