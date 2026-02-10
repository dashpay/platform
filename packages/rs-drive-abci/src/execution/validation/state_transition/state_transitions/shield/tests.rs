#[cfg(test)]
mod tests {
    use crate::config::{PlatformConfig, PlatformTestConfig};
    use crate::execution::validation::state_transition::state_transitions::test_helpers::{
        create_dummy_witness, create_platform_address, setup_address_with_balance,
        TestAddressSigner,
    };
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use assert_matches::assert_matches;
    use dpp::address_funds::{
        AddressFundsFeeStrategy, AddressFundsFeeStrategyStep, AddressWitness, PlatformAddress,
    };
    use dpp::block::block_info::BlockInfo;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::signature::SignatureError;
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;
    use dpp::dash_to_credits;
    use dpp::fee::Credits;
    use dpp::identity::signer::Signer;
    use dpp::prelude::AddressNonce;
    use dpp::serialization::{PlatformSerializable, Signable};
    use dpp::shielded::SerializedAction;
    use dpp::state_transition::shield_transition::v0::ShieldTransitionV0;
    use dpp::state_transition::shield_transition::ShieldTransition;
    use dpp::state_transition::StateTransition;
    use platform_version::version::PlatformVersion;
    use std::collections::BTreeMap;

    // ==========================================
    // Helper Functions
    // ==========================================

    /// Create a `SerializedAction` with syntactically valid sizes but meaningless crypto data.
    /// Passes structure validation (correct field sizes) but will fail ZK proof verification.
    fn create_dummy_serialized_action() -> SerializedAction {
        SerializedAction {
            nullifier: [1u8; 32],
            rk: [2u8; 32],
            cmx: [3u8; 32],
            encrypted_note: vec![4u8; 692], // epk(32) + enc(580) + out(80)
            cv_net: [5u8; 32],
            spend_auth_sig: vec![6u8; 64],
        }
    }

    /// Builds a raw `ShieldTransitionV0` with dummy witnesses. Used for structure validation tests
    /// that don't need valid signatures (the structure error is caught before or alongside witness
    /// validation, or inputs are empty so witness validation is vacuously true).
    fn create_raw_shield_transition(
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        actions: Vec<SerializedAction>,
        flags: u8,
        value_balance: i64,
        proof: Vec<u8>,
        binding_signature: Vec<u8>,
        fee_strategy: AddressFundsFeeStrategy,
        witness_count: usize,
    ) -> StateTransition {
        let witnesses: Vec<AddressWitness> =
            (0..witness_count).map(|_| create_dummy_witness()).collect();
        StateTransition::Shield(ShieldTransition::V0(ShieldTransitionV0 {
            inputs,
            actions,
            flags,
            value_balance,
            anchor: [0u8; 32],
            proof,
            binding_signature,
            fee_strategy,
            user_fee_increase: 0,
            input_witnesses: witnesses,
        }))
    }

    /// Builds a `ShieldTransitionV0` and signs it with the provided signer.
    /// The transition will have valid witnesses for all inputs.
    fn create_signed_shield_transition(
        signer: &TestAddressSigner,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        actions: Vec<SerializedAction>,
        flags: u8,
        value_balance: i64,
        proof: Vec<u8>,
        binding_signature: Vec<u8>,
        fee_strategy: AddressFundsFeeStrategy,
    ) -> StateTransition {
        // First create with empty witnesses to compute signable bytes
        let mut st = StateTransition::Shield(ShieldTransition::V0(ShieldTransitionV0 {
            inputs: inputs.clone(),
            actions,
            flags,
            value_balance,
            anchor: [0u8; 32],
            proof,
            binding_signature,
            fee_strategy,
            user_fee_increase: 0,
            input_witnesses: vec![],
        }));

        // Compute signable bytes (excludes input_witnesses due to #[platform_signable(exclude_from_sig_hash)])
        let signable_bytes = st.signable_bytes().expect("should compute signable bytes");

        // Sign each input with the signer
        let witnesses: Vec<AddressWitness> = inputs
            .keys()
            .map(|address| {
                signer
                    .sign_create_witness(address, &signable_bytes)
                    .expect("should sign")
            })
            .collect();

        // Inject witnesses
        if let StateTransition::Shield(ShieldTransition::V0(ref mut v0)) = st {
            v0.input_witnesses = witnesses;
        }
        st
    }

