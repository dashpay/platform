#[cfg(test)]
mod tests {
    use crate::config::{PlatformConfig, PlatformTestConfig};
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use assert_matches::assert_matches;
    use dpp::address_funds::{
        AddressFundsFeeStrategy, AddressFundsFeeStrategyStep, AddressWitness, PlatformAddress,
    };
    use dpp::block::block_info::BlockInfo;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;
    use dpp::dash_to_credits;
    use dpp::dashcore::PublicKey;
    use dpp::platform_value::BinaryData;
    use dpp::prelude::AddressNonce;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::address_funds_transfer_transition::v0::AddressFundsTransferTransitionV0;
    use dpp::state_transition::address_funds_transfer_transition::AddressFundsTransferTransition;
    use dpp::state_transition::StateTransition;
    use platform_version::version::PlatformVersion;
    use std::collections::BTreeMap;
    // ==========================================
    // Helper Functions
    // ==========================================

    /// Helper function to create a platform address from a seed
    fn create_platform_address(seed: u8) -> PlatformAddress {
        let mut hash = [0u8; 20];
        hash[0] = seed;
        hash[19] = seed;
        PlatformAddress::P2pkh(hash)
    }

    /// Helper function to create a dummy P2PKH witness for testing
    fn create_dummy_witness() -> AddressWitness {
        // Create a valid compressed ECDSA public key (33 bytes)
        let mut pubkey_bytes = vec![0x02]; // compressed prefix
        pubkey_bytes.extend_from_slice(&[0x12; 32]); // x coordinate
        let public_key = PublicKey::from_slice(&pubkey_bytes).expect("valid public key");

        AddressWitness::P2pkh {
            signature: BinaryData::new(vec![0x30, 0x44, 0x02, 0x20]), // dummy signature
            public_key,
        }
    }

    /// Helper function to set up an address with balance and nonce in the drive
    fn setup_address_with_balance(
        platform: &mut crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        address: PlatformAddress,
        nonce: AddressNonce,
        balance: u64,
    ) {
        let platform_version = PlatformVersion::latest();
        let mut drive_operations = Vec::new();

        platform
            .drive
            .set_balance_to_address(
                address,
                nonce,
                balance,
                &mut None,
                &mut drive_operations,
                platform_version,
            )
            .expect("expected to set balance to address");

        platform
            .drive
            .apply_batch_low_level_drive_operations(
                None,
                None,
                drive_operations,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected to apply drive operations");
    }

    /// Create a simple AddressFundsTransferTransition with default fee strategy
    fn create_address_funds_transfer_transition(
        input_address: PlatformAddress,
        input_nonce: AddressNonce,
        input_amount: u64,
        output_address: PlatformAddress,
        output_amount: u64,
    ) -> StateTransition {
        let mut inputs = BTreeMap::new();
        inputs.insert(input_address, (input_nonce, input_amount));

        let mut outputs = BTreeMap::new();
        outputs.insert(output_address, output_amount);

        AddressFundsTransferTransition::V0(AddressFundsTransferTransitionV0 {
            inputs,
            outputs,
            fee_strategy: AddressFundsFeeStrategy::from(vec![
                AddressFundsFeeStrategyStep::DeductFromInput(0),
            ]),
            user_fee_increase: 0,
            input_witnesses: vec![create_dummy_witness()], // One witness per input
        })
        .into()
    }

    /// Create a raw AddressFundsTransferTransitionV0 for more control
    fn create_raw_transition(
        inputs: BTreeMap<PlatformAddress, (AddressNonce, u64)>,
        outputs: BTreeMap<PlatformAddress, u64>,
        fee_strategy: AddressFundsFeeStrategy,
        input_witnesses_count: usize,
    ) -> StateTransition {
        let witnesses: Vec<AddressWitness> = (0..input_witnesses_count)
            .map(|_| create_dummy_witness())
            .collect();
        AddressFundsTransferTransition::V0(AddressFundsTransferTransitionV0 {
            inputs,
            outputs,
            fee_strategy,
            user_fee_increase: 0,
            input_witnesses: witnesses,
        })
        .into()
    }

    // ==========================================
    // STRUCTURE VALIDATION TESTS
    // These test basic structure validation (BasicError)
    // Note: We must set up input addresses in drive first so state validation passes
    // ==========================================

    mod structure_validation {
        use super::*;

        #[test]
        fn test_no_inputs_returns_error() {
            let platform_version = PlatformVersion::latest();

            // No inputs case - doesn't need address setup since there are no inputs
            let inputs = BTreeMap::new(); // Empty inputs
            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(2), dash_to_credits!(0.1));

            let transition = create_raw_transition(
                inputs,
                outputs,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::ReduceOutput(0)]),
                0,
            );

