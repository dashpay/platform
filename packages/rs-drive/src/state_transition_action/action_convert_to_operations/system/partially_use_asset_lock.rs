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

                let max_usage_attempts_reached = self.previous_transaction_hashes_ref().len()
                    as u16
                    >= platform_version
                        .drive_abci
                        .validation_and_processing
                        .state_transitions
                        .max_asset_lock_usage_attempts;

                let previous_transaction_hashes = if max_usage_attempts_reached {
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

                // Tracks how much of `used_credits` was paid from existing address balances. Those
                // credits already count toward total_credits_in_platform, so they must NOT be added
                // again via AddToSystemCredits (only asset-lock-sourced credits are new money).
                let mut total_deducted_from_inputs = 0u64;

                // If we have inputs and a fee strategy, deduct fees from inputs first
                // Note: remaining_credit_value was already pre-computed to deduct ALL used_credits
                // from the asset lock. Here we restore the portion that's covered by inputs.
                if let Some((inputs, fee_strategy)) = inputs_and_strategy {
                    let inputs_ordered: Vec<_> = inputs.iter().collect();
                    let mut remaining_fee = used_credits;

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

                // The input-fee restoration above must not revive a burned remainder: once the max
                // usage attempts are reached the asset lock is fully consumed regardless of who paid
                // the fee, so keep the remainder at 0.
                if max_usage_attempts_reached {
                    remaining_credit_value = 0;
                }

                // Only the portion of the fee sourced from the asset lock is new money entering the
                // platform. The portion paid from existing address balances is already counted in
                // the system credit total, so adding the full `used_credits` would count it twice.
                let new_system_credits = used_credits.saturating_sub(total_deducted_from_inputs);

                // Add system credits operation
                drive_operations.push(SystemOperation(SystemOperationType::AddToSystemCredits {
                    amount: new_system_credits,
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
    use dpp::asset_lock::reduced_asset_lock_value::AssetLockValueGettersV0;
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
                asset_lock_value,
            }) => {
                assert_eq!(*asset_lock_outpoint, Bytes36::new([0xCC; 36]));
                // Verify the full payload of the asset lock value
                assert_eq!(asset_lock_value.initial_credit_value(), 10000);
                assert_eq!(asset_lock_value.remaining_credit_value(), 7000);
                assert_eq!(asset_lock_value.tx_out_script(), &vec![0x76_u8, 0xA9, 0x14]);
                assert!(asset_lock_value.used_tags_ref().is_empty());
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

        // Second should be AddToSystemCredits. The entire fee (3000) was covered by the input
        // address (existing credits already counted in total_credits_in_platform), so NO new money
        // entered from the asset lock -> AddToSystemCredits must be 0 (credit-conservation fix).
        match &ops[1] {
            SystemOperation(SystemOperationType::AddToSystemCredits { amount }) => {
                assert_eq!(*amount, 0);
            }
            other => panic!("expected AddToSystemCredits, got {:?}", other),
        }

        // Third should be AddUsedAssetLock with restored remaining_credit_value
        match &ops[2] {
            SystemOperation(SystemOperationType::AddUsedAssetLock {
                asset_lock_outpoint,
                asset_lock_value,
            }) => {
                assert_eq!(*asset_lock_outpoint, Bytes36::new([0xCC; 36]));
                assert_eq!(asset_lock_value.initial_credit_value(), 10000);
                // remaining_credit_value was 7000 (pre-computed deducting all used_credits from
                // asset lock), but 3000 was covered by the input, so it gets restored:
                // 7000 + 3000 = 10000
                assert_eq!(asset_lock_value.remaining_credit_value(), 10000);
                assert_eq!(asset_lock_value.tx_out_script(), &vec![0x76_u8, 0xA9, 0x14]);
                assert!(asset_lock_value.used_tags_ref().is_empty());
            }
            other => panic!("expected AddUsedAssetLock, got {:?}", other),
        }
    }

    #[test]
    fn test_action_with_max_usage_attempts_exceeded_burns_remaining() {
        let platform_version = PlatformVersion::latest();
        let max_attempts = platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .max_asset_lock_usage_attempts;

        // Create enough previous hashes to reach max_asset_lock_usage_attempts
        let previous_hashes: Vec<Bytes32> =
            (0..max_attempts).map(|i| Bytes32([i as u8; 32])).collect();

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

        // When max attempts reached, the remaining credit value is burned (set to 0)
        // and previous_transaction_hashes is cleared
        match &ops[1] {
            SystemOperation(SystemOperationType::AddUsedAssetLock {
                asset_lock_outpoint,
                asset_lock_value,
            }) => {
                assert_eq!(*asset_lock_outpoint, Bytes36::new([0xCC; 36]));
                assert_eq!(asset_lock_value.initial_credit_value(), 10000);
                // remaining_credit_value should be 0 (burned)
                assert_eq!(asset_lock_value.remaining_credit_value(), 0);
                // previous_transaction_hashes should be cleared (empty used_tags)
                assert!(asset_lock_value.used_tags_ref().is_empty());
                assert_eq!(asset_lock_value.tx_out_script(), &vec![0x76_u8, 0xA9, 0x14]);
            }
            other => panic!("expected AddUsedAssetLock, got {:?}", other),
        }

        // Also verify AddToSystemCredits still has the correct amount
        match &ops[0] {
            SystemOperation(SystemOperationType::AddToSystemCredits { amount }) => {
                assert_eq!(*amount, 3000);
            }
            other => panic!("expected AddToSystemCredits, got {:?}", other),
        }
    }

    #[test]
    fn test_action_with_max_usage_attempts_exceeded_well_above_max() {
        let platform_version = PlatformVersion::latest();
        let max_attempts = platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .max_asset_lock_usage_attempts;

        // Create hashes well above the max to confirm the burn still happens
        let previous_hashes: Vec<Bytes32> = (0..(max_attempts + 5))
            .map(|i| Bytes32([i as u8; 32]))
            .collect();

        let action = PartiallyUseAssetLockAction::V0(PartiallyUseAssetLockActionV0 {
            asset_lock_outpoint: Bytes36::new([0xEE; 36]),
            initial_credit_value: 50000,
            previous_transaction_hashes: previous_hashes,
            asset_lock_script: vec![0xAA],
            remaining_credit_value: 40000,
            used_credits: 10000,
            user_fee_increase: 0,
            inputs_with_remaining_balance: None,
            fee_strategy: None,
        });
        let epoch = Epoch::new(0).unwrap();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        assert_eq!(ops.len(), 2);

        match &ops[1] {
            SystemOperation(SystemOperationType::AddUsedAssetLock {
                asset_lock_value, ..
            }) => {
                // Still burned even when well above max
                assert_eq!(asset_lock_value.remaining_credit_value(), 0);
                assert!(asset_lock_value.used_tags_ref().is_empty());
            }
            other => panic!("expected AddUsedAssetLock, got {:?}", other),
        }
    }

    #[test]
    fn test_action_max_usage_attempts_with_inputs_still_burns_remainder() {
        // When the max usage attempts have been reached, the asset-lock remainder must be burned to
        // 0 (fully consumed) even if the fee is paid from address inputs. The input-fee restoration
        // must not revive the burned remainder.
        let platform_version = PlatformVersion::latest();
        let max_attempts = platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .max_asset_lock_usage_attempts;

        let previous_hashes: Vec<Bytes32> =
            (0..max_attempts).map(|i| Bytes32([i as u8; 32])).collect();

        let mut inputs = BTreeMap::new();
        // Input fully covers the fee (used_credits).
        inputs.insert(PlatformAddress::P2pkh([0xBB; 20]), (7_u32, 10000_u64));

        let action = PartiallyUseAssetLockAction::V0(PartiallyUseAssetLockActionV0 {
            asset_lock_outpoint: Bytes36::new([0xCC; 36]),
            initial_credit_value: 50000,
            previous_transaction_hashes: previous_hashes,
            asset_lock_script: vec![0x76, 0xA9, 0x14],
            remaining_credit_value: 40000,
            used_credits: 10000,
            user_fee_increase: 0,
            inputs_with_remaining_balance: Some(inputs),
            fee_strategy: Some(vec![AddressFundsFeeStrategyStep::DeductFromInput(0)]),
        });
        let epoch = Epoch::new(0).unwrap();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // SetBalanceToAddress, AddToSystemCredits, AddUsedAssetLock
        assert_eq!(ops.len(), 3);

        // The whole fee was covered by the input, so no new system credits are added.
        match &ops[1] {
            SystemOperation(SystemOperationType::AddToSystemCredits { amount }) => {
                assert_eq!(*amount, 0);
            }
            other => panic!("expected AddToSystemCredits, got {:?}", other),
        }

        // The remainder must stay burned (0) -> fully consumed, despite the input paying the fee.
        match &ops[2] {
            SystemOperation(SystemOperationType::AddUsedAssetLock {
                asset_lock_value, ..
            }) => {
                assert_eq!(
                    asset_lock_value.remaining_credit_value(),
                    0,
                    "max-attempt burn must not be revived by the input-fee restoration"
                );
                assert!(asset_lock_value.used_tags_ref().is_empty());
            }
            other => panic!("expected AddUsedAssetLock, got {:?}", other),
        }
    }

    #[test]
    fn test_action_below_max_usage_attempts_preserves_hashes() {
        let platform_version = PlatformVersion::latest();
        let max_attempts = platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .max_asset_lock_usage_attempts;

        // Create one fewer than the max -- should NOT burn
        let num_hashes = max_attempts.saturating_sub(1);
        let previous_hashes: Vec<Bytes32> =
            (0..num_hashes).map(|i| Bytes32([i as u8; 32])).collect();

        let action = PartiallyUseAssetLockAction::V0(PartiallyUseAssetLockActionV0 {
            asset_lock_outpoint: Bytes36::new([0xDD; 36]),
            initial_credit_value: 10000,
            previous_transaction_hashes: previous_hashes.clone(),
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

        match &ops[1] {
            SystemOperation(SystemOperationType::AddUsedAssetLock {
                asset_lock_outpoint,
                asset_lock_value,
            }) => {
                assert_eq!(*asset_lock_outpoint, Bytes36::new([0xDD; 36]));
                // remaining_credit_value should be preserved (not burned)
                assert_eq!(asset_lock_value.remaining_credit_value(), 7000);
                // previous_transaction_hashes should be preserved as used_tags
                assert_eq!(asset_lock_value.used_tags_ref().len(), num_hashes as usize);
                for (i, tag) in asset_lock_value.used_tags_ref().iter().enumerate() {
                    assert_eq!(*tag, Bytes32([i as u8; 32]));
                }
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

        match &ops[1] {
            SystemOperation(SystemOperationType::AddUsedAssetLock {
                asset_lock_outpoint,
                asset_lock_value,
            }) => {
                assert_eq!(*asset_lock_outpoint, Bytes36::new([0xAA; 36]));
                assert_eq!(asset_lock_value.initial_credit_value(), 5000);
                assert_eq!(asset_lock_value.remaining_credit_value(), 5000);
                assert!(asset_lock_value.tx_out_script().is_empty());
                assert!(asset_lock_value.used_tags_ref().is_empty());
            }
            other => panic!("expected AddUsedAssetLock, got {:?}", other),
        }
    }

    #[test]
    fn test_unknown_version_returns_error() {
        let action = make_minimal_action();
        let epoch = Epoch::new(0).unwrap();

        // Clone the latest platform version and set the partially_use_asset_lock
        // version to an unknown value to exercise the error branch
        let mut platform_version = PlatformVersion::latest().clone();
        platform_version
            .drive
            .methods
            .state_transitions
            .convert_to_high_level_operations
            .partially_use_asset_lock = 255;

        let result = action.into_high_level_drive_operations(&epoch, &platform_version);

        assert!(result.is_err(), "expected an error for unknown version");

        match result.unwrap_err() {
            Error::Drive(DriveError::UnknownVersionMismatch {
                method,
                known_versions,
                received,
            }) => {
                assert_eq!(
                    method,
                    "PartiallyUseAssetLockAction::into_high_level_drive_operations"
                );
                assert_eq!(known_versions, vec![0]);
                assert_eq!(received, 255);
            }
            other => panic!(
                "expected DriveError::UnknownVersionMismatch, got {:?}",
                other
            ),
        }
    }

    #[test]
    fn test_action_with_partial_input_deduction() {
        // Test where the input balance is less than used_credits,
        // so only a partial amount is deducted from the input
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([0xAA; 20]), (1_u32, 1000_u64));

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

        assert_eq!(ops.len(), 3);

        // Input has 1000 balance but used_credits is 3000,
        // so only 1000 is deducted from the input
        match &ops[0] {
            AddressFundsOperation(AddressFundsOperationType::SetBalanceToAddress {
                address,
                nonce,
                balance,
            }) => {
                assert_eq!(*address, PlatformAddress::P2pkh([0xAA; 20]));
                assert_eq!(*nonce, 1);
                assert_eq!(*balance, 0); // 1000 - 1000 = 0
            }
            other => panic!("expected SetBalanceToAddress, got {:?}", other),
        }

        // The remaining_credit_value gets 1000 added back (the portion covered by input)
        match &ops[2] {
            SystemOperation(SystemOperationType::AddUsedAssetLock {
                asset_lock_value, ..
            }) => {
                // 7000 (pre-computed) + 1000 (restored from input) = 8000
                assert_eq!(asset_lock_value.remaining_credit_value(), 8000);
            }
            other => panic!("expected AddUsedAssetLock, got {:?}", other),
        }
    }

    #[test]
    fn test_action_with_reduce_output_step_is_skipped() {
        // ReduceOutput steps should be skipped for partial use
        let mut inputs = BTreeMap::new();
        inputs.insert(PlatformAddress::P2pkh([0xBB; 20]), (2_u32, 5000_u64));

        let action = PartiallyUseAssetLockAction::V0(PartiallyUseAssetLockActionV0 {
            asset_lock_outpoint: Bytes36::new([0xCC; 36]),
            initial_credit_value: 10000,
            previous_transaction_hashes: vec![],
            asset_lock_script: vec![0x76, 0xA9, 0x14],
            remaining_credit_value: 7000,
            used_credits: 3000,
            user_fee_increase: 0,
            inputs_with_remaining_balance: Some(inputs),
            fee_strategy: Some(vec![
                AddressFundsFeeStrategyStep::ReduceOutput(0),
                AddressFundsFeeStrategyStep::DeductFromInput(0),
            ]),
        });
        let epoch = Epoch::new(0).unwrap();
        let platform_version = PlatformVersion::latest();

        let ops = action
            .into_high_level_drive_operations(&epoch, platform_version)
            .expect("expected operations");

        // Should produce 3 ops: SetBalanceToAddress (from DeductFromInput), AddToSystemCredits,
        // AddUsedAssetLock. The ReduceOutput step is skipped entirely.
        assert_eq!(ops.len(), 3);

        match &ops[0] {
            AddressFundsOperation(AddressFundsOperationType::SetBalanceToAddress {
                address,
                nonce,
                balance,
            }) => {
                assert_eq!(*address, PlatformAddress::P2pkh([0xBB; 20]));
                assert_eq!(*nonce, 2);
                // 5000 - 3000 = 2000
                assert_eq!(*balance, 2000);
            }
            other => panic!("expected SetBalanceToAddress, got {:?}", other),
        }

        match &ops[2] {
            SystemOperation(SystemOperationType::AddUsedAssetLock {
                asset_lock_value, ..
            }) => {
                // 7000 + 3000 (full used_credits covered by input) = 10000
                assert_eq!(asset_lock_value.remaining_credit_value(), 10000);
            }
            other => panic!("expected AddUsedAssetLock, got {:?}", other),
        }
    }
}