    /// Shorthand for creating a structurally valid (but cryptographically invalid) signed shield
    /// transition with a single input address. The ZK proof data is random/dummy.
    fn create_default_signed_shield_transition(
        signer: &TestAddressSigner,
        input_address: PlatformAddress,
        input_nonce: AddressNonce,
        input_amount: Credits,
    ) -> StateTransition {
        let mut inputs = BTreeMap::new();
        inputs.insert(input_address, (input_nonce, input_amount));

        create_signed_shield_transition(
            signer,
            inputs,
            vec![create_dummy_serialized_action()],
            0x03, // spends_enabled | outputs_enabled
            -1000,
            vec![0u8; 100], // dummy proof bytes
            vec![0u8; 64],  // dummy binding signature
            AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(0)]),
        )
    }

    /// Standard platform setup for tests.
    fn setup_platform(
    ) -> crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike> {
        let platform_config = PlatformConfig {
            testing_configs: PlatformTestConfig {
                disable_instant_lock_signature_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        TestPlatformBuilder::new()
            .with_config(platform_config)
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state()
    }

    /// Execute a state transition through the full processing pipeline and return the result.
    fn process_transition(
        platform: &crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        transition: StateTransition,
        platform_version: &PlatformVersion,
    ) -> crate::platform_types::state_transitions_processing_result::StateTransitionsProcessingResult
    {
        let transition_bytes = transition
            .serialize_to_bytes()
            .expect("should serialize transition");
        let platform_state = platform.state.load();
        let transaction = platform.drive.grove.start_transaction();

        platform
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
            .expect("expected to process state transition")
    }

    // ==========================================
    // STRUCTURE VALIDATION TESTS (BasicError)
    // ==========================================

    mod structure_validation {
        use super::*;

        #[test]
        fn test_empty_actions_returns_error() {
            let platform_version = PlatformVersion::latest();
            let mut platform = setup_platform();

            // Need a properly signed transition with address in state so we get past
            // witness and address validation
            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let transition = create_signed_shield_transition(
                &signer,
                inputs,
                vec![], // Empty actions — invalid
                0x03,
                -1000,
                vec![0u8; 100],
                vec![0u8; 64],
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::OverflowError(_))
                )]
            );
        }

        #[test]
        fn test_no_inputs_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            // Empty inputs — witness and address validation are vacuously true
            let transition = create_raw_shield_transition(
                BTreeMap::new(), // no inputs
                vec![create_dummy_serialized_action()],
                0x03,
                -1000,
                vec![0u8; 100],
                vec![0u8; 64],
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
                0, // 0 witnesses to match 0 inputs
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::TransitionNoInputsError(_))
                )]
            );
        }

        #[test]
        fn test_witness_count_mismatch_returns_error() {
            let platform_version = PlatformVersion::latest();
            let mut platform = setup_platform();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            // Create a properly signed transition (1 input, 1 valid witness)
            let mut transition = create_default_signed_shield_transition(
                &signer,
                input_address,
                1,
                dash_to_credits!(0.5),
            );

            // Add an extra dummy witness to cause mismatch (1 input, 2 witnesses)
            if let StateTransition::Shield(ShieldTransition::V0(ref mut v0)) = transition {
                v0.input_witnesses.push(create_dummy_witness());
            }

            let processing_result = process_transition(&platform, transition, platform_version);

            // Witness validation runs before structure validation in the pipeline,
            // so count mismatch is caught as a SignatureError, not a BasicError.
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::SignatureError(
                        SignatureError::InvalidStateTransitionSignatureError(_)
                    )
                )]
            );
        }

        #[test]
        fn test_input_below_minimum_returns_error() {
            let platform_version = PlatformVersion::latest();
            let mut platform = setup_platform();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, 1)); // 1 credit — below minimum

            let transition = create_signed_shield_transition(
                &signer,
                inputs,
                vec![create_dummy_serialized_action()],
                0x03,
                -1,
                vec![0u8; 100],
                vec![0u8; 64],
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::InputBelowMinimumError(_))
                )]
            );
        }

        #[test]
        fn test_positive_value_balance_returns_error() {
            let platform_version = PlatformVersion::latest();
            let mut platform = setup_platform();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let transition = create_signed_shield_transition(
                &signer,
                inputs,
                vec![create_dummy_serialized_action()],
                0x03,
                1000, // Positive — invalid for shield (must be negative)
                vec![0u8; 100],
                vec![0u8; 64],
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::OverflowError(_))
                )]
            );
        }

        #[test]
        fn test_zero_value_balance_returns_error() {
            let platform_version = PlatformVersion::latest();
            let mut platform = setup_platform();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let transition = create_signed_shield_transition(
                &signer,
                inputs,
                vec![create_dummy_serialized_action()],
                0x03,
                0, // Zero — invalid for shield (must be negative)
                vec![0u8; 100],
                vec![0u8; 64],
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::OverflowError(_))
                )]
            );
        }

        #[test]
        fn test_empty_proof_returns_error() {
            let platform_version = PlatformVersion::latest();
            let mut platform = setup_platform();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let transition = create_signed_shield_transition(
                &signer,
                inputs,
                vec![create_dummy_serialized_action()],
                0x03,
                -1000,
                vec![], // Empty proof — invalid
                vec![0u8; 64],
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::OverflowError(_))
                )]
            );
        }

        #[test]
        fn test_wrong_binding_sig_length_returns_error() {
            let platform_version = PlatformVersion::latest();
            let mut platform = setup_platform();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let transition = create_signed_shield_transition(
                &signer,
                inputs,
                vec![create_dummy_serialized_action()],
                0x03,
                -1000,
                vec![0u8; 100],
                vec![0u8; 32], // 32 bytes instead of 64 — invalid
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::OverflowError(_))
                )]
            );
        }

        #[test]
        fn test_wrong_spend_auth_sig_length_returns_error() {
            let platform_version = PlatformVersion::latest();
            let mut platform = setup_platform();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let mut bad_action = create_dummy_serialized_action();
            bad_action.spend_auth_sig = vec![0u8; 32]; // 32 bytes instead of 64

            let transition = create_signed_shield_transition(
                &signer,
                inputs,
                vec![bad_action],
                0x03,
                -1000,
                vec![0u8; 100],
                vec![0u8; 64],
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::OverflowError(_))
                )]
            );
        }

        #[test]
        fn test_empty_fee_strategy_returns_error() {
            let platform_version = PlatformVersion::latest();
            let mut platform = setup_platform();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let transition = create_signed_shield_transition(
                &signer,
                inputs,
                vec![create_dummy_serialized_action()],
                0x03,
                -1000,
                vec![0u8; 100],
                vec![0u8; 64],
                AddressFundsFeeStrategy::from(vec![]), // Empty fee strategy
            );

            let processing_result = process_transition(&platform, transition, platform_version);

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
            let mut platform = setup_platform();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            // More steps than the max allowed (typically 4)
            let transition = create_signed_shield_transition(
                &signer,
                inputs,
                vec![create_dummy_serialized_action()],
                0x03,
                -1000,
                vec![0u8; 100],
                vec![0u8; 64],
                AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                    AddressFundsFeeStrategyStep::DeductFromInput(1),
                    AddressFundsFeeStrategyStep::DeductFromInput(2),
                    AddressFundsFeeStrategyStep::DeductFromInput(3),
                    AddressFundsFeeStrategyStep::DeductFromInput(4),
                ]),
            );

            let processing_result = process_transition(&platform, transition, platform_version);

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
            let mut platform = setup_platform();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let transition = create_signed_shield_transition(
                &signer,
                inputs,
                vec![create_dummy_serialized_action()],
                0x03,
                -1000,
                vec![0u8; 100],
                vec![0u8; 64],
                AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                    AddressFundsFeeStrategyStep::DeductFromInput(0), // Duplicate
                ]),
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::FeeStrategyDuplicateError(_))
                )]
            );
        }
    }

    // ==========================================
    // WITNESS VALIDATION TESTS (SignatureError)
    // ==========================================

    mod witness_validation {
        use super::*;

        #[test]
        fn test_invalid_witness_returns_error() {
            let platform_version = PlatformVersion::latest();
            let mut platform = setup_platform();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            // Create properly signed transition, then tamper with the witness
            let mut transition = create_default_signed_shield_transition(
                &signer,
                input_address,
                1,
                dash_to_credits!(0.5),
            );

            // Tamper the witness signature
            if let StateTransition::Shield(ShieldTransition::V0(ref mut v0)) = transition {
                if let Some(AddressWitness::P2pkh { ref mut signature }) =
                    v0.input_witnesses.first_mut()
                {
                    // Flip a byte in the signature
                    if let Some(byte) = signature.0.first_mut() {
                        *byte ^= 0xFF;
                    }
                }
            }

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::SignatureError(
                        SignatureError::InvalidStateTransitionSignatureError(_)
                    )
                )]
            );
        }

        #[test]
        fn test_wrong_key_witness_returns_error() {
            let platform_version = PlatformVersion::latest();
            let mut platform = setup_platform();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            // Create a second signer with different key
            let mut wrong_signer = TestAddressSigner::new();
            let _wrong_address = wrong_signer.add_p2pkh([2u8; 32]);

            // Build transition for the real input address but sign with wrong key's signer
            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            // The wrong_signer doesn't have input_address, so we manually create a bad witness
            let mut transition = create_signed_shield_transition(
                &signer,
                inputs,
                vec![create_dummy_serialized_action()],
                0x03,
                -1000,
                vec![0u8; 100],
                vec![0u8; 64],
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
            );

            // Replace the valid witness with one signed by a different key
            let signable_bytes = transition
                .signable_bytes()
                .expect("should compute signable bytes");
            let wrong_witness = wrong_signer
                .sign_create_witness(&_wrong_address, &signable_bytes)
                .expect("should sign");

            if let StateTransition::Shield(ShieldTransition::V0(ref mut v0)) = transition {
                v0.input_witnesses = vec![wrong_witness];
            }

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::SignatureError(
                        SignatureError::InvalidStateTransitionSignatureError(_)
                    )
                )]
            );
        }
    }

    // ==========================================
    // ADDRESS STATE VALIDATION TESTS (StateError)
    // ==========================================

    mod address_state_validation {
        use super::*;

        #[test]
        fn test_address_not_found_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            // NOTE: No setup_address_with_balance — address does not exist in state

            let transition = create_default_signed_shield_transition(
                &signer,
                input_address,
                1,
                dash_to_credits!(0.5),
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::AddressDoesNotExistError(_))
                )]
            );
        }

        #[test]
        fn test_wrong_nonce_returns_error() {
            let platform_version = PlatformVersion::latest();
            let mut platform = setup_platform();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            // Set up address with nonce 0
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            // Create transition with nonce 5 (expected nonce is 1)
            let transition = create_default_signed_shield_transition(
                &signer,
                input_address,
                5, // Wrong nonce — state has 0, expected next is 1
                dash_to_credits!(0.5),
            );

            let processing_result = process_transition(&platform, transition, platform_version);

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
            let mut platform = setup_platform();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            // Set up address with small balance
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(0.001));

            // Try to shield more than the balance
            let transition = create_default_signed_shield_transition(
                &signer,
                input_address,
                1,
                dash_to_credits!(1.0), // Way more than 0.001 Dash balance
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::AddressNotEnoughFundsError(_))
                )]
            );
        }
    }

    // ==========================================
    // ZK PROOF VERIFICATION TESTS (InvalidShieldedProofError)
    // ==========================================

    mod proof_verification {
        use super::*;

        #[test]
        fn test_invalid_proof_returns_shielded_proof_error() {
            let platform_version = PlatformVersion::latest();
            let mut platform = setup_platform();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            // This transition is structurally valid but has random ZK proof data.
            // It should pass structure validation, witness validation, and address validation
            // but fail at proof verification in transform_into_action.
            let transition = create_default_signed_shield_transition(
                &signer,
                input_address,
                1,
                dash_to_credits!(0.5),
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            // The proof verification happens during transform_into_action.
            // With random data, reconstruct_and_verify_bundle should fail at
            // parsing the cryptographic fields (nullifier, rk, cmx, cv_net) or
            // at the actual proof verification step.
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::PaidConsensusError {
                    error: ConsensusError::StateError(StateError::InvalidShieldedProofError(_)),
                    ..
                }]
            );
        }

        #[test]
        fn test_wrong_encrypted_note_size_returns_error() {
            let platform_version = PlatformVersion::latest();
            let mut platform = setup_platform();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            // Create action with wrong encrypted_note size
            let mut bad_action = create_dummy_serialized_action();
            bad_action.encrypted_note = vec![0u8; 100]; // 100 bytes instead of 692

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let transition = create_signed_shield_transition(
                &signer,
                inputs,
                vec![bad_action],
                0x03,
                -1000,
                vec![0u8; 100],
                vec![0u8; 64],
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            // The encrypted_note size check happens in reconstruct_and_verify_bundle,
            // which runs during transform_into_action after all prior validations pass.
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::PaidConsensusError {
                    error: ConsensusError::StateError(StateError::InvalidShieldedProofError(_)),
                    ..
                }]
            );
        }
    }
}