            let result = transition.serialize_to_bytes();
            assert!(result.is_ok());

            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result.unwrap()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::TransitionNoInputsError(_))
                )]
            );
        }

        #[test]
        fn test_no_outputs_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let input_address = create_platform_address(1);
            // Set up the input address with sufficient balance
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.1)));
            let outputs = BTreeMap::new(); // Empty outputs

            let transition = create_raw_transition(
                inputs,
                outputs,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
                1,
            );

            let result = transition.serialize_to_bytes();
            assert!(result.is_ok());

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result.unwrap()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::TransitionNoOutputsError(_))
                )]
            );
        }

        #[test]
        fn test_too_many_inputs_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            // Create 17 inputs (max is 16) and set them up in drive
            let mut inputs = BTreeMap::new();
            for i in 0..17u8 {
                let addr = create_platform_address(i);
                setup_address_with_balance(&mut platform, addr, 0, dash_to_credits!(1.0));
                inputs.insert(addr, (1 as AddressNonce, dash_to_credits!(0.01)));
            }

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(100), dash_to_credits!(0.17));

            let transition = create_raw_transition(
                inputs,
                outputs,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
                17,
            );

            let result = transition.serialize_to_bytes();
            assert!(result.is_ok());

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result.unwrap()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::TransitionOverMaxInputsError(e))
                )] if e.actual_inputs() == 17 && e.max_inputs() == 16
            );
        }

        #[test]
        fn test_input_witness_count_mismatch_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let input_address_1 = create_platform_address(1);
            let input_address_2 = create_platform_address(2);
            setup_address_with_balance(&mut platform, input_address_1, 0, dash_to_credits!(1.0));
            setup_address_with_balance(&mut platform, input_address_2, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address_1, (1 as AddressNonce, dash_to_credits!(0.1)));
            inputs.insert(input_address_2, (1 as AddressNonce, dash_to_credits!(0.1)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(3), dash_to_credits!(0.2));

            // Only 1 witness for 2 inputs
            let transition = create_raw_transition(
                inputs,
                outputs,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
                1, // Mismatch: 1 witness for 2 inputs
            );

            let result = transition.serialize_to_bytes();
            assert!(result.is_ok());

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result.unwrap()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::InputWitnessCountMismatchError(_))
                )]
            );
        }

        #[test]
        fn test_output_address_also_input_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let same_address = create_platform_address(1);
            setup_address_with_balance(&mut platform, same_address, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(same_address, (1 as AddressNonce, dash_to_credits!(0.1)));

            let mut outputs = BTreeMap::new();
            outputs.insert(same_address, dash_to_credits!(0.1)); // Same address as input

            let transition = create_raw_transition(
                inputs,
                outputs,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
                1,
            );

            let result = transition.serialize_to_bytes();
            assert!(result.is_ok());

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result.unwrap()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::OutputAddressAlsoInputError(_))
                )]
            );
        }

        #[test]
        fn test_empty_fee_strategy_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let input_address = create_platform_address(1);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.1)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(2), dash_to_credits!(0.1));

            let transition = create_raw_transition(
                inputs,
                outputs,
                AddressFundsFeeStrategy::from(vec![]), // Empty fee strategy
                1,
            );

            let result = transition.serialize_to_bytes();
            assert!(result.is_ok());

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result.unwrap()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::FeeStrategyEmptyError(_))
                )]
            );
        }

        #[test]
        fn test_fee_strategy_too_many_steps_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let input_address = create_platform_address(1);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.1)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(2), dash_to_credits!(0.1));

            // 5 fee strategy steps (max is 4)
            let transition = create_raw_transition(
                inputs,
                outputs,
                AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                    AddressFundsFeeStrategyStep::ReduceOutput(0),
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                    AddressFundsFeeStrategyStep::ReduceOutput(0),
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                1,
            );

            let result = transition.serialize_to_bytes();
            assert!(result.is_ok());

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result.unwrap()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::FeeStrategyTooManyStepsError(_))
                )]
            );
        }

        #[test]
        fn test_fee_strategy_duplicate_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let input_address = create_platform_address(1);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.1)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(2), dash_to_credits!(0.1));

            // Duplicate fee strategy steps
            let transition = create_raw_transition(
                inputs,
                outputs,
                AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                    AddressFundsFeeStrategyStep::DeductFromInput(0), // Duplicate
                ]),
                1,
            );

            let result = transition.serialize_to_bytes();
            assert!(result.is_ok());

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result.unwrap()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::FeeStrategyDuplicateError(_))
                )]
            );
        }

        #[test]
        fn test_fee_strategy_input_index_out_of_bounds_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let input_address = create_platform_address(1);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.1)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(2), dash_to_credits!(0.1));

            // Fee strategy references input index 5, but we only have 1 input
            let transition = create_raw_transition(
                inputs,
                outputs,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    5,
                )]),
                1,
            );

            let result = transition.serialize_to_bytes();
            assert!(result.is_ok());

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result.unwrap()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::FeeStrategyIndexOutOfBoundsError(_))
                )]
            );
        }

        #[test]
        fn test_fee_strategy_output_index_out_of_bounds_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let input_address = create_platform_address(1);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.1)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(2), dash_to_credits!(0.1));

            // Fee strategy references output index 5, but we only have 1 output
            let transition = create_raw_transition(
                inputs,
                outputs,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::ReduceOutput(5)]),
                1,
            );

            let result = transition.serialize_to_bytes();
            assert!(result.is_ok());

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result.unwrap()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::FeeStrategyIndexOutOfBoundsError(_))
                )]
            );
        }

        #[test]
        fn test_input_below_minimum_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let input_address = create_platform_address(1);
            // Set up with more than the minimum in drive, but transition requests below minimum
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            // Min input amount is 100,000 credits
            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, 50_000)); // Below minimum

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(2), 50_000);

            let transition = create_raw_transition(
                inputs,
                outputs,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
                1,
            );

            let result = transition.serialize_to_bytes();
            assert!(result.is_ok());

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result.unwrap()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::InputBelowMinimumError(_))
                )]
            );
        }

        #[test]
        fn test_output_below_minimum_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let input_address = create_platform_address(1);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            // Min output amount is 500,000 credits
            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, 600_000));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(2), 100_000); // Below minimum (500,000)

            let transition = create_raw_transition(
                inputs,
                outputs,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
                1,
            );

            let result = transition.serialize_to_bytes();
            assert!(result.is_ok());

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result.unwrap()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::OutputBelowMinimumError(_))
                )]
            );
        }

        #[test]
        fn test_input_output_balance_mismatch_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let input_address = create_platform_address(1);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(2.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(1.0)));

            let mut outputs = BTreeMap::new();
            outputs.insert(create_platform_address(2), dash_to_credits!(0.5)); // Doesn't match input

            let transition = create_raw_transition(
                inputs,
                outputs,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
                1,
            );

            let result = transition.serialize_to_bytes();
            assert!(result.is_ok());

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result.unwrap()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::InputOutputBalanceMismatchError(_))
                )]
            );
        }
    }

    // ==========================================
    // STATE VALIDATION TESTS
    // These test address balance and nonce validation (StateError)
    // ==========================================

    mod state_validation {
        use super::*;

        #[test]
        fn test_address_does_not_exist_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_state = platform.state.load();

            // Input address does not exist in state
            let input_address = create_platform_address(1);
            let output_address = create_platform_address(2);

            let transition = create_address_funds_transfer_transition(
                input_address,
                1,
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
            );

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::AddressDoesNotExistError(e))
                )] if e.address() == &input_address
            );
        }

        #[test]
        fn test_wrong_nonce_too_high_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let input_address = create_platform_address(1);
            let output_address = create_platform_address(2);

            // Set up address with nonce 0
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let platform_state = platform.state.load();

            // Provide nonce 5 (should be 1)
            let transition = create_address_funds_transfer_transition(
                input_address,
                5, // Wrong nonce - expected 1
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
            );

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::AddressInvalidNonceError(e))
                )] if e.address() == &input_address && e.provided_nonce() == 5 && e.expected_nonce() == 1
            );
        }

        #[test]
        fn test_wrong_nonce_too_low_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let input_address = create_platform_address(1);
            let output_address = create_platform_address(2);

            // Set up address with nonce 5 (next valid nonce is 6)
            setup_address_with_balance(&mut platform, input_address, 5, dash_to_credits!(1.0));

            let platform_state = platform.state.load();

            // Provide nonce 3 (should be 6)
            let transition = create_address_funds_transfer_transition(
                input_address,
                3, // Too low - expected 6
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
            );

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::AddressInvalidNonceError(e))
                )] if e.address() == &input_address && e.provided_nonce() == 3 && e.expected_nonce() == 6
            );
        }

        #[test]
        fn test_max_nonce_reached_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let input_address = create_platform_address(1);
            let output_address = create_platform_address(2);

            // Set up address with max nonce (u32::MAX)
            let max_nonce: AddressNonce = u32::MAX;
            setup_address_with_balance(
                &mut platform,
                input_address,
                max_nonce,
                dash_to_credits!(1.0),
            );

            let platform_state = platform.state.load();

            // Any nonce will fail because max nonce can't be incremented
            let transition = create_address_funds_transfer_transition(
                input_address,
                0, // Would wrap around
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
            );

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::AddressInvalidNonceError(_))
                )]
            );
        }

        #[test]
        fn test_insufficient_balance_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let input_address = create_platform_address(1);
            let output_address = create_platform_address(2);

            // Set up address with small balance
            let available_balance = dash_to_credits!(0.05);
            setup_address_with_balance(&mut platform, input_address, 0, available_balance);

            let platform_state = platform.state.load();

            // Try to transfer more than available
            let requested_amount = dash_to_credits!(0.1);
            let transition = create_address_funds_transfer_transition(
                input_address,
                1,
                requested_amount,
                output_address,
                requested_amount,
            );

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::AddressNotEnoughFundsError(e))
                )] if e.address() == &input_address
                    && e.balance() == available_balance
                    && e.required_balance() == requested_amount
            );
        }

        #[test]
        fn test_multiple_inputs_one_missing_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let input_address_1 = create_platform_address(1);
            let input_address_2 = create_platform_address(2); // Won't exist
            let output_address = create_platform_address(3);

            // Only set up the first address
            setup_address_with_balance(&mut platform, input_address_1, 0, dash_to_credits!(1.0));

            let platform_state = platform.state.load();

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address_1, (1 as AddressNonce, dash_to_credits!(0.1)));
            inputs.insert(input_address_2, (1 as AddressNonce, dash_to_credits!(0.1)));

            let mut outputs = BTreeMap::new();
            outputs.insert(output_address, dash_to_credits!(0.2));

            let transition: StateTransition =
                AddressFundsTransferTransition::V0(AddressFundsTransferTransitionV0 {
                    inputs,
                    outputs,
                    fee_strategy: AddressFundsFeeStrategy::from(vec![
                        AddressFundsFeeStrategyStep::DeductFromInput(0),
                    ]),
                    user_fee_increase: 0,
                    input_witnesses: vec![create_dummy_witness(), create_dummy_witness()],
                })
                .into();

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::AddressDoesNotExistError(_))
                )]
            );
        }
    }

    // ==========================================
    // SUCCESS TESTS
    // These test successful transfers
    // ==========================================

    mod success {
        use super::*;

        #[test]
        fn test_simple_transfer_success() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let input_address = create_platform_address(1);
            let output_address = create_platform_address(2);

            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let platform_state = platform.state.load();

            let transition = create_address_funds_transfer_transition(
                input_address,
                1, // Correct nonce
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
            );

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(..)]
            );
        }

        #[test]
        fn test_transfer_with_non_zero_starting_nonce_success() {
            let platform_version = PlatformVersion::latest();
            let platform_config = PlatformConfig {
                testing_configs: PlatformTestConfig {
                    disable_instant_lock_signature_verification: true,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut platform = TestPlatformBuilder::new()
                .with_config(platform_config)
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let input_address = create_platform_address(1);
            let output_address = create_platform_address(2);

            // Set up with nonce 5
            let current_nonce: AddressNonce = 5;
            setup_address_with_balance(
                &mut platform,
                input_address,
                current_nonce,
                dash_to_credits!(1.0),
            );

            let platform_state = platform.state.load();

            // Use nonce 6 (current + 1)
            let transition = create_address_funds_transfer_transition(
                input_address,
                current_nonce + 1,
                dash_to_credits!(0.1),
                output_address,
                dash_to_credits!(0.1),
            );

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("expected to serialize transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![transition_bytes],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(..)]
            );
        }
    }
}
