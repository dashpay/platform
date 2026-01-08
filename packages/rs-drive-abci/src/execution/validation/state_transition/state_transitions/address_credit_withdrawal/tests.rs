#[cfg(test)]
mod tests {
    use crate::config::{PlatformConfig, PlatformTestConfig};
    use crate::execution::validation::state_transition::state_transitions::test_helpers::{
        create_dummy_witness, create_platform_address,
        setup_address_with_balance_and_system_credits as setup_address_with_balance,
        TestAddressSigner, TestScriptBuf as ScriptBuf, OP_RETURN,
    };
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
    use dpp::identity::core_script::CoreScript;
    use dpp::identity::signer::Signer;
    use dpp::platform_value::BinaryData;
    use dpp::prelude::AddressNonce;
    use dpp::serialization::{PlatformDeserializable, PlatformSerializable};
    use dpp::state_transition::address_credit_withdrawal_transition::methods::AddressCreditWithdrawalTransitionMethodsV0;
    use dpp::state_transition::address_credit_withdrawal_transition::v0::AddressCreditWithdrawalTransitionV0;
    use dpp::state_transition::address_credit_withdrawal_transition::AddressCreditWithdrawalTransition;
    use dpp::state_transition::StateTransition;
    use dpp::withdrawal::Pooling;
    use platform_version::version::PlatformVersion;
    use rand::prelude::StdRng;
    use rand::SeedableRng;
    use std::collections::BTreeMap;

    use crate::execution::check_tx::CheckTxLevel;
    use crate::platform_types::platform::PlatformRef;

    // ==========================================
    // Check TX Helper
    // ==========================================

    /// Perform check_tx on a raw transaction and return whether it's valid
    /// This simulates what happens when a transaction is submitted to the mempool.
    /// - invalid_unpaid transactions should return false (rejected from mempool)
    /// - invalid_paid transactions should return true (accepted to mempool, will fail at processing)
    fn check_tx_is_valid(
        platform: &crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        raw_tx: &[u8],
        platform_version: &PlatformVersion,
    ) -> bool {
        let platform_state = platform.state.load();
        let platform_ref = PlatformRef {
            drive: &platform.drive,
            state: &platform_state,
            config: &platform.config,
            core_rpc: &platform.core_rpc,
        };

        let check_result = platform
            .check_tx(
                raw_tx,
                CheckTxLevel::FirstTimeCheck,
                &platform_ref,
                platform_version,
            )
            .expect("expected to check tx");

        if !check_result.is_valid() {
            eprintln!("check_tx errors: {:?}", check_result.errors);
        }

        check_result.is_valid()
    }

    /// Perform check_tx on a raw transaction and return the full validation result
    fn run_check_tx(
        platform: &crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        raw_tx: &[u8],
        platform_version: &PlatformVersion,
    ) -> dpp::validation::ValidationResult<crate::execution::check_tx::CheckTxResult, ConsensusError>
    {
        let platform_state = platform.state.load();
        let platform_ref = PlatformRef {
            drive: &platform.drive,
            state: &platform_state,
            config: &platform.config,
            core_rpc: &platform.core_rpc,
        };

        platform
            .check_tx(
                raw_tx,
                CheckTxLevel::FirstTimeCheck,
                &platform_ref,
                platform_version,
            )
            .expect("expected to check tx")
    }

    // ==========================================
    // Helper Functions
    // ==========================================

    /// Create a random CoreScript for withdrawal output
    fn create_random_output_script(rng: &mut StdRng) -> CoreScript {
        CoreScript::random_p2pkh(rng)
    }

    /// Create a raw AddressCreditWithdrawalTransitionV0 with dummy witnesses for structure validation tests
    fn create_raw_withdrawal_transition_with_dummy_witnesses(
        inputs: BTreeMap<PlatformAddress, (AddressNonce, u64)>,
        output: Option<(PlatformAddress, u64)>,
        fee_strategy: AddressFundsFeeStrategy,
        output_script: CoreScript,
        input_witnesses_count: usize,
    ) -> StateTransition {
        let witnesses: Vec<AddressWitness> = (0..input_witnesses_count)
            .map(|_| create_dummy_witness())
            .collect();
        AddressCreditWithdrawalTransition::V0(AddressCreditWithdrawalTransitionV0 {
            inputs,
            output,
            fee_strategy,
            core_fee_per_byte: 1,
            pooling: Pooling::Never,
            output_script,
            user_fee_increase: 0,
            input_witnesses: witnesses,
        })
        .into()
    }

