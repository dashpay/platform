#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::execution::validation::state_transition::state_transitions::shielded_common::compute_platform_sighash;
    use crate::execution::validation::state_transition::state_transitions::test_helpers::{
        create_dummy_serialized_action, create_dummy_witness, create_platform_address,
        get_proving_key, process_transition, serialize_authorized_bundle_with_flags,
        setup_address_with_balance, setup_address_with_balance_and_system_credits, setup_platform,
        TestAddressSigner,
    };
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
    use assert_matches::assert_matches;
    use dpp::address_funds::{
        AddressFundsFeeStrategy, AddressFundsFeeStrategyStep, AddressWitness, PlatformAddress,
    };
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
    use grovedb_commitment_tree::{
        Anchor, Builder, BundleType, DashMemo, Flags as OrchardFlags, FullViewingKey, NoteValue,
        Scope, SpendingKey,
    };
    use platform_version::version::PlatformVersion;
    use rand::rngs::OsRng;
    use std::collections::BTreeMap;

    // ==========================================
    // Helper Functions (transition-specific)
    // ==========================================

    /// Builds a raw `ShieldTransitionV0` with dummy witnesses. Used for structure validation tests
    /// that don't need valid signatures (the structure error is caught before or alongside witness
    /// validation, or inputs are empty so witness validation is vacuously true).
    fn create_raw_shield_transition(
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        actions: Vec<SerializedAction>,
        amount: u64,
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        fee_strategy: AddressFundsFeeStrategy,
        witness_count: usize,
    ) -> StateTransition {
        let witnesses: Vec<AddressWitness> =
            (0..witness_count).map(|_| create_dummy_witness()).collect();
        StateTransition::Shield(ShieldTransition::V0(ShieldTransitionV0 {
            inputs,
            actions,
            amount,
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
    async fn create_signed_shield_transition(
        signer: &TestAddressSigner,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        actions: Vec<SerializedAction>,
        amount: u64,
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        fee_strategy: AddressFundsFeeStrategy,
    ) -> StateTransition {
        // First create with empty witnesses to compute signable bytes
        let mut st = StateTransition::Shield(ShieldTransition::V0(ShieldTransitionV0 {
            inputs: inputs.clone(),
            actions,
            amount,
            anchor: [42u8; 32],
            proof,
            binding_signature,
            fee_strategy,
            user_fee_increase: 0,
            input_witnesses: vec![],
        }));

        // Compute signable bytes (excludes input_witnesses due to #[platform_signable(exclude_from_sig_hash)])
        let signable_bytes = st.signable_bytes().expect("should compute signable bytes");

        // Sign each input with the signer
        let mut witnesses: Vec<AddressWitness> = Vec::with_capacity(inputs.len());
        for address in inputs.keys() {
            let witness = signer
                .sign_create_witness(address, &signable_bytes)
                .await
                .expect("should sign");
            witnesses.push(witness);
        }

        // Inject witnesses
        if let StateTransition::Shield(ShieldTransition::V0(ref mut v0)) = st {
            v0.input_witnesses = witnesses;
        }
        st
    }

    /// Shorthand for creating a structurally valid (but cryptographically invalid) signed shield
    /// transition with a single input address. The ZK proof data is random/dummy.
    async fn create_default_signed_shield_transition(
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
            1000,
            vec![0u8; 100], // dummy proof bytes
            [0u8; 64],      // dummy binding signature
            AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(0)]),
        )
        .await
    }

    // (Orchard ProvingKey and serialize_authorized_bundle are now shared
    //  via test_helpers::get_proving_key / serialize_authorized_bundle_with_flags)

    // ==========================================
    // STRUCTURE VALIDATION TESTS (BasicError)
    // ==========================================

    mod structure_validation {
        use super::*;

        #[tokio::test]
        async fn test_empty_actions_returns_error() {
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
                1000,
                vec![0u8; 100],
                [0u8; 64],
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
            )
            .await;

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::ShieldedNoActionsError(_))
                )]
            );
        }

        #[tokio::test]
        async fn test_no_inputs_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            // Empty inputs — witness and address validation are vacuously true
            let transition = create_raw_shield_transition(
                BTreeMap::new(), // no inputs
                vec![create_dummy_serialized_action()],
                1000,
                vec![0u8; 100],
                [0u8; 64],
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

        #[tokio::test]
        async fn test_witness_count_mismatch_returns_error() {
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
            )
            .await;

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

        #[tokio::test]
        async fn test_input_below_minimum_returns_error() {
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
                1,
                vec![0u8; 100],
                [0u8; 64],
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
            )
            .await;

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::InputBelowMinimumError(_))
                )]
            );
        }

        /// Tests `validate_structure` directly to exercise the
        /// `max_shielded_transition_actions` check in isolation, without
        /// depending on the full transition-processing pipeline.
        #[test]
        fn test_too_many_actions_returns_error() {
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            // 17 actions exceeds max_shielded_transition_actions (16)
            let actions: Vec<SerializedAction> =
                (0..17).map(|_| create_dummy_serialized_action()).collect();

            let transition = ShieldTransitionV0 {
                inputs: BTreeMap::new(),
                actions,
                amount: 1000,
                anchor: [42u8; 32],
                proof: vec![0u8; 100],
                binding_signature: [0u8; 64],
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                user_fee_increase: 0,
                input_witnesses: vec![],
            };

            let result = transition.validate_structure(platform_version);

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::ShieldedTooManyActionsError(_)
                )]
            );
        }

        #[tokio::test]
        async fn test_amount_exceeding_i64_max_returns_error() {
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
                i64::MAX as u64 + 1, // Exceeds i64::MAX
                vec![0u8; 100],
                [0u8; 64],
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
            )
            .await;

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::ShieldedInvalidValueBalanceError(_))
                )]
            );
        }

        #[tokio::test]
        async fn test_zero_value_balance_returns_error() {
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
                0,
                vec![0u8; 100],
                [0u8; 64],
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
            )
            .await;

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::ShieldedInvalidValueBalanceError(_))
                )]
            );
        }

        #[tokio::test]
        async fn test_empty_proof_returns_error() {
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
                1000,
                vec![], // Empty proof — invalid
                [0u8; 64],
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
            )
            .await;

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::ShieldedEmptyProofError(_))
                )]
            );
        }

        #[tokio::test]
        async fn test_empty_fee_strategy_returns_error() {
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
                1000,
                vec![0u8; 100],
                [0u8; 64],
                AddressFundsFeeStrategy::from(vec![]), // Empty fee strategy
            )
            .await;

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::FeeStrategyEmptyError(_))
                )]
            );
        }

        #[tokio::test]
        async fn test_fee_strategy_too_many_steps_returns_error() {
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
                1000,
                vec![0u8; 100],
                [0u8; 64],
                AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                    AddressFundsFeeStrategyStep::DeductFromInput(1),
                    AddressFundsFeeStrategyStep::DeductFromInput(2),
                    AddressFundsFeeStrategyStep::DeductFromInput(3),
                    AddressFundsFeeStrategyStep::DeductFromInput(4),
                ]),
            )
            .await;

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::FeeStrategyTooManyStepsError(_))
                )]
            );
        }

        #[tokio::test]
        async fn test_fee_strategy_duplicate_returns_error() {
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
                1000,
                vec![0u8; 100],
                [0u8; 64],
                AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                    AddressFundsFeeStrategyStep::DeductFromInput(0), // Duplicate
                ]),
            )
            .await;

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

        #[tokio::test]
        async fn test_invalid_witness_returns_error() {
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
            )
            .await;

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

        #[tokio::test]
        async fn test_wrong_key_witness_returns_error() {
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
                1000,
                vec![0u8; 100],
                [0u8; 64],
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
            )
            .await;

            // Replace the valid witness with one signed by a different key
            let signable_bytes = transition
                .signable_bytes()
                .expect("should compute signable bytes");
            let wrong_witness = wrong_signer
                .sign_create_witness(&_wrong_address, &signable_bytes)
                .await
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

        #[tokio::test]
        async fn test_address_not_found_returns_error() {
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
            )
            .await;

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::AddressDoesNotExistError(_))
                )]
            );
        }

        #[tokio::test]
        async fn test_wrong_nonce_returns_error() {
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
            )
            .await;

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::AddressInvalidNonceError(_))
                )]
            );
        }

        #[tokio::test]
        async fn test_insufficient_balance_returns_error() {
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
            )
            .await;

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

        #[tokio::test]
        async fn test_invalid_proof_returns_shielded_proof_error() {
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
            )
            .await;

            let processing_result = process_transition(&platform, transition, platform_version);

            // The proof verification happens during transform_into_action.
            // With random data, reconstruct_and_verify_bundle should fail at
            // parsing the cryptographic fields (nullifier, rk, cmx, cv_net) or
            // at the actual proof verification step.
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidShieldedProofError(_))
                )]
            );
        }

        #[tokio::test]
        async fn test_valid_shield_proof_succeeds() {
            let platform_version = PlatformVersion::latest();
            let mut platform = setup_platform();

            // --- Set up input address with enough balance ---
            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            // --- Build valid Orchard bundle (shield = outputs only) ---
            let mut rng = OsRng;
            let pk = get_proving_key();

            let sk = SpendingKey::from_bytes([0u8; 32]).unwrap();
            let fvk = FullViewingKey::from(&sk);
            let recipient = fvk.address_at(0u32, Scope::External);

            let anchor = Anchor::empty_tree();
            let mut builder = Builder::<DashMemo>::new(
                BundleType::Transactional {
                    flags: OrchardFlags::SPENDS_DISABLED,
                    bundle_required: false,
                },
                anchor,
            );

            let shield_value = 5000u64;
            builder
                .add_output(
                    None,
                    recipient,
                    NoteValue::from_raw(shield_value),
                    [0u8; 36],
                )
                .unwrap();

            let (unauthorized, _) = builder.build::<i64>(&mut rng).unwrap().unwrap();
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &[]);
            let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
            let bundle = proven.apply_signatures(rng, sighash, &[]).unwrap();

            // --- Extract serialized fields from the authorized bundle ---
            let (actions, _flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                serialize_authorized_bundle_with_flags(&bundle);

            // value_balance should be negative for shield (money going into pool)
            assert!(value_balance < 0);
            let shield_amount = (-value_balance) as u64;

            // --- Build and sign the shield transition ---
            let mut inputs = BTreeMap::new();
            inputs.insert(
                input_address,
                (1 as AddressNonce, shield_amount + dash_to_credits!(0.01)),
            );

            let mut st = StateTransition::Shield(ShieldTransition::V0(ShieldTransitionV0 {
                inputs: inputs.clone(),
                actions,
                amount: shield_amount,
                anchor: anchor_bytes,
                proof: proof_bytes,
                binding_signature: binding_sig,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                user_fee_increase: 0,
                input_witnesses: vec![],
            }));

            let signable_bytes = st.signable_bytes().expect("should compute signable bytes");
            let mut witnesses: Vec<AddressWitness> = Vec::with_capacity(inputs.len());
            for address in inputs.keys() {
                let witness = signer
                    .sign_create_witness(address, &signable_bytes)
                    .await
                    .expect("should sign");
                witnesses.push(witness);
            }

            if let StateTransition::Shield(ShieldTransition::V0(ref mut v0)) = st {
                v0.input_witnesses = witnesses;
            }

            // CheckTx root-invariance guard (devnet paloma h788): `check_tx` asserts under
            // cfg(test) that it never mutates committed grovedb state, so running the
            // canonical valid fixture through it pins the invariant for this transition type.
            {
                use dpp::serialization::PlatformSerializable;
                let guard_serialized_transition = st
                    .serialize_to_bytes()
                    .expect("expected to serialize transition for the check_tx guard");
                crate::test::helpers::state_mutation_guard::assert_check_tx_valid_at_all_levels(
                    &platform,
                    &guard_serialized_transition,
                    "shield",
                );
            }

            let processing_result = process_transition(&platform, st, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
        }

        #[tokio::test]
        async fn test_wrong_encrypted_note_size_returns_error() {
            let platform_version = PlatformVersion::latest();
            let mut platform = setup_platform();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            // Create action with wrong encrypted_note size
            let mut bad_action = create_dummy_serialized_action();
            bad_action.encrypted_note = vec![0u8; 100]; // 100 bytes instead of 216

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, dash_to_credits!(0.5)));

            let transition = create_signed_shield_transition(
                &signer,
                inputs,
                vec![bad_action],
                1000,
                vec![0u8; 100],
                [0u8; 64],
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
            )
            .await;

            let processing_result = process_transition(&platform, transition, platform_version);

            // The encrypted_note size check now happens in DPP structure validation
            // (before reaching proof verification), returning a BasicError.
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::ShieldedEncryptedNoteSizeMismatchError(
                        _
                    ))
                )]
            );
        }
    }

    // ==========================================
    // SECURITY AUDIT TESTS
    // ==========================================

    mod security_audit {
        use super::*;

        /// Zero anchor is rejected at structure validation.
        /// Tests validate_structure directly because witness verification runs before
        /// structure validation in the full pipeline.
        #[tokio::test]
        async fn test_zero_anchor_returns_error() {
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            let mut inputs = BTreeMap::new();
            inputs.insert(
                create_platform_address(1),
                (1 as AddressNonce, dash_to_credits!(0.5)),
            );

            let transition = ShieldTransitionV0 {
                inputs,
                actions: vec![create_dummy_serialized_action()],
                amount: 1000,
                anchor: [0u8; 32], // Zero anchor — invalid
                proof: vec![0u8; 100],
                binding_signature: [0u8; 64],
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                user_fee_increase: 0,
                input_witnesses: vec![create_dummy_witness()],
            };

            let result = transition.validate_structure(platform_version);

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::ShieldedZeroAnchorError(_)
                )]
            );
        }

        /// AUDIT FIX VERIFICATION: Mutated value_balance is now rejected.
        ///
        /// Previously, the binding signature was not verified so mutating
        /// value_balance from -5000 to -100000 was accepted. Now with
        /// BatchValidator, the changed value_balance produces a different
        /// bundle commitment (sighash), causing signature verification to fail.
        #[tokio::test]
        async fn test_valid_proof_with_mutated_value_balance_is_rejected() {
            let platform_version = PlatformVersion::latest();
            let mut platform = setup_platform();

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            let mut rng = OsRng;
            let pk = get_proving_key();

            let sk = SpendingKey::from_bytes([0u8; 32]).unwrap();
            let fvk = FullViewingKey::from(&sk);
            let recipient = fvk.address_at(0u32, Scope::External);

            let anchor = Anchor::empty_tree();
            let mut builder = Builder::<DashMemo>::new(
                BundleType::Transactional {
                    flags: OrchardFlags::SPENDS_DISABLED,
                    bundle_required: false,
                },
                anchor,
            );

            builder
                .add_output(None, recipient, NoteValue::from_raw(5_000), [0u8; 36])
                .unwrap();

            let (unauthorized, _) = builder.build::<i64>(&mut rng).unwrap().unwrap();
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &[]);
            let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
            let bundle = proven.apply_signatures(rng, sighash, &[]).unwrap();

            let (actions, _flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                serialize_authorized_bundle_with_flags(&bundle);

            assert!(value_balance < 0);
            let honest_shield_amount = (-value_balance) as u64;
            assert_eq!(honest_shield_amount, 5_000);

            // ATTACK: Mutate amount to claim shielding 100,000 instead of 5,000
            let mutated_amount = 100_000u64;

            // Input only provides enough for a small amount, but shield_amount
            // comes from amount, not from inputs
            let mut inputs = BTreeMap::new();
            inputs.insert(
                input_address,
                (
                    1 as AddressNonce,
                    honest_shield_amount + dash_to_credits!(0.01),
                ),
            );

            let mut st = StateTransition::Shield(ShieldTransition::V0(ShieldTransitionV0 {
                inputs: inputs.clone(),
                actions,
                amount: mutated_amount, // MUTATED
                anchor: anchor_bytes,   // Must match the proof's anchor (circuit instance)
                proof: proof_bytes,
                binding_signature: binding_sig,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                user_fee_increase: 0,
                input_witnesses: vec![],
            }));

            let signable_bytes = st.signable_bytes().expect("should compute signable bytes");
            let mut witnesses: Vec<AddressWitness> = Vec::with_capacity(inputs.len());
            for address in inputs.keys() {
                let witness = signer
                    .sign_create_witness(address, &signable_bytes)
                    .await
                    .expect("should sign");
                witnesses.push(witness);
            }

            if let StateTransition::Shield(ShieldTransition::V0(ref mut v0)) = st {
                v0.input_witnesses = witnesses;
            }

            let processing_result = process_transition(&platform, st, platform_version);

            // FIXED: BatchValidator now verifies binding signature and spend auth sigs.
            // Mutated value_balance changes the sighash, causing signature verification to fail.
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidShieldedProofError(_))
                )]
            );
        }

        /// Regression test for shield input mismatch inflation bug.
        ///
        /// An attacker constructs a shield transition where the transparent inputs
        /// provide far less than the declared shield `amount`. Without the
        /// `sum(inputs) >= amount` check in structure validation, the pool would be
        /// credited by `amount` while addresses are only debited by `requested_input_amount`,
        /// minting credits from nothing.
        ///
        /// With the fix, the transition is rejected at structure validation.
        ///
        /// Based on reproducer by pasta (commit a85f4b74).
        #[tokio::test]
        async fn test_rejects_shield_when_inputs_less_than_amount() {
            let platform_version = PlatformVersion::latest();
            let mut platform = setup_platform();

            let initial_address_balance = dash_to_credits!(1.0);
            let requested_input_amount = 100_000u64;
            let forged_shield_amount = 50_000_000u64;

            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([9u8; 32]);
            setup_address_with_balance(&mut platform, input_address, 0, initial_address_balance);

            let mut rng = OsRng;
            let pk = get_proving_key();

            let sk = SpendingKey::from_bytes([0u8; 32]).unwrap();
            let fvk = FullViewingKey::from(&sk);
            let recipient = fvk.address_at(0u32, Scope::External);

            let anchor = Anchor::empty_tree();
            let mut builder = Builder::<DashMemo>::new(
                BundleType::Transactional {
                    flags: OrchardFlags::SPENDS_DISABLED,
                    bundle_required: false,
                },
                anchor,
            );

            builder
                .add_output(
                    None,
                    recipient,
                    NoteValue::from_raw(forged_shield_amount),
                    [0u8; 36],
                )
                .unwrap();

            let (unauthorized, _) = builder.build::<i64>(&mut rng).unwrap().unwrap();
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &[]);
            let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
            let bundle = proven.apply_signatures(rng, sighash, &[]).unwrap();

            let (actions, _flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                serialize_authorized_bundle_with_flags(&bundle);

            assert_eq!(
                (-value_balance) as u64,
                forged_shield_amount,
                "bundle should authorize the inflated shield amount",
            );

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, requested_input_amount));

            let mut st = StateTransition::Shield(ShieldTransition::V0(ShieldTransitionV0 {
                inputs: inputs.clone(),
                actions,
                amount: forged_shield_amount,
                anchor: anchor_bytes,
                proof: proof_bytes,
                binding_signature: binding_sig,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                user_fee_increase: 0,
                input_witnesses: vec![],
            }));

            let signable_bytes = st.signable_bytes().expect("should compute signable bytes");
            let mut witnesses: Vec<AddressWitness> = Vec::with_capacity(inputs.len());
            for address in inputs.keys() {
                let witness = signer
                    .sign_create_witness(address, &signable_bytes)
                    .await
                    .expect("should sign");
                witnesses.push(witness);
            }

            if let StateTransition::Shield(ShieldTransition::V0(ref mut v0)) = st {
                v0.input_witnesses = witnesses;
            }

            let processing_result = process_transition(&mut platform, st, platform_version);

            // The transition must be rejected — inputs (100k) < shield amount (50M).
            // Structure validation catches this before any state reads or ZK proof verification.
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::ShieldedInvalidValueBalanceError(_))
                )]
            );
        }
    }

    // ==========================================
    // RETURN PROOF TESTS (prove + verify round-trip)
    // ==========================================

    mod return_proof {
        use super::*;
        use dpp::block::block_info::BlockInfo;
        use dpp::serialization::PlatformSerializable;
        use drive::drive::Drive;
        use drive::error::proof::ProofError;
        use drive::error::Error as DriveError;
        use grovedb_commitment_tree::{
            Anchor, Builder, BundleType, DashMemo, Flags as OrchardFlags, FullViewingKey,
            NoteValue, Scope, SpendingKey,
        };
        use rand::rngs::OsRng;

        #[tokio::test]
        async fn test_shield_state_proof_verifies_balances_but_not_execution() {
            let platform_version = PlatformVersion::latest();
            let mut platform = setup_platform();
            let mut rng = OsRng;
            let pk = get_proving_key();

            // --- Create keys ---
            let sk = SpendingKey::from_bytes([0u8; 32]).unwrap();
            let fvk = FullViewingKey::from(&sk);
            let recipient = fvk.address_at(0u32, Scope::External);

            // --- Build valid Orchard bundle (shield = outputs only, no spends) ---
            let anchor = Anchor::empty_tree();
            let mut builder = Builder::<DashMemo>::new(
                BundleType::Transactional {
                    flags: OrchardFlags::SPENDS_DISABLED,
                    bundle_required: false,
                },
                anchor,
            );

            let shield_value = 5_000u64;
            builder
                .add_output(
                    None,
                    recipient,
                    NoteValue::from_raw(shield_value),
                    [0u8; 36],
                )
                .unwrap();

            let (unauthorized, _) = builder.build::<i64>(&mut rng).unwrap().unwrap();
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            // Shield sighash extra_data is empty (no transparent output fields)
            let sighash = compute_platform_sighash(&bundle_commitment, &[]);
            let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
            let bundle = proven.apply_signatures(rng, sighash, &[]).unwrap();

            // --- Extract serialized fields from the authorized bundle ---
            let (actions, _flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                serialize_authorized_bundle_with_flags(&bundle);

            // value_balance should be negative for shield (money going into pool)
            assert!(value_balance < 0);
            let shield_amount = (-value_balance) as u64;

            // --- Set up input address with enough balance ---
            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            let input_amount = shield_amount + dash_to_credits!(0.01);
            setup_address_with_balance(&mut platform, input_address, 0, dash_to_credits!(1.0));

            // --- Build and sign the shield transition ---
            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, input_amount));

            let mut st = StateTransition::Shield(ShieldTransition::V0(ShieldTransitionV0 {
                inputs: inputs.clone(),
                actions,
                amount: shield_amount,
                anchor: anchor_bytes,
                proof: proof_bytes,
                binding_signature: binding_sig,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                user_fee_increase: 0,
                input_witnesses: vec![],
            }));

            let signable_bytes = st.signable_bytes().expect("should compute signable bytes");
            let mut witnesses: Vec<AddressWitness> = Vec::with_capacity(inputs.len());
            for address in inputs.keys() {
                let witness = signer
                    .sign_create_witness(address, &signable_bytes)
                    .await
                    .expect("should sign");
                witnesses.push(witness);
            }

            if let StateTransition::Shield(ShieldTransition::V0(ref mut v0)) = st {
                v0.input_witnesses = witnesses;
            }

            // --- Serialize and process with manual transaction so we can commit before proving ---
            let transition_bytes = st
                .serialize_to_bytes()
                .expect("should serialize transition");

            let platform_state = platform.state.load();
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
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );

            // Commit the transaction so prove_state_transition can read committed state
            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            // --- Generate proof ---
            let proof_result = platform
                .drive
                .prove_state_transition(&st, None, platform_version)
                .expect("expected to generate proof for shield");

            let proof_bytes = proof_result
                .into_data()
                .expect("expected proof data, not an error");

            // The state proof still authenticates the resulting address balance.
            let (root_hash, address_infos): (
                _,
                BTreeMap<PlatformAddress, Option<(AddressNonce, Credits)>>,
            ) = Drive::verify_addresses_infos(
                &proof_bytes,
                [&input_address],
                false,
                platform_version,
            )
            .expect("expected to verify the shield address state proof");

            assert_ne!(root_hash, [0u8; 32], "root hash should not be zeroed");

            assert!(
                address_infos.contains_key(&input_address),
                "proof result should contain the input address"
            );

            // The address should have a balance entry (Some) after the shield
            let address_info = address_infos
                .get(&input_address)
                .expect("input address should be in result");

            assert!(
                address_info.is_some(),
                "input address should have balance info after shield"
            );

            let (nonce_after, balance_after) = address_info.unwrap();

            // Nonce should have been incremented
            assert_eq!(nonce_after, 1, "nonce should be 1 after first shield");

            // Balance should be less than original (shield_amount + fees were deducted)
            assert!(
                balance_after < dash_to_credits!(1.0),
                "balance should be less than original after shield"
            );

            // A post-state balance alone cannot prove that this specific shield
            // transition executed; the outcome must therefore stay tagged as
            // an affected-state snapshot rather than execution evidence.
            let (_outcome_root_hash, outcome) =
                Drive::verify_state_transition_was_executed_with_proof(
                    &st,
                    &BlockInfo::default(),
                    &proof_bytes,
                    &|_| Ok(None),
                    platform_version,
                )
                .expect("shield affected-state verification should succeed");

            assert!(
                matches!(
                    outcome,
                    dpp::state_transition::proof_result::StateTransitionProofOutcome::AffectedState(
                        _
                    )
                ),
                "a shield state proof must not be treated as execution evidence, got {outcome:?}"
            );
        }
    }

    // ==========================================
    // CREDIT CONSERVATION TESTS (sum-tree balance)
    // ==========================================

    /// Regression test for the shield credit-destruction chain-halt bug.
    ///
    /// A `Shield` whose inputs declare a per-input `requested` (max contribution)
    /// larger than the shielded `amount` must NOT destroy the excess credits. The
    /// shared address-balance validation debits the FULL `requested` per input, but
    /// the pool is only credited `amount`. Without the reallocation fix in
    /// `transform_into_action_v0`, the addresses lose `Σrequested + fee` while the
    /// pool gains only `amount`, destroying `Σrequested - amount` credits and tripping
    /// the block-end sum-tree conservation check (`CorruptedCreditsNotBalanced`),
    /// which halts the chain.
    ///
    /// This drives a real shield (real Halo2 proof via the shared process-wide
    /// proving key — no fresh ~30s proof) through the FULL block pipeline via
    /// `process_state_transitions`, which runs
    /// `process_block_fees_and_validate_sum_trees`. That block-end routine calls
    /// `calculate_total_credits_balance(...).ok()` and returns the
    /// `CorruptedCreditsNotBalanced` error (the chain-halt) if the sum of all credit
    /// sub-trees no longer equals the platform total. With `verify_sum_trees`
    /// enabled (the default), the helper's `.expect("expected to process block fees")`
    /// therefore panics if the shield destroyed credits — so a passing test is a
    /// genuine block-level proof of conservation.
    mod credit_conservation {
        use super::*;
        use crate::execution::validation::state_transition::tests::process_state_transitions;
        use dpp::block::block_info::BlockInfo;
        // `Anchor`, `Builder`, `BundleType`, `DashMemo`, `OrchardFlags`, `FullViewingKey`,
        // `NoteValue`, `Scope`, `SpendingKey` and `OsRng` are already in scope via `use super::*`
        // (the parent module's imports), so they are intentionally not re-imported here.

        #[tokio::test]
        async fn test_shield_with_inputs_greater_than_amount_conserves_credits() {
            let platform_version = PlatformVersion::latest();
            let mut platform = setup_platform();

            // --- Set up input address with a generous balance ---
            // Use the system-credits variant so the fixture starts in a *balanced*
            // state (total_credits_in_platform == sum of sub-trees). The block-end
            // sum-tree validation inside `process_state_transitions` then asserts the
            // exact production invariant, faithfully reproducing the chain-halt check.
            let mut signer = TestAddressSigner::new();
            let input_address = signer.add_p2pkh([1u8; 32]);
            setup_address_with_balance_and_system_credits(
                &mut platform,
                input_address,
                0,
                dash_to_credits!(1.0),
            );

            // Platform total credits before the shield. A shield only moves credits
            // between addresses, the shielded pool, and fee pools — it never mints or
            // burns system credits — so this total must be unchanged afterwards and
            // the sum-tree conservation invariant must continue to hold.
            let credits_before = platform
                .drive
                .calculate_total_credits_balance(None, &platform_version.drive)
                .expect("should calculate total credits before shield");
            assert!(
                credits_before
                    .ok()
                    .expect("credit balance check should not overflow"),
                "credits must be balanced before the shield: {}",
                credits_before
            );

            // --- Build a valid Orchard shield bundle (outputs only) ---
            let mut rng = OsRng;
            let pk = get_proving_key();

            let sk = SpendingKey::from_bytes([0u8; 32]).unwrap();
            let fvk = FullViewingKey::from(&sk);
            let recipient = fvk.address_at(0u32, Scope::External);

            let anchor = Anchor::empty_tree();
            let mut builder = Builder::<DashMemo>::new(
                BundleType::Transactional {
                    flags: OrchardFlags::SPENDS_DISABLED,
                    bundle_required: false,
                },
                anchor,
            );

            let shield_value = 5_000u64;
            builder
                .add_output(
                    None,
                    recipient,
                    NoteValue::from_raw(shield_value),
                    [0u8; 36],
                )
                .unwrap();

            let (unauthorized, _) = builder.build::<i64>(&mut rng).unwrap().unwrap();
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &[]);
            let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
            let bundle = proven.apply_signatures(rng, sighash, &[]).unwrap();

            let (actions, _flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                serialize_authorized_bundle_with_flags(&bundle);

            assert!(value_balance < 0);
            let shield_amount = (-value_balance) as u64;
            // Action count used to price the shielded compute fee (asserted on the booked fee below).
            let num_actions_in_shield = actions.len();

            // CRITICAL: requested (max contribution) is MUCH larger than the shielded
            // amount. This is the routine builder behavior that triggered the bug: the
            // full `requested` would be debited from the address while the pool only
            // gains `shield_amount`. With the fix, only `shield_amount` is debited and
            // the excess stays in the address.
            let requested = shield_amount + dash_to_credits!(0.2);
            assert!(
                requested > shield_amount,
                "test must exercise Σinputs > amount"
            );

            let mut inputs = BTreeMap::new();
            inputs.insert(input_address, (1 as AddressNonce, requested));

            let mut st = StateTransition::Shield(ShieldTransition::V0(ShieldTransitionV0 {
                inputs: inputs.clone(),
                actions,
                amount: shield_amount,
                anchor: anchor_bytes,
                proof: proof_bytes,
                binding_signature: binding_sig,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                user_fee_increase: 0,
                input_witnesses: vec![],
            }));

            let signable_bytes = st.signable_bytes().expect("should compute signable bytes");
            let mut witnesses: Vec<AddressWitness> = Vec::with_capacity(inputs.len());
            for address in inputs.keys() {
                let witness = signer
                    .sign_create_witness(address, &signable_bytes)
                    .await
                    .expect("should sign");
                witnesses.push(witness);
            }

            if let StateTransition::Shield(ShieldTransition::V0(ref mut v0)) = st {
                v0.input_witnesses = witnesses;
            }

            // --- Run the FULL block pipeline (execute + distribute fees + validate
            //     sum trees). With `verify_sum_trees` enabled (the default),
            //     `process_block_fees_and_validate_sum_trees` would return
            //     `CorruptedCreditsNotBalanced` and the helper's `.expect(...)` would
            //     panic here if the shield destroyed credits. This is the block-level
            //     conservation assertion. ---
            let platform_state = platform.state.load();
            let (fee_results, _processed_block_fees) =
                process_state_transitions(&platform, &[st], BlockInfo::default(), &platform_state);

            // --- Assert the metered + compute fee model directly on the booked fee. ---
            // The Shield fee is `metered_storage + metered_processing + shielded_verification_fee`.
            // A positive `storage_fee` proves the metered GroveDB storage of the note/nullifier
            // writes is captured (it is not discarded in favor of a flat estimate), and
            // `processing_fee >= shielded_verification_fee` proves the ZK compute fee is added on
            // top of the metered processing.
            let shielded_verification_fee = dpp::shielded::compute_shielded_verification_fee(
                num_actions_in_shield,
                platform_version,
            )
            .expect("shielded compute fee");
            let booked = &fee_results[0];
            assert!(
                booked.storage_fee > 0,
                "metered storage of the note/nullifier writes must be captured (storage_fee > 0), got {}",
                booked.storage_fee
            );
            assert!(
                booked.processing_fee >= shielded_verification_fee,
                "processing fee ({}) must include the shielded compute fee ({}) on top of metered processing",
                booked.processing_fee,
                shielded_verification_fee
            );

            // --- Additionally assert the invariant directly post-block ---
            let credits_after = platform
                .drive
                .calculate_total_credits_balance(None, &platform_version.drive)
                .expect("should calculate total credits after shield");

            assert!(
                credits_after
                    .ok()
                    .expect("credit balance check should not overflow"),
                "credits must remain balanced after shield with Σinputs > amount \
                 (this is the invariant whose failure halts the chain): {}",
                credits_after
            );

            // The shield neither mints nor burns system credits.
            assert_eq!(
                credits_after.total_credits_in_platform, credits_before.total_credits_in_platform,
                "shield must not change total platform credits"
            );

            // The shielded pool gained exactly `shield_amount`.
            assert_eq!(
                credits_after.total_in_shielded_balances
                    - credits_before.total_in_shielded_balances,
                shield_amount as i64,
                "shielded pool must gain exactly the shield amount"
            );
        }

        /// Multi-input variant: two funded addresses with Σrequested > amount and a
        /// fee strategy pointing at input 0. The shield amount is chosen to exceed a
        /// single input's `requested`, so the reallocation must spill across BOTH
        /// inputs. Runs the full block pipeline (execute + fee distribution + sum-tree
        /// validation) and asserts conservation holds with more than one input — the
        /// path the single-input test cannot exercise.
        #[tokio::test]
        async fn test_multi_input_shield_conserves_credits() {
            let platform_version = PlatformVersion::latest();
            let mut platform = setup_platform();

            let mut signer = TestAddressSigner::new();
            let addr_a = signer.add_p2pkh([1u8; 32]);
            let addr_b = signer.add_p2pkh([2u8; 32]);
            setup_address_with_balance_and_system_credits(
                &mut platform,
                addr_a,
                0,
                dash_to_credits!(1.0),
            );
            setup_address_with_balance_and_system_credits(
                &mut platform,
                addr_b,
                0,
                dash_to_credits!(1.0),
            );

            let credits_before = platform
                .drive
                .calculate_total_credits_balance(None, &platform_version.drive)
                .expect("should calculate total credits before shield");
            assert!(
                credits_before
                    .ok()
                    .expect("credit balance check should not overflow"),
                "credits must be balanced before the shield: {}",
                credits_before
            );

            // Orchard bundle whose value is large enough that the shield amount spans
            // both inputs (forces cross-input consumption / spillover).
            let mut rng = OsRng;
            let pk = get_proving_key();
            let sk = SpendingKey::from_bytes([0u8; 32]).unwrap();
            let fvk = FullViewingKey::from(&sk);
            let recipient = fvk.address_at(0u32, Scope::External);
            let anchor = Anchor::empty_tree();
            let mut builder = Builder::<DashMemo>::new(
                BundleType::Transactional {
                    flags: OrchardFlags::SPENDS_DISABLED,
                    bundle_required: false,
                },
                anchor,
            );
            let shield_value = 15_000_000_000u64; // 0.15 DASH
            builder
                .add_output(
                    None,
                    recipient,
                    NoteValue::from_raw(shield_value),
                    [0u8; 36],
                )
                .unwrap();
            let (unauthorized, _) = builder.build::<i64>(&mut rng).unwrap().unwrap();
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &[]);
            let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
            let bundle = proven.apply_signatures(rng, sighash, &[]).unwrap();
            let (actions, _flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                serialize_authorized_bundle_with_flags(&bundle);
            assert!(value_balance < 0);
            let shield_amount = (-value_balance) as u64;

            // Each input contributes up to 0.1 DASH. Σrequested = 0.2 DASH > amount,
            // and amount (0.15) exceeds a single input's requested (0.1), so the
            // reallocation must consume from both inputs.
            let requested = dash_to_credits!(0.1);
            assert!(
                requested < shield_amount,
                "amount must span more than one input"
            );
            assert!(requested * 2 > shield_amount, "Σinputs must exceed amount");

            let mut inputs = BTreeMap::new();
            inputs.insert(addr_a, (1 as AddressNonce, requested));
            inputs.insert(addr_b, (1 as AddressNonce, requested));

            let mut st = StateTransition::Shield(ShieldTransition::V0(ShieldTransitionV0 {
                inputs: inputs.clone(),
                actions,
                amount: shield_amount,
                anchor: anchor_bytes,
                proof: proof_bytes,
                binding_signature: binding_sig,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                user_fee_increase: 0,
                input_witnesses: vec![],
            }));

            let signable_bytes = st.signable_bytes().expect("should compute signable bytes");
            let mut witnesses: Vec<AddressWitness> = Vec::with_capacity(inputs.len());
            for address in inputs.keys() {
                let witness = signer
                    .sign_create_witness(address, &signable_bytes)
                    .await
                    .expect("should sign");
                witnesses.push(witness);
            }
            if let StateTransition::Shield(ShieldTransition::V0(ref mut v0)) = st {
                v0.input_witnesses = witnesses;
            }

            let platform_state = platform.state.load();
            let (_fee_results, _processed_block_fees) =
                process_state_transitions(&platform, &[st], BlockInfo::default(), &platform_state);

            let credits_after = platform
                .drive
                .calculate_total_credits_balance(None, &platform_version.drive)
                .expect("should calculate total credits after shield");
            assert!(
                credits_after
                    .ok()
                    .expect("credit balance check should not overflow"),
                "credits must remain balanced after a multi-input shield: {}",
                credits_after
            );
            assert_eq!(
                credits_after.total_credits_in_platform, credits_before.total_credits_in_platform,
                "shield must not change total platform credits"
            );
            assert_eq!(
                credits_after.total_in_shielded_balances
                    - credits_before.total_in_shielded_balances,
                shield_amount as i64,
                "shielded pool must gain exactly the shield amount"
            );
        }
    }

    /// MAINNET HALT INVESTIGATION (evo1, 2026-08-14/15: ~2h stalls after 415652 and after 415661).
    ///
    /// `execute_event_v0` validates a `Shield`'s fee TWICE against two DIFFERENT cost models:
    /// `validate_fees_of_event` estimates with `apply_drive_operations(.., apply=false, ..)`, which
    /// prices the batch from a SYNTHETIC layer model, while `paid_from_address_inputs_and_outputs`
    /// re-meters with `apply=true` against the REAL tree. If the synthetic estimate can come in
    /// LOWER than the real cost, there is a band of funding levels where validation ACCEPTS the
    /// transition and execution then rejects it — and that rejection (`CorruptedCodeExecution`,
    /// "address-input fee not fully covered at execution") happens AFTER the shield's writes were
    /// already applied to the shared block transaction, with no per-transition rollback.
    ///
    /// These tests answer, empirically: does such a band exist, and does a transition landing in it
    /// leave state behind?
    mod mainnet_halt_repro {
        use super::*;
        use crate::execution::validation::state_transition::state_transitions::test_helpers::insert_dummy_encrypted_notes;
        use dpp::block::block_info::BlockInfo;

        /// Note count on mainnet's shielded commitment tree around the halt.
        const MAINNET_NOTES: u64 = 494;

        #[derive(Debug, PartialEq, Eq, Clone, Copy)]
        enum Outcome {
            Success,
            NotEnoughFunds,
            Internal,
            Other,
        }

        /// The reusable pieces of a valid shield bundle. Orchard proof generation dominates the
        /// runtime, so it is done once and the transition re-signed per funding level.
        struct Bundle {
            actions: Vec<SerializedAction>,
            shield_amount: u64,
            anchor: [u8; 32],
            proof: Vec<u8>,
            binding_sig: [u8; 64],
        }

        fn build_bundle() -> Bundle {
            let mut rng = OsRng;
            let pk = get_proving_key();
            let sk = SpendingKey::from_bytes([0u8; 32]).unwrap();
            let fvk = FullViewingKey::from(&sk);
            let recipient = fvk.address_at(0u32, Scope::External);

            let mut builder = Builder::<DashMemo>::new(
                BundleType::Transactional {
                    flags: OrchardFlags::SPENDS_DISABLED,
                    bundle_required: false,
                },
                Anchor::empty_tree(),
            );
            builder
                .add_output(None, recipient, NoteValue::from_raw(5000u64), [0u8; 36])
                .unwrap();
            let (unauthorized, _) = builder.build::<i64>(&mut rng).unwrap().unwrap();
            let commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&commitment, &[]);
            let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
            let bundle = proven.apply_signatures(rng, sighash, &[]).unwrap();

            let (actions, _flags, value_balance, anchor, proof, binding_sig) =
                serialize_authorized_bundle_with_flags(&bundle);
            assert!(
                value_balance < 0,
                "a shield must have negative value balance"
            );
            Bundle {
                actions,
                shield_amount: (-value_balance) as u64,
                anchor,
                proof,
                binding_sig,
            }
        }

        /// Build a signed `Shield` spending `declared_input` credits from `addr`.
        async fn build_signed(
            b: &Bundle,
            signer: &TestAddressSigner,
            addr: PlatformAddress,
            declared_input: u64,
        ) -> StateTransition {
            let mut inputs = BTreeMap::new();
            inputs.insert(addr, (1 as AddressNonce, declared_input));

            let mut st = StateTransition::Shield(ShieldTransition::V0(ShieldTransitionV0 {
                inputs: inputs.clone(),
                actions: b.actions.clone(),
                amount: b.shield_amount,
                anchor: b.anchor,
                proof: b.proof.clone(),
                binding_signature: b.binding_sig,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                user_fee_increase: 0,
                input_witnesses: vec![],
            }));
            let signable = st.signable_bytes().expect("should compute signable bytes");
            let mut witnesses: Vec<AddressWitness> = Vec::with_capacity(inputs.len());
            for a in inputs.keys() {
                witnesses.push(
                    signer
                        .sign_create_witness(a, &signable)
                        .await
                        .expect("sign"),
                );
            }
            if let StateTransition::Shield(ShieldTransition::V0(ref mut v0)) = st {
                v0.input_witnesses = witnesses;
            }
            st
        }

        /// Run the shield on a fresh platform whose input address holds exactly
        /// `shield_amount + headroom`, i.e. `headroom` credits are available to pay the fee.
        async fn run_at(headroom: u64, b: &Bundle, pv: &PlatformVersion) -> (Outcome, String) {
            let mut platform = setup_platform();
            insert_dummy_encrypted_notes(&platform, MAINNET_NOTES);

            let mut signer = TestAddressSigner::new();
            let addr = signer.add_p2pkh([1u8; 32]);
            let declared_input = b.shield_amount + headroom;
            setup_address_with_balance_and_system_credits(&mut platform, addr, 0, declared_input);

            let st = build_signed(b, &signer, addr, declared_input).await;
            let result = process_transition(&platform, st, pv);
            match result.execution_results().first() {
                Some(StateTransitionExecutionResult::SuccessfulExecution { .. }) => {
                    (Outcome::Success, String::new())
                }
                Some(StateTransitionExecutionResult::InternalError(msg)) => {
                    (Outcome::Internal, msg.clone())
                }
                Some(StateTransitionExecutionResult::UnpaidConsensusError(e)) => {
                    let rendered = format!("{:?}", e);
                    if rendered.contains("AddressesNotEnoughFunds") {
                        (Outcome::NotEnoughFunds, rendered)
                    } else {
                        (Outcome::Other, rendered)
                    }
                }
                other => (Outcome::Other, format!("{:?}", other)),
            }
        }

        /// Binary-search both edges of the funding range to measure the band where
        /// `validate_fees_of_event` accepts a shield that execution then rejects.
        ///
        /// * lower edge = the ESTIMATED fee (below it, validation rejects cleanly)
        /// * upper edge = the ACTUAL metered fee (at or above it, the shield executes)
        ///
        /// If the two cost models agreed, the edges would coincide and the band would be empty.
        /// Any width is a range of funding levels where the transition is accepted by validation
        /// and then dropped at execution — no longer a chain halt since the proposer-side
        /// per-transition rollback (shipped in 4.1.1), but still a transition that can never
        /// confirm despite paying the quoted fee.
        ///
        /// Ignored until the estimation gap is closed: the estimated-cost path skips the keyless
        /// commitment-tree append entirely (dashpay/grovedb#812), so the band is measurably open
        /// (18,919,200 credits, 10.7% of the fee, at 494 notes). Re-enable with the grovedb pin
        /// bump that fixes it. Also ~40 full Orchard proving runs, so keep it out of routine CI.
        #[ignore = "open until the grovedb#812 estimator fix is pinned; ~40 Orchard proving runs"]
        #[tokio::test]
        async fn shield_fee_estimate_and_actual_must_not_leave_a_halting_band() {
            let pv = PlatformVersion::latest();
            let b = build_bundle();
            const CEILING: u64 = 5_000_000_000; // 0.05 DASH, far above any plausible shield fee

            let (top, top_msg) = run_at(CEILING, &b, pv).await;
            assert_eq!(
                top,
                Outcome::Success,
                "sanity: the upper bound must comfortably fund the shield ({top_msg})"
            );

            // Upper edge: least headroom that actually executes == the ACTUAL metered fee.
            let (mut lo, mut hi) = (0u64, CEILING);
            while lo + 1 < hi {
                let mid = lo + (hi - lo) / 2;
                if run_at(mid, &b, pv).await.0 == Outcome::Success {
                    hi = mid;
                } else {
                    lo = mid;
                }
            }
            let actual_fee = hi;

            // Lower edge: below the flat structural minimum shielded fee the transition is
            // rejected in BASIC validation (`ShieldedInvalidValueBalanceError`) before any write,
            // which is safe. The dangerous band starts where that gate stops rejecting.
            let (mut lo2, mut hi2) = (0u64, actual_fee);
            while lo2 + 1 < hi2 {
                let mid = lo2 + (hi2 - lo2) / 2;
                if run_at(mid, &b, pv).await.0 == Outcome::Internal {
                    hi2 = mid;
                } else {
                    lo2 = mid;
                }
            }
            let band_start = hi2;

            let (edge_outcome, edge_msg) = run_at(band_start - 1, &b, pv).await;

            println!("shield_amount   = {}", b.shield_amount);
            println!("band start      = {band_start}  (first headroom that reaches execution)");
            println!("actual fee      = {actual_fee}  (execution, apply=true)");
            println!("just below band = {edge_outcome:?} :: {edge_msg}");
            println!(
                "HALTING BAND    = [{band_start}, {actual_fee})  width = {} credits ({:.1}% of the fee)",
                actual_fee - band_start,
                100.0 * (actual_fee - band_start) as f64 / actual_fee as f64
            );

            assert_eq!(
                band_start, actual_fee,
                "HALTING BAND: any shield whose fee headroom falls in [{band_start}, {actual_fee}) \
                 clears both the structural minimum-fee gate and validate_fees_of_event (which \
                 prices the batch with the synthetic apply=false cost model, and which SKIPS the \
                 keyless commitment-tree append entirely), and is then REJECTED by \
                 paid_from_address_inputs_and_outputs on the real apply=true cost — after its drive \
                 operations were already written to the block transaction. Such a transition is \
                 stripped from the block as TxAction::Removed while its writes remain in the \
                 proposer's app hash, so no validator can reproduce that hash."
            );
        }

        /// The consequence: a transition rejected this way is reported as `InternalError`, which
        /// `prepare_proposal` maps to `TxAction::Removed` — Tenderdash strips it from the gossiped
        /// block. But nothing rolls its writes back, so the proposer's app hash (computed over the
        /// block transaction) reflects a shield the block does not contain. Validators replaying
        /// the block without it compute a different app hash and can never agree.
        ///
        /// This pins the invariant that a dropped transition must not mutate state.
        #[tokio::test]
        async fn dropped_shield_must_not_mutate_state() {
            let pv = PlatformVersion::latest();
            let b = build_bundle();
            // Sits inside the measured band: accepted by validation, rejected by execution.
            let headroom = 177_215_759u64;

            let mut platform = setup_platform();
            insert_dummy_encrypted_notes(&platform, MAINNET_NOTES);
            let mut signer = TestAddressSigner::new();
            let addr = signer.add_p2pkh([1u8; 32]);
            let declared_input = b.shield_amount + headroom;
            setup_address_with_balance_and_system_credits(&mut platform, addr, 0, declared_input);

            let pool_before = platform
                .drive
                .read_shielded_pool_total_balance(None, &mut vec![], pv)
                .expect("pool balance");
            let notes_before = platform
                .drive
                .shielded_pool_notes_count(None, &mut vec![], pv)
                .expect("notes count");
            let hash_before = platform
                .drive
                .grove
                .root_hash(None, &pv.drive.grove_version)
                .unwrap()
                .expect("root hash");

            let st = build_signed(&b, &signer, addr, declared_input).await;
            let bytes = st.serialize_to_bytes().expect("serialize");
            let state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();
            let result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![bytes],
                    &state,
                    &BlockInfo::default(),
                    &transaction,
                    pv,
                    true, // proposing, exactly as prepare_proposal does
                    None,
                )
                .expect("processing must not be a block-level error");

            let dropped = matches!(
                result.execution_results().first(),
                Some(StateTransitionExecutionResult::InternalError(_))
            );
            assert!(
                dropped,
                "expected the mid-band shield to be dropped as InternalError, got {:?}",
                result.execution_results()
            );

            let pool_after = platform
                .drive
                .read_shielded_pool_total_balance(Some(&transaction), &mut vec![], pv)
                .expect("pool balance");
            let notes_after = platform
                .drive
                .shielded_pool_notes_count(Some(&transaction), &mut vec![], pv)
                .expect("notes count");
            let hash_after = platform
                .drive
                .grove
                .root_hash(Some(&transaction), &pv.drive.grove_version)
                .unwrap()
                .expect("root hash");

            println!("shielded pool : {pool_before} -> {pool_after}");
            println!("notes in tree : {notes_before} -> {notes_after}");
            println!(
                "app hash      : {} -> {}",
                hex::encode(hash_before),
                hex::encode(hash_after)
            );

            assert_eq!(
                pool_after,
                pool_before,
                "STATE LEAK: a transition that was DROPPED from the block still credited the \
                 shielded pool by {}. Tenderdash gossips the block without this transition, so \
                 every other validator computes an app hash without this credit.",
                pool_after.saturating_sub(pool_before)
            );
            assert_eq!(
                notes_after, notes_before,
                "STATE LEAK: a dropped transition still appended a note commitment"
            );
            assert_eq!(
                hash_before, hash_after,
                "APP HASH DIVERGENCE: a dropped transition changed the proposer's app hash. \
                 Validators replaying the gossiped block (which excludes it) cannot reproduce this \
                 hash, so the proposal is rejected every round and the chain stalls."
            );
        }

        /// Can a per-transition savepoint undo a shield's writes mid-transaction?
        ///
        /// The candidate fix for the leak above is `set_savepoint()` before each transition and
        /// `rollback_to_savepoint()` on failure, inside `process_raw_state_transitions`. That is
        /// only sound if a rollback restores everything the apply touched — including GroveDB's
        /// in-memory Merk state, not just the RocksDB write batch. This runs a fully-funded shield
        /// (so the writes are the same ones a mid-band shield leaks), rolls it back, and checks
        /// pool balance, note count, and root hash against the pre-apply snapshot.
        #[tokio::test]
        async fn savepoint_rollback_must_undo_an_applied_shield() {
            let pv = PlatformVersion::latest();
            let b = build_bundle();
            // Generous headroom: this shield must SUCCEED so its writes all land.
            let headroom = 5_000_000_000u64;

            let mut platform = setup_platform();
            insert_dummy_encrypted_notes(&platform, MAINNET_NOTES);
            let mut signer = TestAddressSigner::new();
            let addr = signer.add_p2pkh([1u8; 32]);
            let declared_input = b.shield_amount + headroom;
            setup_address_with_balance_and_system_credits(&mut platform, addr, 0, declared_input);

            let st = build_signed(&b, &signer, addr, declared_input).await;
            let bytes = st.serialize_to_bytes().expect("serialize");
            let state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let pool_before = platform
                .drive
                .read_shielded_pool_total_balance(Some(&transaction), &mut vec![], pv)
                .expect("pool balance");
            let notes_before = platform
                .drive
                .shielded_pool_notes_count(Some(&transaction), &mut vec![], pv)
                .expect("notes count");
            let hash_before = platform
                .drive
                .grove
                .root_hash(Some(&transaction), &pv.drive.grove_version)
                .unwrap()
                .expect("root hash");

            // Note on savepoint provenance: while proposing at a non-genesis height the
            // processing loop sets its OWN savepoint (recording this same state — nothing is
            // written in between) on top of this one, and leaves it on the stack for a kept
            // transition. The `rollback_to_savepoint()` below therefore pops the LOOP's
            // savepoint, not this one, which stays on the stack unused. Both record identical
            // state, so every assertion is unaffected; this savepoint documents the mechanism
            // under test and kept the test meaningful before the loop rolled back on its own.
            transaction.set_savepoint();

            let result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![bytes],
                    &state,
                    &BlockInfo::default(),
                    &transaction,
                    pv,
                    true,
                    None,
                )
                .expect("processing must not be a block-level error");
            assert!(
                matches!(
                    result.execution_results().first(),
                    Some(StateTransitionExecutionResult::SuccessfulExecution { .. })
                ),
                "sanity: the fully-funded shield must execute, got {:?}",
                result.execution_results()
            );

            let hash_applied = platform
                .drive
                .grove
                .root_hash(Some(&transaction), &pv.drive.grove_version)
                .unwrap()
                .expect("root hash");
            assert_ne!(
                hash_before, hash_applied,
                "sanity: the applied shield must have changed the root hash"
            );

            transaction
                .rollback_to_savepoint()
                .expect("rollback to savepoint");

            let pool_after = platform
                .drive
                .read_shielded_pool_total_balance(Some(&transaction), &mut vec![], pv)
                .expect("pool balance");
            let notes_after = platform
                .drive
                .shielded_pool_notes_count(Some(&transaction), &mut vec![], pv)
                .expect("notes count");
            let hash_after = platform
                .drive
                .grove
                .root_hash(Some(&transaction), &pv.drive.grove_version)
                .unwrap()
                .expect("root hash");

            println!("shielded pool : {pool_before} -> {pool_after} (want {pool_before})");
            println!("notes in tree : {notes_before} -> {notes_after} (want {notes_before})");
            println!(
                "root hash     : applied {} -> rolled back {} (want {})",
                hex::encode(hash_applied),
                hex::encode(hash_after),
                hex::encode(hash_before)
            );

            assert_eq!(
                pool_after, pool_before,
                "savepoint rollback did not undo the shielded pool credit"
            );
            assert_eq!(
                notes_after, notes_before,
                "savepoint rollback did not undo the note commitment append"
            );
            assert_eq!(
                hash_before, hash_after,
                "savepoint rollback did not restore the root hash: GroveDB's in-memory Merk \
                 state survives a RocksDB-level rollback, so a per-transition savepoint is NOT a \
                 sound implementation of the leak fix (use proposal re-execution instead)"
            );

            // Rollback restoring READS is necessary but not sufficient: in the fix, the next
            // transition in the block applies onto the rolled-back transaction. If any in-memory
            // Merk state survived the rollback, that second apply would build on phantom nodes and
            // diverge. Re-applying the identical shield onto the restored state is deterministic,
            // so it must reproduce the first apply's root hash exactly.
            let bytes_again = st.serialize_to_bytes().expect("serialize");
            let result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![bytes_again],
                    &state,
                    &BlockInfo::default(),
                    &transaction,
                    pv,
                    true,
                    None,
                )
                .expect("processing must not be a block-level error");
            assert!(
                matches!(
                    result.execution_results().first(),
                    Some(StateTransitionExecutionResult::SuccessfulExecution { .. })
                ),
                "the shield must execute again on the rolled-back state (its nonce and balance \
                 were restored), got {:?}",
                result.execution_results()
            );
            let hash_reapplied = platform
                .drive
                .grove
                .root_hash(Some(&transaction), &pv.drive.grove_version)
                .unwrap()
                .expect("root hash");
            println!(
                "root hash     : re-applied {} (want {})",
                hex::encode(hash_reapplied),
                hex::encode(hash_applied)
            );
            assert_eq!(
                hash_applied, hash_reapplied,
                "applying onto a rolled-back transaction diverged from applying onto the \
                 original state: stale in-memory Merk state survived the rollback, so a \
                 per-transition savepoint is NOT a sound implementation of the leak fix"
            );
        }

        /// Generic leak guard, independent of the fee bug: force a fully-funded shield — one
        /// that executes successfully and therefore definitely wrote — to be reported as
        /// `InternalError` after the fact, and assert the processing loop rolls its writes
        /// back. An `InternalError` maps to `TxAction::Removed`, so ANY path that produces one
        /// after `apply_drive_operations(apply = true)` must leave no trace in the state. This
        /// pins the proposer-side per-transition rollback even after the estimation gap
        /// (dashpay/grovedb#812) is closed and no real transition can reach execution
        /// under-funded anymore.
        #[tokio::test]
        async fn injected_post_apply_failure_must_not_mutate_state() {
            use crate::execution::platform_events::state_transition_processing::test_fault_injection::FAIL_NEXT_SUCCESSFUL_EXECUTION;

            let pv = PlatformVersion::latest();
            let b = build_bundle();
            // Fully funded: without the injected failure this shield would execute and land.
            let headroom = 5_000_000_000u64;

            let mut platform = setup_platform();
            insert_dummy_encrypted_notes(&platform, MAINNET_NOTES);
            let mut signer = TestAddressSigner::new();
            let addr = signer.add_p2pkh([1u8; 32]);
            let declared_input = b.shield_amount + headroom;
            setup_address_with_balance_and_system_credits(&mut platform, addr, 0, declared_input);

            let st = build_signed(&b, &signer, addr, declared_input).await;
            let bytes = st.serialize_to_bytes().expect("serialize");
            let state = platform.state.load();
            let transaction = platform.drive.grove.start_transaction();

            let pool_before = platform
                .drive
                .read_shielded_pool_total_balance(Some(&transaction), &mut vec![], pv)
                .expect("pool balance");
            let notes_before = platform
                .drive
                .shielded_pool_notes_count(Some(&transaction), &mut vec![], pv)
                .expect("notes count");
            let hash_before = platform
                .drive
                .grove
                .root_hash(Some(&transaction), &pv.drive.grove_version)
                .unwrap()
                .expect("root hash");

            FAIL_NEXT_SUCCESSFUL_EXECUTION.with(|flag| flag.set(true));

            let result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![bytes],
                    &state,
                    &BlockInfo::default(),
                    &transaction,
                    pv,
                    true,
                    None,
                )
                .expect("processing must not be a block-level error");

            assert!(
                !FAIL_NEXT_SUCCESSFUL_EXECUTION.with(|flag| flag.get()),
                "sanity: the injection must have been consumed (the shield must have executed \
                 successfully before being overridden)"
            );
            assert!(
                matches!(
                    result.execution_results().first(),
                    Some(StateTransitionExecutionResult::InternalError(_))
                ),
                "expected the injected InternalError, got {:?}",
                result.execution_results()
            );

            let pool_after = platform
                .drive
                .read_shielded_pool_total_balance(Some(&transaction), &mut vec![], pv)
                .expect("pool balance");
            let notes_after = platform
                .drive
                .shielded_pool_notes_count(Some(&transaction), &mut vec![], pv)
                .expect("notes count");
            let hash_after = platform
                .drive
                .grove
                .root_hash(Some(&transaction), &pv.drive.grove_version)
                .unwrap()
                .expect("root hash");

            assert_eq!(
                pool_after, pool_before,
                "STATE LEAK: a transition reported InternalError kept its shielded pool credit"
            );
            assert_eq!(
                notes_after, notes_before,
                "STATE LEAK: a transition reported InternalError kept its note commitments"
            );
            assert_eq!(
                hash_before, hash_after,
                "APP HASH DIVERGENCE: a transition reported InternalError (and therefore \
                 stripped from the block as TxAction::Removed) changed the root hash"
            );
        }
    }
}
