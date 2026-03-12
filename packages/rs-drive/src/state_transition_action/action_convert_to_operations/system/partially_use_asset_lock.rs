use crate::error::drive::DriveError;
use crate::error::Error;
use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::state_transition_action::system::partially_use_asset_lock_action::{
    PartiallyUseAssetLockAction, PartiallyUseAssetLockActionAccessorsV0,
};
use crate::util::batch::drive_op_batch::AddressFundsOperationType;
use crate::util::batch::DriveOperation::{AddressFundsOperation, SystemOperation};
use crate::util::batch::{DriveOperation, SystemOperationType};
use dpp::address_funds::AddressFundsFeeStrategyStep;
use dpp::asset_lock::reduced_asset_lock_value::AssetLockValue;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for PartiallyUseAssetLockAction {
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
            .partially_use_asset_lock
        {
            0 => {
                let initial_credit_value = self.initial_credit_value();
                // The remaining credit value is already computed here
                let mut remaining_credit_value = self.remaining_credit_value();
                let used_credits = self.used_credits();
                let asset_lock_outpoint = self.asset_lock_outpoint();

                let previous_transaction_hashes = if self.previous_transaction_hashes_ref().len()
                    as u16
                    >= platform_version
                        .drive_abci
                        .validation_and_processing
                        .state_transitions
                        .max_asset_lock_usage_attempts
                {
                    // There have been 16 failed attempts at using the asset lock
                    // In this case the remaining credit value is burned and there is no need to keep around previous
                    // transaction hashes
                    remaining_credit_value = 0;
                    vec![]
                } else {
                    self.previous_transaction_hashes_ref().clone()
                };

                // Get inputs and fee_strategy before consuming self
                let inputs_and_strategy = self
                    .inputs_with_remaining_balance()
                    .cloned()
                    .zip(self.fee_strategy().cloned());

                let tx_out_script = self.asset_lock_script_owned();

                let mut drive_operations = Vec::new();

                // If we have inputs and a fee strategy, deduct fees from inputs first
                // Note: remaining_credit_value was already pre-computed to deduct ALL used_credits
                // from the asset lock. Here we restore the portion that's covered by inputs.
                if let Some((inputs, fee_strategy)) = inputs_and_strategy {
                    let inputs_ordered: Vec<_> = inputs.iter().collect();
                    let mut remaining_fee = used_credits;
                    let mut total_deducted_from_inputs = 0u64;

                    // Process fee strategy steps in order
                    for step in &fee_strategy {
                        if remaining_fee == 0 {
                            break;
                        }

                        match step {
                            AddressFundsFeeStrategyStep::DeductFromInput(index) => {
                                // Get the input at this index
                                if let Some((address, (nonce, balance))) =
                                    inputs_ordered.get(*index as usize)
                                {
                                    // Deduct as much as possible from this input
                                    let deduction = std::cmp::min(*balance, remaining_fee);
                                    if deduction > 0 {
                                        let new_balance = balance.saturating_sub(deduction);
                                        remaining_fee = remaining_fee.saturating_sub(deduction);
                                        total_deducted_from_inputs += deduction;

                                        // Add operation to set the new balance
                                        drive_operations.push(AddressFundsOperation(
                                            AddressFundsOperationType::SetBalanceToAddress {
                                                address: **address,
                                                nonce: *nonce,
                                                balance: new_balance,
                                            },
                                        ));
                                    }
                                }
                            }
                            AddressFundsFeeStrategyStep::ReduceOutput(_) => {
                                // ReduceOutput is handled differently for partial use -
                                // since the transition failed, outputs aren't created,
                                // so we skip this step
                            }
                        }
                    }

                    // The remaining_credit_value was pre-computed assuming ALL fees come from
                    // asset lock. Restore the portion that was covered by inputs.
                    remaining_credit_value =
                        remaining_credit_value.saturating_add(total_deducted_from_inputs);
                }

                // Add system credits operation
                drive_operations.push(SystemOperation(SystemOperationType::AddToSystemCredits {
                    amount: used_credits,
                }));

                // Add used asset lock operation
                drive_operations.push(SystemOperation(SystemOperationType::AddUsedAssetLock {
                    asset_lock_outpoint,
                    asset_lock_value: AssetLockValue::new(
                        initial_credit_value,
                        tx_out_script,
                        remaining_credit_value,
                        previous_transaction_hashes,
                        platform_version,
                    )?,
                }));

                Ok(drive_operations)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "PartiallyUseAssetLockAction::into_high_level_drive_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_transition_action::system::partially_use_asset_lock_action::PartiallyUseAssetLockActionV0;
    use dpp::address_funds::fee_strategy::AddressFundsFeeStrategyStep;
    use dpp::address_funds::PlatformAddress;
    use dpp::block::epoch::Epoch;
    use dpp::platform_value::{Bytes32, Bytes36};
    use dpp::version::PlatformVersion;
    use std::collections::BTreeMap;

    fn make_minimal_action() -> PartiallyUseAssetLockAction {
        PartiallyUseAssetLockAction::V0(PartiallyUseAssetLockActionV0 {
            asset_lock_outpoint: Bytes36::new([0xCC; 36]),
            initial_credit_value: 10000,
            previous_transaction_hashes: vec![],
            asset_lock_script: vec![0x76, 0xA9, 0x14],
            remaining_credit_value: 7000,
            used_credits: 3000,
            user_fee_increase: 0,
            inputs_with_remaining_balance: None,
            fee_strategy: None,
        })
    }

    #[test]
    fn test_minimal_action_produces_add_system_credits_and_used_asset_lock() {
        let action = make_minimal_action();
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // Should produce exactly 2 ops: AddToSystemCredits and AddUsedAssetLock
        assert_eq!(ops.len(), 2);

        match &ops[0] {
            SystemOperation(SystemOperationType::AddToSystemCredits { amount }) => {
                assert_eq!(*amount, 3000);
            }
            other => panic!("expected AddToSystemCredits, got {:?}", other),
        }

        match &ops[1] {
            SystemOperation(SystemOperationType::AddUsedAssetLock {
                asset_lock_outpoint,
                ..
            }) => {
                assert_eq!(*asset_lock_outpoint, Bytes36::new([0xCC; 36]));
            }
            other => panic!("expected AddUsedAssetLock, got {:?}", other),
        }
    }

    #[test]
    fn test_action_with_inputs_and_fee_strategy_deduct_from_input() {
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([0xBB; 20]), (5_u32, 3000_u64));

        let action = PartiallyUseAssetLockAction::V0(PartiallyUseAssetLockActionV0 {
            asset_lock_outpoint: Bytes36::new([0xCC; 36]),
            initial_credit_value: 10000,
            previous_transaction_hashes: vec![],
            asset_lock_script: vec![0x76, 0xA9, 0x14],
            remaining_credit_value: 7000,
            used_credits: 3000,
            user_fee_increase: 0,
            inputs_with_remaining_balance: Some(inputs),
            fee_strategy: Some(vec![AddressFundsFeeStrategyStep::DeductFromInput(0)]),
        });
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // Should produce: SetBalanceToAddress, AddToSystemCredits, AddUsedAssetLock
        assert_eq!(ops.len(), 3);

        // First op should be the address funds deduction
        match &ops[0] {
            AddressFundsOperation(AddressFundsOperationType::SetBalanceToAddress {
                address,
                nonce,
                balance,
            }) => {
                assert_eq!(*address, PlatformAddress::P2pkh([0xBB; 20]));
                assert_eq!(*nonce, 5);
                // The input has 3000 balance, used_credits is 3000, so it deducts 3000
                assert_eq!(*balance, 0);
            }
            other => panic!("expected SetBalanceToAddress, got {:?}", other),
        }

        // Second should be AddToSystemCredits
        match &ops[1] {
            SystemOperation(SystemOperationType::AddToSystemCredits { amount }) => {
                assert_eq!(*amount, 3000);
            }
            other => panic!("expected AddToSystemCredits, got {:?}", other),
        }

        // Third should be AddUsedAssetLock
        assert!(matches!(
            &ops[2],
            SystemOperation(SystemOperationType::AddUsedAssetLock { .. })
        ));
    }

    #[test]
    fn test_action_with_max_usage_attempts_exceeded_burns_remaining() {
        let platform_version = PlatformVersion::latest();
        let max_attempts = platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .max_asset_lock_usage_attempts;

        // Create enough previous hashes to exceed max_asset_lock_usage_attempts
        let previous_hashes: Vec<Bytes32> = (0..max_attempts)
            .map(|i| Bytes32([i as u8; 32]))
            .collect();

        let action = PartiallyUseAssetLockAction::V0(PartiallyUseAssetLockActionV0 {
            asset_lock_outpoint: Bytes36::new([0xCC; 36]),
            initial_credit_value: 10000,
            previous_transaction_hashes: previous_hashes,
            asset_lock_script: vec![0x76, 0xA9, 0x14],
            remaining_credit_value: 7000,
            used_credits: 3000,
            user_fee_increase: 0,
            inputs_with_remaining_balance: None,
            fee_strategy: None,
        });
        let epoch = Epoch::new(0).unwrap();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        assert_eq!(ops.len(), 2);

        // When max attempts exceeded, the remaining credit value is burned (set to 0)
        // and previous_transaction_hashes is cleared
        match &ops[1] {
            SystemOperation(SystemOperationType::AddUsedAssetLock {
                asset_lock_value, ..
            }) => {
                use dpp::asset_lock::reduced_asset_lock_value::AssetLockValueGettersV0;
                // remaining_credit_value should be 0 (burned)
                assert_eq!(asset_lock_value.remaining_credit_value(), 0);
            }
            other => panic!("expected AddUsedAssetLock, got {:?}", other),
        }
    }

    #[test]
    fn test_action_with_zero_used_credits() {
        let action = PartiallyUseAssetLockAction::V0(PartiallyUseAssetLockActionV0 {
            asset_lock_outpoint: Bytes36::new([0xAA; 36]),
            initial_credit_value: 5000,
            previous_transaction_hashes: vec![],
            asset_lock_script: vec![],
            remaining_credit_value: 5000,
            used_credits: 0,
            user_fee_increase: 0,
            inputs_with_remaining_balance: None,
            fee_strategy: None,
        });
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        assert_eq!(ops.len(), 2);

        match &ops[0] {
            SystemOperation(SystemOperationType::AddToSystemCredits { amount }) => {
                assert_eq!(*amount, 0);
            }
            other => panic!("expected AddToSystemCredits, got {:?}", other),
        }
    }
}