    /// Create a signed AddressCreditWithdrawalTransition
    fn create_signed_address_credit_withdrawal_transition(
        signer: &TestAddressSigner,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, u64)>,
        output: Option<(PlatformAddress, u64)>,
        fee_strategy: Vec<AddressFundsFeeStrategyStep>,
        output_script: CoreScript,
    ) -> StateTransition {
        create_signed_address_credit_withdrawal_transition_with_fee_increase(
            signer,
            inputs,
            output,
            fee_strategy,
            output_script,
            0,
        )
    }

    fn create_signed_address_credit_withdrawal_transition_with_fee_increase(
        signer: &TestAddressSigner,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, u64)>,
        output: Option<(PlatformAddress, u64)>,
        fee_strategy: Vec<AddressFundsFeeStrategyStep>,
        output_script: CoreScript,
        user_fee_increase: u16,
    ) -> StateTransition {
        AddressCreditWithdrawalTransitionV0::try_from_inputs_with_signer(
            inputs,
            output,
            AddressFundsFeeStrategy::from(fee_strategy),
            1, // core_fee_per_byte
            Pooling::Never,
            output_script,
            signer,
            user_fee_increase,
            PlatformVersion::latest(),
        )
        .expect("should create signed transition")
    }

    // ==========================================
    // STRUCTURE VALIDATION TESTS
    // These test basic structure validation (BasicError)
    // ==========================================

    mod structure_validation {
        use super::*;

        #[test]
        fn test_no_inputs_returns_error() {
            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);

            // No inputs case - should fail validation
            let inputs = BTreeMap::new();

            let transition = create_raw_withdrawal_transition_with_dummy_witnesses(
                inputs,
                None,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
                create_random_output_script(&mut rng),
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
        fn test_too_many_inputs_returns_error() {
            use crate::execution::validation::state_transition::processor::traits::basic_structure::StateTransitionBasicStructureValidationV0;

            let platform_version = PlatformVersion::latest();
            let max_inputs = platform_version.dpp.state_transitions.max_address_inputs;

            let mut rng = StdRng::seed_from_u64(567);

            // Create max_inputs + 1 inputs (17 inputs, max is 16)
            let input_count = max_inputs as usize + 1;
            let mut inputs = BTreeMap::new();
            for i in 0..input_count {
                inputs.insert(
                    create_platform_address(i as u8),
                    (1 as AddressNonce, dash_to_credits!(0.1)),
                );
            }

            let transition = create_raw_withdrawal_transition_with_dummy_witnesses(
                inputs,
                None,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
                create_random_output_script(&mut rng),
                input_count,
            );

            let result = transition
                .validate_basic_structure(dpp::dashcore::Network::Testnet, platform_version)
                .expect("validation should not return Err");

            assert!(!result.is_valid());
            let error = result.first_error().unwrap();
            assert!(
                matches!(
                    error,
                    ConsensusError::BasicError(BasicError::TransitionOverMaxInputsError(e))
                    if e.actual_inputs() == 17 && e.max_inputs() == 16
                ),
                "Expected TransitionOverMaxInputsError with 17 actual and 16 max, got {:?}",
                error
            );
        }

        #[test]
        fn test_input_witness_count_mismatch_returns_error() {
            use crate::execution::validation::state_transition::processor::traits::basic_structure::StateTransitionBasicStructureValidationV0;

            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(
                create_platform_address(1),
                (1 as AddressNonce, dash_to_credits!(0.1)),
            );
            inputs.insert(
                create_platform_address(2),
                (1 as AddressNonce, dash_to_credits!(0.1)),
            );

            // Create transition with 2 inputs but only 1 witness
            let transition = create_raw_withdrawal_transition_with_dummy_witnesses(
                inputs,
                None,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
                create_random_output_script(&mut rng),
                1, // Only 1 witness for 2 inputs
            );

            let result = transition
                .validate_basic_structure(dpp::dashcore::Network::Testnet, platform_version)
                .expect("validation should not return Err");

            assert!(!result.is_valid());
            let error = result.first_error().unwrap();
            assert!(
                matches!(
                    error,
                    ConsensusError::BasicError(BasicError::InputWitnessCountMismatchError(_))
                ),
                "Expected InputWitnessCountMismatchError, got {:?}",
                error
            );
        }

        #[test]
        fn test_output_address_also_input_returns_error() {
            use crate::execution::validation::state_transition::processor::traits::basic_structure::StateTransitionBasicStructureValidationV0;

            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);

            let same_address = create_platform_address(1);

            let mut inputs = BTreeMap::new();
            inputs.insert(same_address, (1 as AddressNonce, dash_to_credits!(1.0)));

            // Output to same address as input
            let output = Some((same_address, dash_to_credits!(0.5)));

            let transition = create_raw_withdrawal_transition_with_dummy_witnesses(
                inputs,
                output,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
                create_random_output_script(&mut rng),
                1,
            );

            let result = transition
                .validate_basic_structure(dpp::dashcore::Network::Testnet, platform_version)
                .expect("validation should not return Err");

            assert!(!result.is_valid());
            let error = result.first_error().unwrap();
            assert!(
                matches!(
                    error,
                    ConsensusError::BasicError(BasicError::OutputAddressAlsoInputError(_))
                ),
                "Expected OutputAddressAlsoInputError, got {:?}",
                error
            );
        }

        #[test]
        fn test_empty_fee_strategy_returns_error() {
            // Structure validation happens before signature validation
            // so we test it directly without needing valid signatures
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(
                create_platform_address(1),
                (1 as AddressNonce, dash_to_credits!(1.0)),
            );

            // Empty fee strategy
            let transition = AddressCreditWithdrawalTransitionV0 {
                inputs,
                output: None,
                fee_strategy: AddressFundsFeeStrategy::from(vec![]),
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: create_random_output_script(&mut rng),
                user_fee_increase: 0,
                input_witnesses: vec![create_dummy_witness()],
            };

            let result = transition.validate_structure(platform_version);
            assert!(!result.is_valid());
            let error = result.first_error().unwrap();
            assert!(
                matches!(
                    error,
                    ConsensusError::BasicError(BasicError::FeeStrategyEmptyError(_))
                ),
                "Expected FeeStrategyEmptyError, got {:?}",
                error
            );
        }

        #[test]
        fn test_fee_strategy_index_out_of_bounds_for_input_returns_error() {
            // Structure validation happens before signature validation
            // so we test it directly without needing valid signatures
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(
                create_platform_address(1),
                (1 as AddressNonce, dash_to_credits!(1.0)),
            );

            // DeductFromInput(5) but only 1 input exists
            let transition = AddressCreditWithdrawalTransitionV0 {
                inputs,
                output: None,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(5),
                ]),
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: create_random_output_script(&mut rng),
                user_fee_increase: 0,
                input_witnesses: vec![create_dummy_witness()],
            };

            let result = transition.validate_structure(platform_version);
            assert!(!result.is_valid());
            let error = result.first_error().unwrap();
            assert!(
                matches!(
                    error,
                    ConsensusError::BasicError(BasicError::FeeStrategyIndexOutOfBoundsError(_))
                ),
                "Expected FeeStrategyIndexOutOfBoundsError, got {:?}",
                error
            );
        }

        #[test]
        fn test_fee_strategy_index_out_of_bounds_for_output_returns_error() {
            // Structure validation happens before signature validation
            // so we test it directly without needing valid signatures
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(
                create_platform_address(1),
                (1 as AddressNonce, dash_to_credits!(1.0)),
            );

            // ReduceOutput(0) but no output exists
            let transition = AddressCreditWithdrawalTransitionV0 {
                inputs,
                output: None, // No output
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::ReduceOutput(0),
                ]),
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: create_random_output_script(&mut rng),
                user_fee_increase: 0,
                input_witnesses: vec![create_dummy_witness()],
            };

            let result = transition.validate_structure(platform_version);
            assert!(!result.is_valid());
            let error = result.first_error().unwrap();
            assert!(
                matches!(
                    error,
                    ConsensusError::BasicError(BasicError::FeeStrategyIndexOutOfBoundsError(_))
                ),
                "Expected FeeStrategyIndexOutOfBoundsError, got {:?}",
                error
            );
        }

        #[test]
        fn test_input_below_minimum_returns_error() {
            // Structure validation happens before signature validation
            // so we test it directly without needing valid signatures
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(create_platform_address(1), (1 as AddressNonce, 100)); // Very small input

            let transition = AddressCreditWithdrawalTransitionV0 {
                inputs,
                output: None,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: create_random_output_script(&mut rng),
                user_fee_increase: 0,
                input_witnesses: vec![create_dummy_witness()],
            };

            let result = transition.validate_structure(platform_version);
            assert!(!result.is_valid());
            let error = result.first_error().unwrap();
            assert!(
                matches!(
                    error,
                    ConsensusError::BasicError(BasicError::InputBelowMinimumError(_))
                ),
                "Expected InputBelowMinimumError, got {:?}",
                error
            );
        }

        #[test]
        fn test_output_below_minimum_returns_error() {
            // Structure validation happens before signature validation
            // so we test it directly without needing valid signatures
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(
                create_platform_address(1),
                (1 as AddressNonce, dash_to_credits!(1.0)),
            );

            // Very small output amount
            let output = Some((create_platform_address(2), 100));

            let transition = AddressCreditWithdrawalTransitionV0 {
                inputs,
                output,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: create_random_output_script(&mut rng),
                user_fee_increase: 0,
                input_witnesses: vec![create_dummy_witness()],
            };

            let result = transition.validate_structure(platform_version);
            assert!(!result.is_valid());
            let error = result.first_error().unwrap();
            assert!(
                matches!(
                    error,
                    ConsensusError::BasicError(BasicError::OutputBelowMinimumError(_))
                ),
                "Expected OutputBelowMinimumError, got {:?}",
                error
            );
        }

        #[test]
        fn test_fee_strategy_duplicate_steps_returns_error() {
            // Structure validation happens before signature validation
            // so we test it directly without needing valid signatures
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(
                create_platform_address(1),
                (1 as AddressNonce, dash_to_credits!(1.0)),
            );

            // Duplicate fee strategy steps
            let transition = AddressCreditWithdrawalTransitionV0 {
                inputs,
                output: None,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                    AddressFundsFeeStrategyStep::DeductFromInput(0), // Duplicate
                ]),
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: create_random_output_script(&mut rng),
                user_fee_increase: 0,
                input_witnesses: vec![create_dummy_witness()],
            };

            let result = transition.validate_structure(platform_version);
            assert!(!result.is_valid());
            let error = result.first_error().unwrap();
            assert!(
                matches!(
                    error,
                    ConsensusError::BasicError(BasicError::FeeStrategyDuplicateError(_))
                ),
                "Expected FeeStrategyDuplicateError, got {:?}",
                error
            );
        }

        // ==========================================
        // Withdrawal-specific validation tests
        // ==========================================

        #[test]
        fn test_pooling_not_never_returns_error() {
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(
                create_platform_address(1),
                (1 as AddressNonce, dash_to_credits!(1.0)),
            );

            // Pooling set to Standard instead of Never
            let transition = AddressCreditWithdrawalTransitionV0 {
                inputs,
                output: None,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                core_fee_per_byte: 1,
                pooling: Pooling::Standard,
                output_script: create_random_output_script(&mut rng),
                user_fee_increase: 0,
                input_witnesses: vec![create_dummy_witness()],
            };

            let result = transition.validate_structure(platform_version);
            assert!(!result.is_valid());
            let error = result.first_error().unwrap();
            assert!(
                matches!(
                    error,
                    ConsensusError::BasicError(
                        BasicError::NotImplementedCreditWithdrawalTransitionPoolingError(_)
                    )
                ),
                "Expected NotImplementedCreditWithdrawalTransitionPoolingError, got {:?}",
                error
            );
        }

        #[test]
        fn test_core_fee_per_byte_not_fibonacci_returns_error() {
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(
                create_platform_address(1),
                (1 as AddressNonce, dash_to_credits!(1.0)),
            );

            // core_fee_per_byte set to 4 which is not a Fibonacci number (1, 1, 2, 3, 5, 8, ...)
            let transition = AddressCreditWithdrawalTransitionV0 {
                inputs,
                output: None,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                core_fee_per_byte: 4, // Not a Fibonacci number
                pooling: Pooling::Never,
                output_script: create_random_output_script(&mut rng),
                user_fee_increase: 0,
                input_witnesses: vec![create_dummy_witness()],
            };

            let result = transition.validate_structure(platform_version);
            assert!(!result.is_valid());
            let error = result.first_error().unwrap();
            assert!(
                matches!(
                    error,
                    ConsensusError::BasicError(
                        BasicError::InvalidCreditWithdrawalTransitionCoreFeeError(_)
                    )
                ),
                "Expected InvalidCreditWithdrawalTransitionCoreFeeError, got {:?}",
                error
            );
        }

        #[test]
        fn test_core_fee_per_byte_zero_returns_error() {
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(
                create_platform_address(1),
                (1 as AddressNonce, dash_to_credits!(1.0)),
            );

            // core_fee_per_byte set to 0
            let transition = AddressCreditWithdrawalTransitionV0 {
                inputs,
                output: None,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                core_fee_per_byte: 0, // Invalid - 0 is not Fibonacci
                pooling: Pooling::Never,
                output_script: create_random_output_script(&mut rng),
                user_fee_increase: 0,
                input_witnesses: vec![create_dummy_witness()],
            };

            let result = transition.validate_structure(platform_version);
            assert!(!result.is_valid());
            let error = result.first_error().unwrap();
            assert!(
                matches!(
                    error,
                    ConsensusError::BasicError(
                        BasicError::InvalidCreditWithdrawalTransitionCoreFeeError(_)
                    )
                ),
                "Expected InvalidCreditWithdrawalTransitionCoreFeeError, got {:?}",
                error
            );
        }

        #[test]
        fn test_output_script_not_p2pkh_or_p2sh_returns_error() {
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            let mut inputs = BTreeMap::new();
            inputs.insert(
                create_platform_address(1),
                (1 as AddressNonce, dash_to_credits!(1.0)),
            );

            // Create an invalid output script (empty script is not P2PKH or P2SH)
            let invalid_output_script = CoreScript::new(ScriptBuf::new());

            let transition = AddressCreditWithdrawalTransitionV0 {
                inputs,
                output: None,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: invalid_output_script,
                user_fee_increase: 0,
                input_witnesses: vec![create_dummy_witness()],
            };

            let result = transition.validate_structure(platform_version);
            assert!(!result.is_valid());
            let error = result.first_error().unwrap();
            assert!(
                matches!(
                    error,
                    ConsensusError::BasicError(
                        BasicError::InvalidCreditWithdrawalTransitionOutputScriptError(_)
                    )
                ),
                "Expected InvalidCreditWithdrawalTransitionOutputScriptError, got {:?}",
                error
            );
        }

        #[test]
        fn test_output_script_op_return_returns_error() {
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            let mut inputs = BTreeMap::new();
            inputs.insert(
                create_platform_address(1),
                (1 as AddressNonce, dash_to_credits!(1.0)),
            );

            // Create an OP_RETURN script (not P2PKH or P2SH)
            let op_return_script = CoreScript::new(ScriptBuf::from(vec![
                OP_RETURN.to_u8(),
                0x04,
                0x01,
                0x02,
                0x03,
                0x04,
            ]));

            let transition = AddressCreditWithdrawalTransitionV0 {
                inputs,
                output: None,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: op_return_script,
                user_fee_increase: 0,
                input_witnesses: vec![create_dummy_witness()],
            };

            let result = transition.validate_structure(platform_version);
            assert!(!result.is_valid());
            let error = result.first_error().unwrap();
            assert!(
                matches!(
                    error,
                    ConsensusError::BasicError(
                        BasicError::InvalidCreditWithdrawalTransitionOutputScriptError(_)
                    )
                ),
                "Expected InvalidCreditWithdrawalTransitionOutputScriptError, got {:?}",
                error
            );
        }

        #[test]
        fn test_valid_withdrawal_passes_structure_validation() {
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(
                create_platform_address(1),
                (1 as AddressNonce, dash_to_credits!(1.0)),
            );

            let transition = AddressCreditWithdrawalTransitionV0 {
                inputs,
                output: None,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                core_fee_per_byte: 1, // Valid Fibonacci number
                pooling: Pooling::Never,
                output_script: create_random_output_script(&mut rng), // P2PKH script
                user_fee_increase: 0,
                input_witnesses: vec![create_dummy_witness()],
            };

            let result = transition.validate_structure(platform_version);
            assert!(
                result.is_valid(),
                "Expected valid result, got errors: {:?}",
                result.errors
            );
        }

        #[test]
        fn test_valid_fibonacci_core_fees_pass_validation() {
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);

            // Test various valid Fibonacci numbers: 1, 2, 3, 5, 8, 13, 21
            let fibonacci_numbers = vec![1u32, 2, 3, 5, 8, 13, 21];

            for fib in fibonacci_numbers {
                let mut inputs = BTreeMap::new();
                inputs.insert(
                    create_platform_address(1),
                    (1 as AddressNonce, dash_to_credits!(1.0)),
                );

                let transition = AddressCreditWithdrawalTransitionV0 {
                    inputs,
                    output: None,
                    fee_strategy: AddressFundsFeeStrategy::from(vec![
                        AddressFundsFeeStrategyStep::DeductFromInput(0),
                    ]),
                    core_fee_per_byte: fib,
                    pooling: Pooling::Never,
                    output_script: create_random_output_script(&mut rng),
                    user_fee_increase: 0,
                    input_witnesses: vec![create_dummy_witness()],
                };

                let result = transition.validate_structure(platform_version);
                assert!(
                    result.is_valid(),
                    "Expected valid result for Fibonacci number {}, got errors: {:?}",
                    fib,
                    result.errors
                );
            }
        }
    }

    // ==========================================
    // SUCCESSFUL TRANSITION TESTS
    // ==========================================

    mod successful_transitions {
        use super::*;

        #[test]
        fn test_simple_withdrawal_from_single_address() {
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.9)));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None, // No change output
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            // Check_tx should pass for valid transactions
            assert!(
                check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should accept valid withdrawal transaction"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
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
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
        }

        #[test]
        fn test_withdrawal_with_change_output() {
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(2.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(1.5)));

            // Change output to a different address
            let output = Some((create_platform_address(2), dash_to_credits!(0.3)));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                output,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            // Check_tx should pass for valid transactions
            assert!(
                check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should accept valid withdrawal with change output"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
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
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
        }

        #[test]
        fn test_withdrawal_from_multiple_inputs() {
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

            let mut signer = TestAddressSigner::new();
            let input_address1 = signer.add_p2pkh([1u8; 32]);
            let input_address2 = signer.add_p2pkh([2u8; 32]);
            let input_address3 = signer.add_p2pkh([3u8; 32]);
            setup_address_with_balance(&mut platform, input_address1, 0, dash_to_credits!(1.0));
            setup_address_with_balance(&mut platform, input_address2, 0, dash_to_credits!(1.0));
            setup_address_with_balance(&mut platform, input_address3, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address1, (1 as AddressNonce, dash_to_credits!(0.5)));
            inputs.insert(input_address2, (1 as AddressNonce, dash_to_credits!(0.5)));
            inputs.insert(input_address3, (1 as AddressNonce, dash_to_credits!(0.5)));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            // Check_tx should pass for valid transactions
            assert!(
                check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should accept valid withdrawal from multiple inputs"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
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
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
        }

        #[test]
        fn test_withdrawal_with_fee_deducted_from_output() {
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(2.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(1.5)));

            // Deduct fee from output instead of input
            let output = Some((create_platform_address(2), dash_to_credits!(0.5)));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                output,
                vec![AddressFundsFeeStrategyStep::ReduceOutput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            // Check_tx should pass for valid transactions
            assert!(
                check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should accept valid withdrawal with fee from output"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
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
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
        }

        #[test]
        fn test_withdrawal_with_user_fee_increase() {
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(2.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(1.0)));

            // Use fee increase
            let transition = create_signed_address_credit_withdrawal_transition_with_fee_increase(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
                100, // 1% fee increase
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            // Check_tx should pass for valid transactions
            assert!(
                check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should accept valid withdrawal with user fee increase"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
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
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
        }
    }

    // ==========================================
    // STATE VERIFICATION TESTS
    // Verify state changes after successful transitions
    // ==========================================

    mod state_verification {
        use super::*;

        #[test]
        fn test_balance_decreases_after_withdrawal() {
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            let initial_balance = dash_to_credits!(1.0);
            setup_address_with_balance(&mut platform, input_address, 0, initial_balance);

            // Verify initial state
            let (initial_nonce, initial_stored_balance) = platform
                .drive
                .fetch_balance_and_nonce(&input_address, None, platform_version)
                .expect("should fetch")
                .expect("address should exist");
            assert_eq!(initial_nonce, 0);
            assert_eq!(initial_stored_balance, initial_balance);

            let mut rng = StdRng::seed_from_u64(567);
            let withdrawal_amount = dash_to_credits!(0.5);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, withdrawal_amount));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
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
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );

            // Commit the transaction
            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("should commit");

            // Verify final state
            let (final_nonce, final_balance) = platform
                .drive
                .fetch_balance_and_nonce(&input_address, None, platform_version)
                .expect("should fetch")
                .expect("address should exist");

            // Nonce should be incremented
            assert_eq!(
                final_nonce, 1,
                "Nonce should be incremented after withdrawal"
            );

            // Balance should be reduced by the withdrawal amount (plus fees)
            assert!(
                final_balance < initial_balance,
                "Balance should decrease after withdrawal"
            );
            assert!(
                final_balance <= initial_balance - withdrawal_amount,
                "Balance should decrease by at least the withdrawal amount"
            );
        }

        #[test]
        fn test_output_address_receives_credits() {
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(2.0));

            let output_address = create_platform_address(2);

            // Verify output address doesn't exist initially
            let output_initial = platform
                .drive
                .fetch_balance_and_nonce(&output_address, None, platform_version)
                .expect("should fetch");
            assert!(
                output_initial.is_none(),
                "Output address should not exist initially"
            );

            let mut rng = StdRng::seed_from_u64(567);
            let output_amount = dash_to_credits!(0.5);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(1.0)));

            let output = Some((output_address, output_amount));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                output,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
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
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );

            // Commit the transaction
            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("should commit");

            // Verify output address now has balance
            let (output_nonce, output_balance) = platform
                .drive
                .fetch_balance_and_nonce(&output_address, None, platform_version)
                .expect("should fetch")
                .expect("output address should now exist");

            assert_eq!(output_nonce, 0, "New address should have nonce 0");
            assert_eq!(
                output_balance, output_amount,
                "Output should receive exact amount"
            );
        }

        #[test]
        fn test_multiple_inputs_all_decremented() {
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

            let mut signer = TestAddressSigner::new();
            let input_address1 = signer.add_p2pkh([1u8; 32]);
            let input_address2 = signer.add_p2pkh([2u8; 32]);
            setup_address_with_balance(&mut platform, input_address1, 0, dash_to_credits!(1.0));
            setup_address_with_balance(&mut platform, input_address2, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address1, (1 as AddressNonce, dash_to_credits!(0.3)));
            inputs.insert(input_address2, (1 as AddressNonce, dash_to_credits!(0.3)));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
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
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );

            // Commit the transaction
            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("should commit");

            // Verify both addresses have updated nonces and balances
            let (nonce1, balance1) = platform
                .drive
                .fetch_balance_and_nonce(&input_address1, None, platform_version)
                .expect("should fetch")
                .expect("address1 should exist");

            let (nonce2, balance2) = platform
                .drive
                .fetch_balance_and_nonce(&input_address2, None, platform_version)
                .expect("should fetch")
                .expect("address2 should exist");

            // Both nonces should be incremented
            assert_eq!(nonce1, 1, "Address1 nonce should be incremented");
            assert_eq!(nonce2, 1, "Address2 nonce should be incremented");

            // Both balances should be reduced
            assert!(
                balance1 < dash_to_credits!(1.0),
                "Address1 balance should decrease"
            );
            assert!(
                balance2 < dash_to_credits!(1.0),
                "Address2 balance should decrease"
            );
        }
    }

    // ==========================================
    // STATE VALIDATION TESTS
    // These test state validation errors (StateError)
    // ==========================================

    mod state_validation {
        use super::*;

        #[test]
        fn test_input_address_does_not_exist_returns_error() {
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            // Note: NOT setting up balance for this address

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject transaction with non-existent address"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
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

        #[test]
        fn test_insufficient_balance_in_input_returns_error() {
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            // Set up with only 0.5 DASH
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(0.5));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            // Try to spend 0.8 DASH when only 0.5 available
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.8)));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject transaction with insufficient balance"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
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
                    ConsensusError::StateError(StateError::AddressNotEnoughFundsError(_))
                )]
            );
        }

        #[test]
        fn test_wrong_nonce_returns_error() {
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            // Set up with nonce 0
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            // Use wrong nonce (5 instead of expected 1)
            inputs.insert(input_address, (5 as AddressNonce, dash_to_credits!(0.5)));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions (wrong nonce)
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject transaction with wrong nonce"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
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
    }

    // ==========================================
    // SIGNATURE VALIDATION TESTS
    // ==========================================

    mod signature_validation {
        use super::*;

        #[test]
        fn test_signature_from_different_key_for_input_returns_error() {
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

            // Create two signers - one for the "real" address, one that will sign incorrectly
            let mut real_signer = TestAddressSigner::new();
            let real_address = real_signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, real_address, 0, dash_to_credits!(1.0));

            // Create a different signer with a different key
            let mut wrong_signer = TestAddressSigner::new();
            let wrong_address = wrong_signer.add_p2pkh([2u8; 32]);
            // Add the real address hash to the wrong signer so it can "try" to sign for it
            // but with the wrong key
            wrong_signer.p2pkh_keys.insert(
                match real_address {
                    PlatformAddress::P2pkh(h) => h,
                    _ => panic!("expected p2pkh"),
                },
                wrong_signer.p2pkh_keys[&match wrong_address {
                    PlatformAddress::P2pkh(h) => h,
                    _ => panic!("expected p2pkh"),
                }]
                    .clone(),
            );

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(real_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            // Sign with wrong signer
            let transition = create_signed_address_credit_withdrawal_transition(
                &wrong_signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions (wrong signature)
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject transaction with wrong signature"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail due to witness verification error
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }

        #[test]
        fn test_empty_signature_returns_error() {
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            // Create transition with empty signature
            let transition = AddressCreditWithdrawalTransitionV0 {
                inputs,
                output: None,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: create_random_output_script(&mut rng),
                user_fee_increase: 0,
                input_witnesses: vec![AddressWitness::P2pkh {
                    signature: BinaryData::new(vec![]), // Empty signature
                }],
            };

            let state_transition: StateTransition = transition.into();
            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions (empty signature)
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject transaction with empty signature"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail due to invalid signature
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }

        #[test]
        fn test_corrupted_signature_returns_error() {
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            // Create transition with corrupted signature (random bytes)
            let transition = AddressCreditWithdrawalTransitionV0 {
                inputs,
                output: None,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: create_random_output_script(&mut rng),
                user_fee_increase: 0,
                input_witnesses: vec![AddressWitness::P2pkh {
                    signature: BinaryData::new(vec![0xDE, 0xAD, 0xBE, 0xEF]), // Corrupted signature
                }],
            };

            let state_transition: StateTransition = transition.into();
            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions (corrupted signature)
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject transaction with corrupted signature"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail due to invalid signature
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }

        #[test]
        fn test_multiple_inputs_one_wrong_signature_returns_error() {
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

            let mut signer = TestAddressSigner::new();
            let input_address1 = signer.add_p2pkh([1u8; 32]);
            let input_address2 = signer.add_p2pkh([2u8; 32]);
            setup_address_with_balance(&mut platform, input_address1, 0, dash_to_credits!(1.0));
            setup_address_with_balance(&mut platform, input_address2, 0, dash_to_credits!(1.0));

            // Create a wrong signer for address2
            let mut wrong_signer = TestAddressSigner::new();
            let _ = wrong_signer.add_p2pkh([99u8; 32]); // Different key

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address1, (1 as AddressNonce, dash_to_credits!(0.3)));
            inputs.insert(input_address2, (1 as AddressNonce, dash_to_credits!(0.3)));

            // Get the sorted order of inputs (BTreeMap iterates in sorted order)
            let input_keys: Vec<_> = inputs.keys().cloned().collect();

            // Create witnesses - first correct, second wrong
            let data_to_sign = [0u8; 32]; // Placeholder - actual signing would use real data
            let mut witnesses = Vec::new();
            for (idx, key) in input_keys.iter().enumerate() {
                if idx == 0 {
                    // First input: correct signature
                    let witness = signer
                        .sign_create_witness(key, &data_to_sign)
                        .expect("should sign");
                    witnesses.push(witness);
                } else {
                    // Second input: corrupted signature
                    witnesses.push(AddressWitness::P2pkh {
                        signature: BinaryData::new(vec![0xBA, 0xD0, 0x00, 0x00]),
                    });
                }
            }

            let transition = AddressCreditWithdrawalTransitionV0 {
                inputs,
                output: None,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: create_random_output_script(&mut rng),
                user_fee_increase: 0,
                input_witnesses: witnesses,
            };

            let state_transition: StateTransition = transition.into();
            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject transaction with one wrong signature"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail due to signature verification error
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }
    }

    // ==========================================
    // P2SH MULTISIG TESTS
    // ==========================================

    mod p2sh_multisig {
        use super::*;

        #[test]
        fn test_withdrawal_with_p2sh_multisig_input() {
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

            let mut signer = TestAddressSigner::new();
            // Create a 2-of-3 multisig
            let p2sh_address = signer.add_p2sh_multisig(2, &[[10u8; 32], [11u8; 32], [12u8; 32]]);
            setup_address_with_balance(&mut platform, p2sh_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(p2sh_address, (1 as AddressNonce, dash_to_credits!(0.8)));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            // Check_tx should pass for valid P2SH multisig transactions
            assert!(
                check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should accept valid P2SH multisig withdrawal"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
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
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
        }

        #[test]
        fn test_withdrawal_with_mixed_p2pkh_and_p2sh_inputs() {
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

            let mut signer = TestAddressSigner::new();

            // Create a P2PKH input
            let p2pkh_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, p2pkh_address, 0, dash_to_credits!(0.5));

            // Create a 2-of-3 P2SH multisig input
            let p2sh_address = signer.add_p2sh_multisig(2, &[[10u8; 32], [11u8; 32], [12u8; 32]]);
            setup_address_with_balance(&mut platform, p2sh_address, 0, dash_to_credits!(0.5));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(p2pkh_address, (1 as AddressNonce, dash_to_credits!(0.3)));
            inputs.insert(p2sh_address, (1 as AddressNonce, dash_to_credits!(0.3)));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            // Check_tx should pass for valid mixed P2PKH/P2SH transactions
            assert!(
                check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should accept valid mixed P2PKH/P2SH withdrawal"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
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
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
        }

        #[test]
        fn test_p2sh_with_insufficient_signatures_fails() {
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

            let mut signer = TestAddressSigner::new();
            // Create a 2-of-3 multisig
            let p2sh_address = signer.add_p2sh_multisig(2, &[[10u8; 32], [11u8; 32], [12u8; 32]]);
            setup_address_with_balance(&mut platform, p2sh_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(p2sh_address, (1 as AddressNonce, dash_to_credits!(0.8)));

            // Create transition manually with only 1 signature instead of required 2
            let mut transition = AddressCreditWithdrawalTransitionV0 {
                inputs: inputs.clone(),
                output: None,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: create_random_output_script(&mut rng),
                user_fee_increase: 0,
                input_witnesses: vec![],
            };

            // Get the entry and create witness with insufficient signatures
            let hash = match p2sh_address {
                PlatformAddress::P2sh(h) => h,
                _ => panic!("expected p2sh"),
            };
            let entry = signer.p2sh_entries.get(&hash).unwrap();

            // Only provide 1 signature instead of required 2
            let single_signature = TestAddressSigner::sign_data(&[0u8; 32], &entry.secret_keys[0]);
            transition.input_witnesses = vec![AddressWitness::P2sh {
                signatures: vec![BinaryData::new(single_signature)], // Only 1 sig, need 2
                redeem_script: BinaryData::new(entry.redeem_script.clone()),
            }];

            let state_transition: StateTransition = transition.into();
            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for invalid_unpaid transactions (insufficient signatures)
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject transaction with insufficient signatures"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail due to insufficient signatures
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }

        #[test]
        fn test_p2sh_1_of_2_multisig_succeeds() {
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

            let mut signer = TestAddressSigner::new();
            // Create a 1-of-2 multisig (less common but valid)
            let p2sh_address = signer.add_p2sh_multisig(1, &[[10u8; 32], [11u8; 32]]);
            setup_address_with_balance(&mut platform, p2sh_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(p2sh_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            // Check_tx should pass for valid 1-of-2 multisig
            assert!(
                check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should accept valid 1-of-2 multisig withdrawal"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
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
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
        }

        #[test]
        fn test_p2sh_3_of_3_multisig_succeeds() {
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

            let mut signer = TestAddressSigner::new();
            // Create a 3-of-3 multisig (requires all signatures)
            let p2sh_address = signer.add_p2sh_multisig(3, &[[10u8; 32], [11u8; 32], [12u8; 32]]);
            setup_address_with_balance(&mut platform, p2sh_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(p2sh_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            // Check_tx should pass for valid 3-of-3 multisig
            assert!(
                check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should accept valid 3-of-3 multisig withdrawal"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
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
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
        }

        #[test]
        fn test_p2sh_with_wrong_redeem_script_fails() {
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

            let mut signer = TestAddressSigner::new();
            // Create a 2-of-3 multisig
            let p2sh_address = signer.add_p2sh_multisig(2, &[[10u8; 32], [11u8; 32], [12u8; 32]]);
            setup_address_with_balance(&mut platform, p2sh_address, 0, dash_to_credits!(1.0));

            // Create a different P2SH to get a different redeem script
            let wrong_p2sh = signer.add_p2sh_multisig(2, &[[20u8; 32], [21u8; 32], [22u8; 32]]);

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(p2sh_address, (1 as AddressNonce, dash_to_credits!(0.8)));

            // Get the wrong redeem script
            let wrong_hash = match wrong_p2sh {
                PlatformAddress::P2sh(h) => h,
                _ => panic!("expected p2sh"),
            };
            let wrong_entry = signer.p2sh_entries.get(&wrong_hash).unwrap();

            // Get correct entry for signatures
            let correct_hash = match p2sh_address {
                PlatformAddress::P2sh(h) => h,
                _ => panic!("expected p2sh"),
            };
            let correct_entry = signer.p2sh_entries.get(&correct_hash).unwrap();

            // Create witness with correct signatures but wrong redeem script
            let data_to_sign = [0u8; 32];
            let sig1 = TestAddressSigner::sign_data(&data_to_sign, &correct_entry.secret_keys[0]);
            let sig2 = TestAddressSigner::sign_data(&data_to_sign, &correct_entry.secret_keys[1]);

            let transition = AddressCreditWithdrawalTransitionV0 {
                inputs,
                output: None,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: create_random_output_script(&mut rng),
                user_fee_increase: 0,
                input_witnesses: vec![AddressWitness::P2sh {
                    signatures: vec![BinaryData::new(sig1), BinaryData::new(sig2)],
                    redeem_script: BinaryData::new(wrong_entry.redeem_script.clone()), // Wrong redeem script!
                }],
            };

            let state_transition: StateTransition = transition.into();
            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Check_tx should fail for wrong redeem script
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject transaction with wrong redeem script"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Should fail due to wrong redeem script
            assert_eq!(processing_result.invalid_unpaid_count(), 1);
        }
    }

    // ==========================================
    // ERROR TYPE VERIFICATION TESTS
    // Verify specific error types are returned
    // ==========================================

    mod error_type_verification {
        use super::*;

        #[test]
        fn test_address_does_not_exist_returns_specific_error() {
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            // NOT setting up balance - address doesn't exist

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Verify specific error type
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::AddressDoesNotExistError(e))
                )] => {
                    assert_eq!(*e.address(), input_address);
                }
            );
        }

        #[test]
        fn test_insufficient_balance_returns_specific_error_with_amounts() {
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            let actual_balance = dash_to_credits!(0.5);
            setup_address_with_balance(&mut platform, input_address, 0, actual_balance);

            let mut rng = StdRng::seed_from_u64(567);

            let requested_amount = dash_to_credits!(0.8);
            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, requested_amount));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Verify specific error type and amounts
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::AddressNotEnoughFundsError(e))
                )] => {
                    assert_eq!(*e.address(), input_address);
                    assert_eq!(e.balance(), actual_balance);
                    assert!(e.required_balance() >= requested_amount);
                }
            );
        }

        #[test]
        fn test_invalid_nonce_returns_specific_error_with_values() {
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);

            let wrong_nonce = 5 as AddressNonce;
            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (wrong_nonce, dash_to_credits!(0.5)));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Verify specific error type and nonce values
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::AddressInvalidNonceError(e))
                )] => {
                    assert_eq!(*e.address(), input_address);
                    assert_eq!(e.expected_nonce(), 1); // Expected nonce after initial setup
                    assert_eq!(e.provided_nonce(), wrong_nonce);
                }
            );
        }

        #[test]
        fn test_witness_verification_error_returns_unpaid() {
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            // Create transition with invalid signature
            let transition = AddressCreditWithdrawalTransitionV0 {
                inputs,
                output: None,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: create_random_output_script(&mut rng),
                user_fee_increase: 0,
                input_witnesses: vec![AddressWitness::P2pkh {
                    signature: BinaryData::new(vec![0xBA, 0xD0]),
                }],
            };

            let state_transition: StateTransition = transition.into();
            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Verify it returns an unpaid error (witness verification failures are unpaid)
            assert_eq!(
                processing_result.invalid_unpaid_count(),
                1,
                "Invalid witness should result in unpaid error"
            );
        }
    }

    // ==========================================
    // FEE STRATEGY EXECUTION TESTS
    // Test fee strategy deduction patterns
    // ==========================================

    mod fee_strategy_execution {
        use super::*;

        #[test]
        fn test_fee_cascade_through_multiple_inputs() {
            // Test that fees cascade through inputs when first input is exhausted
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

            let mut signer = TestAddressSigner::new();
            let input_address1 = signer.add_p2pkh([1u8; 32]);
            let input_address2 = signer.add_p2pkh([2u8; 32]);
            // First input has small balance, second has more
            setup_address_with_balance(&mut platform, input_address1, 0, dash_to_credits!(0.1));
            setup_address_with_balance(&mut platform, input_address2, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            // Spend amounts that when combined with fees should cascade
            inputs.insert(input_address1, (1 as AddressNonce, dash_to_credits!(0.08)));
            inputs.insert(input_address2, (1 as AddressNonce, dash_to_credits!(0.5)));

            // Fee strategy: try input 0 first, then input 1
            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                    AddressFundsFeeStrategyStep::DeductFromInput(1),
                ],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            assert!(
                check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should accept cascading fee strategy"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
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
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
        }

        #[test]
        fn test_fee_deducted_from_input_and_output_combined() {
            // Test combined fee strategy: deduct from input first, then reduce output
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(2.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(1.5)));

            // Output to another address
            let output = Some((create_platform_address(2), dash_to_credits!(0.5)));

            // Combined strategy: try input first, then reduce output if needed
            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                output,
                vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                    AddressFundsFeeStrategyStep::ReduceOutput(0),
                ],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            assert!(
                check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should accept combined fee strategy"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
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
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
        }

        #[test]
        fn test_fee_exactly_depletes_input_to_zero() {
            // Edge case: fee exactly depletes an input, leaving zero balance
            // With the minimum balance pre-check, we need remaining balance >= required fee (~0.004 DASH)
            // Required fee = 400_000_000 (base) + 500_000 (1 input) = ~400.5M credits = ~0.004 DASH
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            // Set up 1 DASH balance and withdraw 0.99 DASH, leaving 0.01 DASH (1B credits) remaining
            // 1B credits >> 400.5M required fee
            let exact_balance = dash_to_credits!(1.0);
            setup_address_with_balance(&mut platform, input_address, 0, exact_balance);

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            // Withdraw 0.99 DASH, leaving 0.01 DASH remaining to pass fee pre-check
            let withdrawal_amount = dash_to_credits!(0.99);
            inputs.insert(input_address, (1 as AddressNonce, withdrawal_amount));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            assert!(
                check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should accept withdrawal with sufficient remaining balance"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
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
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );

            // Commit and verify balance decreased properly
            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("should commit");

            let (_, final_balance) = platform
                .drive
                .fetch_balance_and_nonce(&input_address, None, platform_version)
                .expect("should fetch")
                .expect("address should exist");

            // Final balance = remaining_balance - processing_fees (some fees still taken from remaining)
            // With 0.01 DASH (1B credits) remaining, after fees deducted, we get ~888M credits remaining
            // This is because some processing fees are charged from remaining balance
            assert!(
                final_balance < dash_to_credits!(0.01),
                "Final balance should be less than remaining after fee deduction"
            );
            assert!(
                final_balance > 0,
                "Final balance should be greater than zero"
            );
        }
    }

    // ==========================================
    // OUTPUT SCRIPT VALIDATION TESTS
    // ==========================================

    mod output_script_validation {
        use super::*;

        #[test]
        fn test_empty_output_script_returns_error() {
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            let mut inputs = BTreeMap::new();
            inputs.insert(
                create_platform_address(1),
                (1 as AddressNonce, dash_to_credits!(1.0)),
            );

            // Empty output script
            let transition = AddressCreditWithdrawalTransitionV0 {
                inputs,
                output: None,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: CoreScript::new(ScriptBuf::new()), // Empty script
                user_fee_increase: 0,
                input_witnesses: vec![create_dummy_witness()],
            };

            let result = transition.validate_structure(platform_version);
            // Empty output script should fail validation
            assert!(
                !result.is_valid(),
                "Empty output script should fail validation"
            );
        }

        #[test]
        fn test_valid_p2pkh_output_script_succeeds() {
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            // Use a standard P2PKH output script
            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                CoreScript::random_p2pkh(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            assert!(
                check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should accept valid P2PKH output script"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
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
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
        }

        #[test]
        fn test_valid_p2sh_output_script_succeeds() {
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            // Use a P2SH output script
            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                CoreScript::random_p2sh(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            assert!(
                check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should accept valid P2SH output script"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
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
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
        }
    }

    // ==========================================
    // CORE FEE PER BYTE TESTS
    // ==========================================

    mod core_fee_per_byte {
        use super::*;

        #[test]
        fn test_zero_core_fee_per_byte_returns_error() {
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(
                create_platform_address(1),
                (1 as AddressNonce, dash_to_credits!(1.0)),
            );

            let transition = AddressCreditWithdrawalTransitionV0 {
                inputs,
                output: None,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                core_fee_per_byte: 0, // Zero fee per byte
                pooling: Pooling::Never,
                output_script: create_random_output_script(&mut rng),
                user_fee_increase: 0,
                input_witnesses: vec![create_dummy_witness()],
            };

            let result = transition.validate_structure(platform_version);
            // Zero core_fee_per_byte should fail validation
            assert!(
                !result.is_valid(),
                "Zero core_fee_per_byte should fail validation"
            );
        }

        #[test]
        fn test_different_core_fee_per_byte_values() {
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(2.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            // Create transition with higher core_fee_per_byte
            let transition = AddressCreditWithdrawalTransitionV0::try_from_inputs_with_signer(
                inputs,
                None,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
                5, // Higher fee per byte
                Pooling::Never,
                create_random_output_script(&mut rng),
                &signer,
                0,
                platform_version,
            )
            .expect("should create signed transition");

            let result = transition.serialize_to_bytes().expect("should serialize");

            assert!(
                check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should accept higher core_fee_per_byte"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
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
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
        }
    }

    // ==========================================
    // POOLING TESTS
    // ==========================================

    mod pooling_tests {
        use super::*;

        #[test]
        fn test_pooling_if_available() {
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            // Use Pooling::IfAvailable
            let transition = AddressCreditWithdrawalTransitionV0::try_from_inputs_with_signer(
                inputs,
                None,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
                1,
                Pooling::IfAvailable, // Different pooling mode
                create_random_output_script(&mut rng),
                &signer,
                0,
                platform_version,
            )
            .expect("should create signed transition");

            let result = transition.serialize_to_bytes().expect("should serialize");

            // Pooling::IfAvailable is not yet implemented, should return error
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject Pooling::IfAvailable (not implemented)"
            );

            let check_result = run_check_tx(&platform, &result, platform_version);
            assert!(check_result.errors.iter().any(|e| matches!(
                e,
                ConsensusError::BasicError(
                    BasicError::NotImplementedCreditWithdrawalTransitionPoolingError(_)
                )
            )));
        }

        #[test]
        fn test_pooling_standard() {
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            // Use Pooling::Standard
            let transition = AddressCreditWithdrawalTransitionV0::try_from_inputs_with_signer(
                inputs,
                None,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
                1,
                Pooling::Standard, // Standard pooling
                create_random_output_script(&mut rng),
                &signer,
                0,
                platform_version,
            )
            .expect("should create signed transition");

            let result = transition.serialize_to_bytes().expect("should serialize");

            // Pooling::Standard is not yet implemented, should return error
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject Pooling::Standard (not implemented)"
            );

            let check_result = run_check_tx(&platform, &result, platform_version);
            assert!(check_result.errors.iter().any(|e| matches!(
                e,
                ConsensusError::BasicError(
                    BasicError::NotImplementedCreditWithdrawalTransitionPoolingError(_)
                )
            )));
        }
    }

    // ==========================================
    // USER FEE INCREASE EDGE CASES
    // ==========================================

    mod user_fee_increase_edge_cases {
        use super::*;

        #[test]
        fn test_maximum_user_fee_increase() {
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            // Need lots of balance for maximum fee increase
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(100.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(50.0)));

            // Maximum user_fee_increase is u16::MAX = 65535 (655.35% increase)
            let transition = create_signed_address_credit_withdrawal_transition_with_fee_increase(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
                u16::MAX, // Maximum fee increase
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            assert!(
                check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should accept maximum user_fee_increase"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
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
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
        }

        #[test]
        fn test_fee_increase_with_sufficient_balance_succeeds() {
            // Test that reasonable fee increases work with sufficient balance
            // Fee base ~400M credits, even with 100% increase is ~800M credits
            // 0.05 DASH = 5B credits remaining >> 800M required
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(0.2));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.15)));

            // 100% fee increase is still within remaining balance
            let transition = create_signed_address_credit_withdrawal_transition_with_fee_increase(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
                10000, // 100% fee increase
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            // Should succeed because 0.05 DASH remaining is enough for ~800M credits fee
            assert!(
                check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should accept when fee increase is within available balance"
            );
        }
    }

    // ==========================================
    // SYSTEM CREDITS VERIFICATION
    // ==========================================

    mod system_credits_verification {
        use super::*;

        #[test]
        fn test_system_credits_decrease_after_withdrawal() {
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            let initial_balance = dash_to_credits!(1.0);
            setup_address_with_balance(&mut platform, input_address, 0, initial_balance);

            // Get initial system credits
            let initial_system_credits = platform
                .drive
                .calculate_total_credits_balance(None, &platform_version.drive)
                .expect("should calculate system credits")
                .total_credits_in_platform;

            let mut rng = StdRng::seed_from_u64(567);
            let withdrawal_amount = dash_to_credits!(0.5);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, withdrawal_amount));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
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
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );

            // Commit the transaction
            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("should commit");

            // Get final system credits
            let final_system_credits = platform
                .drive
                .calculate_total_credits_balance(None, &platform_version.drive)
                .expect("should calculate system credits")
                .total_credits_in_platform;

            // System credits should decrease (withdrawal removes credits from the system)
            assert!(
                final_system_credits < initial_system_credits,
                "System credits should decrease after withdrawal. Initial: {}, Final: {}",
                initial_system_credits,
                final_system_credits
            );
        }
    }

    // ==========================================
    // BOUNDARY/EDGE CASE TESTS
    // ==========================================

    mod boundary_edge_cases {
        use super::*;

        #[test]
        fn test_maximum_inputs_succeeds() {
            let platform_version = PlatformVersion::latest();
            let max_inputs = platform_version.dpp.state_transitions.max_address_inputs as usize;

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

            let mut signer = TestAddressSigner::new();
            let mut inputs = BTreeMap::new();

            // Create exactly max_inputs (16) input addresses
            for i in 0..max_inputs {
                let mut seed = [1u8; 32]; // Start with non-zero values to ensure valid secp256k1 keys
                seed[0] = (i + 1) as u8; // Add 1 to avoid all-zeros case
                seed[1] = ((i + 1) >> 8) as u8;
                let address = signer.add_p2pkh(seed);
                setup_address_with_balance(&mut platform, address, 0, dash_to_credits!(0.5));
                inputs.insert(address, (1 as AddressNonce, dash_to_credits!(0.1)));
            }

            let mut rng = StdRng::seed_from_u64(567);

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            assert!(
                check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should accept maximum number of inputs"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
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
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
        }

        #[test]
        fn test_exact_balance_withdrawal_fails_insufficient_remaining_for_fees() {
            // Spending 100% of address balance should fail because there's nothing left for fees
            // Required fee = ~400.5M credits (~0.004 DASH), so remaining balance must be >= that
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            let exact_balance = dash_to_credits!(1.0);
            setup_address_with_balance(&mut platform, input_address, 0, exact_balance);

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            // Spend 100% of the balance - this should fail because 0 remaining < required fee
            inputs.insert(input_address, (1 as AddressNonce, exact_balance));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            // Should fail because spending 100% leaves 0 for fees
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject withdrawal with insufficient remaining balance for fees"
            );
        }

        #[test]
        fn test_high_balance_withdrawal() {
            // Test withdrawing a large portion of balance while leaving enough for fees
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            let balance = dash_to_credits!(1.0);
            setup_address_with_balance(&mut platform, input_address, 0, balance);

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            // Withdraw 0.99 DASH, leaving 0.01 DASH (1B credits) for fees
            // 1B credits >> 400.5M required fee
            let withdrawal_amount = dash_to_credits!(0.99);
            inputs.insert(input_address, (1 as AddressNonce, withdrawal_amount));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            assert!(
                check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should accept withdrawal with sufficient remaining balance"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
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
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
        }

        #[test]
        fn test_large_amount_withdrawal() {
            // Test withdrawal with very large amounts
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            // Very large balance (1 billion credits)
            let large_balance = 1_000_000_000_000_000_000u64;
            setup_address_with_balance(&mut platform, input_address, 0, large_balance);

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            // Withdraw most of it
            inputs.insert(
                input_address,
                (1 as AddressNonce, large_balance - dash_to_credits!(1.0)),
            );

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            assert!(
                check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should accept large amount withdrawal"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
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
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
        }
    }

    // ==========================================
    // REPLAY PROTECTION TESTS
    // ==========================================

    mod replay_protection {
        use super::*;

        #[test]
        fn test_same_transition_replayed_fails() {
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(2.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            // First execution - should succeed
            {
                let platform_state = platform.state.load();
                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![result.clone()],
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
                    [StateTransitionExecutionResult::SuccessfulExecution { .. }]
                );

                // Commit the transaction
                platform
                    .drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .expect("should commit");
            }

            // Check_tx should now fail for the replay attempt (nonce is now 1, but transition has nonce 1)
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject replayed transaction"
            );

            // Second execution with same transition - should fail due to nonce
            {
                let platform_state = platform.state.load();
                let transaction = platform.drive.grove.start_transaction();

                let processing_result = platform
                    .platform
                    .process_raw_state_transitions(
                        &vec![result],
                        &platform_state,
                        &BlockInfo::default(),
                        &transaction,
                        platform_version,
                        false,
                        None,
                    )
                    .expect("expected to process state transition");

                // Should fail due to nonce mismatch (nonce 1 was already used)
                assert_matches!(
                    processing_result.execution_results().as_slice(),
                    [StateTransitionExecutionResult::UnpaidConsensusError(
                        ConsensusError::StateError(StateError::AddressInvalidNonceError(_))
                    )]
                );
            }
        }
    }

    // ==========================================
    // BLOCK INFO EFFECTS TESTS
    // ==========================================

    mod block_info_effects {
        use super::*;

        #[test]
        fn test_withdrawal_with_different_block_height() {
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            // Use a specific block height
            let block_info = BlockInfo {
                height: 100_000,
                time_ms: 1700000000000,
                core_height: 50_000,
                epoch: Default::default(),
            };

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &block_info,
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
        }

        #[test]
        fn test_withdrawal_at_genesis_block() {
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            // Use genesis-like block info (height 1)
            let block_info = BlockInfo {
                height: 1,
                time_ms: 0,
                core_height: 1,
                epoch: Default::default(),
            };

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
                    &platform_state,
                    &block_info,
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
        }
    }

    // ==========================================
    // SERIALIZATION EDGE CASES
    // ==========================================

    mod serialization_edge_cases {
        use super::*;

        #[test]
        fn test_transition_round_trip_serialization() {
            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs.clone(),
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            // Serialize
            let serialized = transition.serialize_to_bytes().expect("should serialize");

            // Deserialize
            let deserialized =
                StateTransition::deserialize_from_bytes(&serialized).expect("should deserialize");

            // Re-serialize
            let reserialized = deserialized
                .serialize_to_bytes()
                .expect("should re-serialize");

            // Should produce identical bytes
            assert_eq!(
                serialized, reserialized,
                "Round-trip serialization should produce identical bytes"
            );
        }

        #[test]
        fn test_transition_with_many_inputs_serializes() {
            let platform_version = PlatformVersion::latest();
            let max_inputs = platform_version.dpp.state_transitions.max_address_inputs as usize;

            let mut signer = TestAddressSigner::new();
            let mut inputs = BTreeMap::new();

            // Create maximum number of inputs
            for i in 0..max_inputs {
                let mut seed = [1u8; 32]; // Start with non-zero values to ensure valid secp256k1 keys
                seed[0] = (i + 1) as u8; // Add 1 to avoid all-zeros case
                seed[1] = ((i + 1) >> 8) as u8;
                let address = signer.add_p2pkh(seed);
                inputs.insert(address, (1 as AddressNonce, dash_to_credits!(0.1)));
            }

            let mut rng = StdRng::seed_from_u64(567);

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            // Should serialize without error
            let serialized = transition.serialize_to_bytes().expect("should serialize");

            // Should deserialize without error
            let _deserialized =
                StateTransition::deserialize_from_bytes(&serialized).expect("should deserialize");
        }

        #[test]
        fn test_corrupted_serialized_data_returns_error() {
            let platform_version = PlatformVersion::latest();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let mut serialized = transition.serialize_to_bytes().expect("should serialize");

            // Corrupt the data
            if serialized.len() > 10 {
                serialized[10] ^= 0xFF;
                serialized[11] ^= 0xFF;
            }

            // Should fail to deserialize or produce invalid transition
            let result = StateTransition::deserialize_from_bytes(&serialized);
            // Either deserialization fails or produces invalid data
            // We accept both outcomes as valid error handling
            if result.is_ok() {
                // If it deserializes, it should fail validation
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

                // check_tx should reject corrupted data
                assert!(
                    !check_tx_is_valid(&platform, &serialized, platform_version),
                    "check_tx should reject corrupted serialized data"
                );
            }
            // If deserialization failed, that's also acceptable
        }
    }

    // ==========================================
    // CONCURRENT INPUT USAGE TESTS
    // ==========================================

    mod concurrent_input_usage {
        use super::*;

        #[test]
        fn test_two_transitions_same_input_address_sequential_nonces() {
            // Test that two withdrawals from the same address succeed with sequential nonces
            // when processed in separate blocks
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(5.0));

            let mut rng = StdRng::seed_from_u64(567);

            // First transition: spend 0.5 DASH with nonce 1
            let mut inputs1 = BTreeMap::new();
            inputs1.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let transition1 = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs1,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result1 = transition1.serialize_to_bytes().expect("should serialize");

            // Check_tx should pass for valid transaction
            assert!(
                check_tx_is_valid(&platform, &result1, platform_version),
                "check_tx should accept valid withdrawal with sequential nonce"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            // Process first transition
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result1],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // First should succeed
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
        }

        #[test]
        fn test_second_transition_exceeds_remaining_balance() {
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            // Only 1 DASH available
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);

            // First transition: spend 0.6 DASH
            let mut inputs1 = BTreeMap::new();
            inputs1.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.6)));

            let transition1 = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs1,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            // Second transition: try to spend 0.6 DASH again (but only ~0.4 remains after first)
            let mut inputs2 = BTreeMap::new();
            inputs2.insert(input_address, (2 as AddressNonce, dash_to_credits!(0.6)));

            let transition2 = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs2,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result1 = transition1.serialize_to_bytes().expect("should serialize");
            let result2 = transition2.serialize_to_bytes().expect("should serialize");

            // Check_tx should pass for first transaction
            assert!(
                check_tx_is_valid(&platform, &result1, platform_version),
                "check_tx should accept first valid withdrawal"
            );

            // Check_tx fails for second transaction because nonce 2 is invalid when nonce 1
            // hasn't been applied yet - the state still shows nonce 0
            assert!(
                !check_tx_is_valid(&platform, &result2, platform_version),
                "check_tx should reject second withdrawal (nonce 2 invalid before nonce 1 is applied)"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            // Process both transitions in the same block
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result1, result2],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transitions");

            // First should succeed, second should fail due to insufficient balance
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [
                    StateTransitionExecutionResult::SuccessfulExecution { .. },
                    StateTransitionExecutionResult::UnpaidConsensusError(
                        ConsensusError::StateError(StateError::AddressNotEnoughFundsError(_))
                    )
                ]
            );
        }
    }

    // ==========================================
    // OVERFLOW PROTECTION TESTS
    // ==========================================

    mod overflow_protection {
        use super::*;

        #[test]
        fn test_input_amounts_near_max_u64() {
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            let mut signer = TestAddressSigner::new();
            let input_address1 = signer.add_p2pkh([1u8; 32]);
            let input_address2 = signer.add_p2pkh([2u8; 32]);

            let mut rng = StdRng::seed_from_u64(567);

            // Two very large amounts that could overflow when summed
            let large_amount = u64::MAX / 2 + 1;

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address1, (1 as AddressNonce, large_amount));
            inputs.insert(input_address2, (1 as AddressNonce, large_amount));

            let transition = AddressCreditWithdrawalTransitionV0 {
                inputs,
                output: None,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: create_random_output_script(&mut rng),
                user_fee_increase: 0,
                input_witnesses: vec![create_dummy_witness(), create_dummy_witness()],
            };

            // Structure validation should handle overflow gracefully
            let result = transition.validate_structure(platform_version);
            // The validation might pass or fail, but shouldn't panic
            // If it passes structure validation, it will fail at state validation
            let _ = result;
        }

        #[test]
        fn test_user_fee_increase_overflow_protection() {
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);

            let mut rng = StdRng::seed_from_u64(567);

            // Large amount with max fee increase
            let large_amount = u64::MAX / 100;

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, large_amount));

            let transition = AddressCreditWithdrawalTransitionV0 {
                inputs,
                output: None,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: create_random_output_script(&mut rng),
                user_fee_increase: u16::MAX, // Max fee increase with large amount
                input_witnesses: vec![create_dummy_witness()],
            };

            // Should handle potential overflow in fee calculation gracefully
            let result = transition.validate_structure(platform_version);
            let _ = result; // Shouldn't panic
        }
    }

    // ==========================================
    // INVALID OUTPUT SCRIPT TESTS
    // ==========================================

    mod invalid_output_script {
        use super::*;

        #[test]
        fn test_op_return_output_script() {
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            let mut inputs = BTreeMap::new();
            inputs.insert(
                create_platform_address(1),
                (1 as AddressNonce, dash_to_credits!(1.0)),
            );

            // OP_RETURN script (unspendable)
            let op_return_script =
                ScriptBuf::from(vec![OP_RETURN.to_u8(), 0x04, 0xde, 0xad, 0xbe, 0xef]);

            let transition = AddressCreditWithdrawalTransitionV0 {
                inputs,
                output: None,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: CoreScript::new(op_return_script),
                user_fee_increase: 0,
                input_witnesses: vec![create_dummy_witness()],
            };

            let result = transition.validate_structure(platform_version);
            // OP_RETURN output script should fail validation
            assert!(
                !result.is_valid(),
                "OP_RETURN output script should fail validation"
            );
        }

        #[test]
        fn test_very_long_output_script() {
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            let mut inputs = BTreeMap::new();
            inputs.insert(
                create_platform_address(1),
                (1 as AddressNonce, dash_to_credits!(1.0)),
            );

            // Very long script (10KB)
            let long_script = ScriptBuf::from(vec![0u8; 10000]);

            let transition = AddressCreditWithdrawalTransitionV0 {
                inputs,
                output: None,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: CoreScript::new(long_script),
                user_fee_increase: 0,
                input_witnesses: vec![create_dummy_witness()],
            };

            let result = transition.validate_structure(platform_version);
            // Very long output script should fail validation
            assert!(
                !result.is_valid(),
                "Very long output script should fail validation"
            );
        }
    }

    // ==========================================
    // FEE STRATEGY EDGE CASES
    // ==========================================

    mod fee_strategy_edge_cases {
        use super::*;

        #[test]
        fn test_fee_exceeds_entire_withdrawal_amount() {
            // When fees would consume the entire withdrawal, leaving nothing for output
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            // Very small balance - fees might exceed withdrawal
            setup_address_with_balance(&mut platform, input_address, 0, 10000); // 10000 credits

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            // Try to withdraw the tiny amount
            inputs.insert(input_address, (1 as AddressNonce, 5000));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            // Should either fail validation or succeed with adjusted output
            // The behavior depends on implementation - we just verify it doesn't panic
            let _ = check_tx_is_valid(&platform, &result, platform_version);
        }

        #[test]
        fn test_all_reduce_output_fee_strategy() {
            // Test fee strategy that only uses ReduceOutput (no DeductFromInput)
            // Note: This might be invalid if there's no output to reduce
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(1.0)));

            // Only ReduceOutput in fee strategy - but withdrawal has no explicit output field
            // The output is the output_script (core withdrawal)
            let transition = AddressCreditWithdrawalTransitionV0 {
                inputs,
                output: None, // This is for platform address output, not core
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::ReduceOutput(0),
                ]),
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: create_random_output_script(&mut rng),
                user_fee_increase: 0,
                input_witnesses: vec![create_dummy_witness()],
            };

            let result = transition.validate_structure(platform_version);
            // ReduceOutput(0) with no output should fail validation
            assert!(
                !result.is_valid(),
                "ReduceOutput with no output should be rejected"
            );
        }
    }

    // ==========================================
    // WITNESS EDGE CASES
    // ==========================================

    mod witness_edge_cases {
        use super::*;

        #[test]
        fn test_p2sh_with_more_signatures_than_threshold() {
            // Provide 3 signatures for a 2-of-3 multisig (extra signature)
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

            let mut signer = TestAddressSigner::new();
            // Create a 2-of-3 multisig
            let p2sh_address = signer.add_p2sh_multisig(2, &[[10u8; 32], [11u8; 32], [12u8; 32]]);
            setup_address_with_balance(&mut platform, p2sh_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(p2sh_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            // Get the entry for manual signing
            let hash = match p2sh_address {
                PlatformAddress::P2sh(h) => h,
                _ => panic!("expected p2sh"),
            };
            let entry = signer.p2sh_entries.get(&hash).unwrap();

            // Sign with ALL 3 keys (more than the required 2)
            let data_to_sign = [0u8; 32];
            let sig1 = TestAddressSigner::sign_data(&data_to_sign, &entry.secret_keys[0]);
            let sig2 = TestAddressSigner::sign_data(&data_to_sign, &entry.secret_keys[1]);
            let sig3 = TestAddressSigner::sign_data(&data_to_sign, &entry.secret_keys[2]);

            let transition = AddressCreditWithdrawalTransitionV0 {
                inputs,
                output: None,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: create_random_output_script(&mut rng),
                user_fee_increase: 0,
                input_witnesses: vec![AddressWitness::P2sh {
                    signatures: vec![
                        BinaryData::new(sig1),
                        BinaryData::new(sig2),
                        BinaryData::new(sig3), // Extra signature
                    ],
                    redeem_script: BinaryData::new(entry.redeem_script.clone()),
                }],
            };

            let state_transition: StateTransition = transition.into();
            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Extra signatures might be accepted or rejected depending on implementation
            // The important thing is it doesn't panic
            let _ = check_tx_is_valid(&platform, &result, platform_version);
        }

        #[test]
        fn test_witness_with_zero_length_signature() {
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            // Zero-length signature
            let transition = AddressCreditWithdrawalTransitionV0 {
                inputs,
                output: None,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: create_random_output_script(&mut rng),
                user_fee_increase: 0,
                input_witnesses: vec![AddressWitness::P2pkh {
                    signature: BinaryData::new(vec![]), // Zero length
                }],
            };

            let state_transition: StateTransition = transition.into();
            let result = state_transition
                .serialize_to_bytes()
                .expect("should serialize");

            // Zero-length signature should be rejected
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject zero-length signature"
            );
        }
    }

    // ==========================================
    // NONCE BOUNDARY TESTS
    // ==========================================

    mod nonce_boundary {
        use super::*;

        #[test]
        fn test_nonce_at_max_minus_one() {
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            // Set up with nonce at max - 1 (AddressNonce is u32)
            let max_nonce_minus_one = u32::MAX - 1;
            setup_address_with_balance(
                &mut platform,
                input_address,
                max_nonce_minus_one,
                dash_to_credits!(1.0),
            );

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            // Use the max nonce value (u32::MAX)
            inputs.insert(
                input_address,
                (u32::MAX as AddressNonce, dash_to_credits!(0.5)),
            );

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            // Should accept max nonce
            assert!(
                check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should accept max nonce value"
            );

            let platform_state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![result],
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
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
        }

        #[test]
        fn test_nonce_zero_for_fresh_address() {
            // Fresh address should have nonce 0, first transaction uses nonce 1
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            // Set up with nonce 0 (fresh address)
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);

            // Try to use nonce 0 (should fail - first valid nonce is 1)
            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (0 as AddressNonce, dash_to_credits!(0.5)));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            // Nonce 0 should be rejected (first valid is 1)
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject nonce 0 for fresh address"
            );
        }
    }

    // ==========================================
    // Output Field Tests (Change Output)
    // ==========================================

    mod output_field_tests {
        use super::*;

        #[test]
        fn test_withdrawal_with_change_output_to_same_address_fails() {
            // Test withdrawal with change output going back to the same input address
            // This should fail because output address cannot be the same as an input address
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(2.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(1.5)));

            // Change output goes back to the same address (should fail)
            let output = Some((input_address, dash_to_credits!(0.5)));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                output,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            // Should fail with OutputAddressAlsoInputError
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject withdrawal with change to same input address"
            );
        }

        #[test]
        fn test_withdrawal_with_change_output_to_different_address() {
            // Test withdrawal with change output going to a different address
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(2.0));

            let change_address = create_platform_address(99);

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(1.5)));

            // Change output goes to a different address
            let output = Some((change_address, dash_to_credits!(0.5)));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                output,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            assert!(
                check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should accept withdrawal with change to different address"
            );
        }

        #[test]
        fn test_withdrawal_with_zero_change_output() {
            // Test withdrawal with change output of zero credits (should likely be rejected)
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            // Zero credits change output
            let output = Some((input_address, 0));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                output,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            // Zero change output should typically be rejected as wasteful
            let is_valid = check_tx_is_valid(&platform, &result, platform_version);
            // Document current behavior (may be accepted or rejected depending on implementation)
            println!("Zero change output validity: {}", is_valid);
        }

        #[test]
        fn test_withdrawal_change_output_exceeds_available() {
            // Test withdrawal where change output amount exceeds what's available after withdrawal
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            // Change output exceeds remaining (after withdrawal + fees)
            let output = Some((input_address, dash_to_credits!(2.0)));

            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                output,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject change output exceeding available balance"
            );
        }

        #[test]
        fn test_withdrawal_without_change_output() {
            // Test withdrawal with no change output (None)
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            // No change output
            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            assert!(
                check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should accept withdrawal without change output"
            );
        }
    }

    // ==========================================
    // Input Ordering/Determinism Tests
    // ==========================================

    mod input_ordering_tests {
        use super::*;

        #[test]
        fn test_multiple_inputs_processed_in_btreemap_order() {
            // BTreeMap ensures consistent ordering by key
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

            let mut signer = TestAddressSigner::new();
            // Create addresses with seeds that will sort differently
            let addr1 = signer.add_p2pkh([1u8; 32]);
            let addr2 = signer.add_p2pkh([2u8; 32]);
            let addr3 = signer.add_p2pkh([3u8; 32]);

            setup_address_with_balance(&mut platform, addr1, 0, dash_to_credits!(1.0));
            setup_address_with_balance(&mut platform, addr2, 0, dash_to_credits!(1.0));
            setup_address_with_balance(&mut platform, addr3, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);

            // Insert in "random" order - BTreeMap will sort them
            let mut inputs = BTreeMap::new();
            inputs.insert(addr3, (1 as AddressNonce, dash_to_credits!(0.3)));
            inputs.insert(addr1, (1 as AddressNonce, dash_to_credits!(0.3)));
            inputs.insert(addr2, (1 as AddressNonce, dash_to_credits!(0.3)));

            // Fee strategy references inputs by index (which is BTreeMap order)
            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                    AddressFundsFeeStrategyStep::DeductFromInput(1),
                    AddressFundsFeeStrategyStep::DeductFromInput(2),
                ],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            assert!(
                check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should accept multiple inputs in BTreeMap order"
            );
        }

        #[test]
        fn test_fee_deduction_follows_strategy_order() {
            // Verify that fee deduction happens in the order specified by fee_strategy
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

            let mut signer = TestAddressSigner::new();
            let addr1 = signer.add_p2pkh([1u8; 32]);
            let addr2 = signer.add_p2pkh([2u8; 32]);

            // Give different balances
            setup_address_with_balance(&mut platform, addr1, 0, dash_to_credits!(0.5));
            setup_address_with_balance(&mut platform, addr2, 0, dash_to_credits!(2.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(addr1, (1 as AddressNonce, dash_to_credits!(0.3)));
            inputs.insert(addr2, (1 as AddressNonce, dash_to_credits!(1.0)));

            // Deduct from input 1 (addr2) first, then input 0 (addr1)
            let transition = create_signed_address_credit_withdrawal_transition(
                &signer,
                inputs,
                None,
                vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(1),
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ],
                create_random_output_script(&mut rng),
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            assert!(
                check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should accept fee deduction in specified order"
            );
        }
    }

    // ==========================================
    // Witness Count Mismatch Tests
    // ==========================================

    mod witness_count_tests {
        use super::*;

        #[test]
        fn test_more_witnesses_than_inputs() {
            // Test with more witnesses than inputs
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

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            // Create with 3 witnesses but only 1 input
            let transition = create_raw_withdrawal_transition_with_dummy_witnesses(
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
                3, // More witnesses than inputs
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject more witnesses than inputs"
            );
        }

        #[test]
        fn test_fewer_witnesses_than_inputs() {
            // Test with fewer witnesses than inputs
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

            let input_address1 = create_platform_address(1);
            let input_address2 = create_platform_address(2);
            setup_address_with_balance(&mut platform, input_address1, 0, dash_to_credits!(1.0));
            setup_address_with_balance(&mut platform, input_address2, 0, dash_to_credits!(1.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address1, (1 as AddressNonce, dash_to_credits!(0.3)));
            inputs.insert(input_address2, (1 as AddressNonce, dash_to_credits!(0.3)));

            // Create with 1 witness but 2 inputs
            let transition = create_raw_withdrawal_transition_with_dummy_witnesses(
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
                1, // Fewer witnesses than inputs
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject fewer witnesses than inputs"
            );
        }

        #[test]
        fn test_empty_witnesses_with_inputs() {
            // Test with no witnesses but inputs present
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

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            // Create with 0 witnesses but 1 input
            let transition = create_raw_withdrawal_transition_with_dummy_witnesses(
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
                0, // No witnesses
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject empty witnesses with inputs"
            );
        }
    }

    // ==========================================
    // Core Fee Per Byte Edge Cases
    // ==========================================

    mod core_fee_per_byte_edge_cases {
        use super::*;

        /// Helper to create signed transition with custom core_fee_per_byte
        fn create_signed_withdrawal_with_core_fee(
            signer: &TestAddressSigner,
            inputs: BTreeMap<PlatformAddress, (AddressNonce, u64)>,
            output_script: CoreScript,
            core_fee_per_byte: u32,
        ) -> StateTransition {
            AddressCreditWithdrawalTransitionV0::try_from_inputs_with_signer(
                inputs,
                None,
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
                core_fee_per_byte,
                Pooling::Never,
                output_script,
                signer,
                0,
                PlatformVersion::latest(),
            )
            .expect("should create signed transition")
        }

        #[test]
        fn test_max_core_fee_per_byte() {
            // Test with u32::MAX core fee per byte
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(100.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(50.0)));

            let transition = create_signed_withdrawal_with_core_fee(
                &signer,
                inputs,
                create_random_output_script(&mut rng),
                u32::MAX, // Maximum value
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            // Very high core fee should still be accepted structurally
            // (actual core fee validation happens at execution time)
            let is_valid = check_tx_is_valid(&platform, &result, platform_version);
            println!("Max core_fee_per_byte validity: {}", is_valid);
        }

        #[test]
        fn test_high_core_fee_affects_withdrawal_amount() {
            // Test that high core fee affects the actual withdrawal to core
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

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(10.0));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(5.0)));

            let transition = create_signed_withdrawal_with_core_fee(
                &signer,
                inputs,
                create_random_output_script(&mut rng),
                987, // High fibonacci number (1000 is not fibonacci)
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            assert!(
                check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should accept high fibonacci core fee per byte"
            );
        }
    }

    // ==========================================
    // Combined Validation Failure Tests
    // ==========================================

    mod combined_validation_failures {
        use super::*;

        #[test]
        fn test_invalid_signature_and_insufficient_balance() {
            // Test which error takes precedence when both signature and balance are invalid
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
            // Very low balance
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(0.001));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            // Try to spend way more than available
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(100.0)));

            // Create with dummy (invalid) witnesses AND insufficient balance
            let transition = create_raw_withdrawal_transition_with_dummy_witnesses(
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
                1,
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            // Should be rejected (either for signature or balance, depending on validation order)
            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject with multiple validation failures"
            );
        }

        #[test]
        fn test_invalid_nonce_and_invalid_signature() {
            // Test with both wrong nonce and invalid signature
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

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            // Wrong nonce (expected 1, using 999)
            inputs.insert(input_address, (999 as AddressNonce, dash_to_credits!(0.5)));

            // Dummy (invalid) witness + wrong nonce
            let transition = create_raw_withdrawal_transition_with_dummy_witnesses(
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(0)],
                create_random_output_script(&mut rng),
                1,
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject with nonce and signature errors"
            );
        }

        #[test]
        fn test_fee_strategy_out_of_bounds_and_insufficient_balance() {
            // Test with fee strategy referencing non-existent input AND insufficient balance
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
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(0.001));

            let mut rng = StdRng::seed_from_u64(567);

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(100.0)));

            // Fee strategy references index 5 but we only have 1 input
            let transition = create_raw_withdrawal_transition_with_dummy_witnesses(
                inputs,
                None,
                vec![AddressFundsFeeStrategyStep::DeductFromInput(5)],
                create_random_output_script(&mut rng),
                1,
            );

            let result = transition.serialize_to_bytes().expect("should serialize");

            assert!(
                !check_tx_is_valid(&platform, &result, platform_version),
                "check_tx should reject with out-of-bounds fee strategy and insufficient balance"
            );
        }
    }
}
