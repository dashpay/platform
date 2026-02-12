#[cfg(test)]
mod tests {
    use crate::execution::validation::state_transition::state_transitions::shielded_common::compute_platform_sighash;
    use crate::execution::validation::state_transition::state_transitions::test_helpers::{
        create_dummy_serialized_action, create_dummy_witness, create_platform_address,
        process_transition, setup_address_with_balance, setup_platform, TestAddressSigner,
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
        Anchor, Authorized as OrchardAuthorized, Builder, Bundle, BundleType,
        Flags as OrchardFlags, FullViewingKey, NoteValue, ProvingKey, Scope, SpendingKey,
    };
    use platform_version::version::PlatformVersion;
    use rand::rngs::OsRng;
    use std::collections::BTreeMap;
    use std::sync::OnceLock;

    // ==========================================
    // Helper Functions (transition-specific)
    // ==========================================

    /// Builds a raw `ShieldTransitionV0` with dummy witnesses. Used for structure validation tests
    /// that don't need valid signatures (the structure error is caught before or alongside witness
    /// validation, or inputs are empty so witness validation is vacuously true).
    fn create_raw_shield_transition(
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        actions: Vec<SerializedAction>,
        flags: u8,
        value_balance: i64,
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
        binding_signature: [u8; 64],
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
            [0u8; 64],      // dummy binding signature
            AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(0)]),
        )
    }

    // ==========================================
    // Orchard Proving Key (cached, ~30s to build)
    // ==========================================

    static TEST_PROVING_KEY: OnceLock<ProvingKey> = OnceLock::new();
    fn get_proving_key() -> &'static ProvingKey {
        TEST_PROVING_KEY.get_or_init(ProvingKey::build)
    }

    /// Extract serialized fields from an authorized Orchard bundle into the
    /// platform-compatible format: (actions, flags, value_balance, anchor, proof, binding_sig).
    fn serialize_authorized_bundle(
        bundle: &Bundle<OrchardAuthorized, i64>,
    ) -> (Vec<SerializedAction>, u8, i64, [u8; 32], Vec<u8>, [u8; 64]) {
        let actions: Vec<SerializedAction> = bundle
            .actions()
            .iter()
            .map(|action| {
                let enc = action.encrypted_note();
                let mut encrypted_note = Vec::with_capacity(692);
                encrypted_note.extend_from_slice(&enc.epk_bytes);
                encrypted_note.extend_from_slice(&enc.enc_ciphertext);
                encrypted_note.extend_from_slice(&enc.out_ciphertext);

                SerializedAction {
                    nullifier: action.nullifier().to_bytes(),
                    rk: <[u8; 32]>::from(action.rk()),
                    cmx: action.cmx().to_bytes(),
                    encrypted_note,
                    cv_net: action.cv_net().to_bytes(),
                    spend_auth_sig: <[u8; 64]>::from(action.authorization()),
                }
            })
            .collect();

        let flags = bundle.flags().to_byte();
        let value_balance = *bundle.value_balance();
        let anchor = bundle.anchor().to_bytes();
        let proof = bundle.authorization().proof().as_ref().to_vec();
        let binding_sig = <[u8; 64]>::from(bundle.authorization().binding_signature());

        (actions, flags, value_balance, anchor, proof, binding_sig)
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
                [0u8; 64],
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::ShieldedNoActionsError(_))
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
                [0u8; 64],
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
                [0u8; 64],
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::ShieldedInvalidValueBalanceError(_))
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
                [0u8; 64],
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::ShieldedInvalidValueBalanceError(_))
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
                [0u8; 64],
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::ShieldedEmptyProofError(_))
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
                [0u8; 64],
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
                [0u8; 64],
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
                [0u8; 64],
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
                [0u8; 64],
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
        fn test_valid_shield_proof_succeeds() {
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
            let mut builder = Builder::new(
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
                    [0u8; 512],
                )
                .unwrap();

            let (unauthorized, _) = builder.build::<i64>(&mut rng).unwrap().unwrap();
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &[]);
            let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
            let bundle = proven.apply_signatures(rng, sighash, &[]).unwrap();

            // --- Extract serialized fields from the authorized bundle ---
            let (actions, flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                serialize_authorized_bundle(&bundle);

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
                flags,
                value_balance,
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
            let witnesses: Vec<AddressWitness> = inputs
                .keys()
                .map(|address| {
                    signer
                        .sign_create_witness(address, &signable_bytes)
                        .expect("should sign")
                })
                .collect();

            if let StateTransition::Shield(ShieldTransition::V0(ref mut v0)) = st {
                v0.input_witnesses = witnesses;
            }

            let processing_result = process_transition(&platform, st, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
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
                [0u8; 64],
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

    // ==========================================
    // SECURITY AUDIT TESTS
    // ==========================================

    mod security_audit {
        use super::*;

        /// AUDIT FIX VERIFICATION: `value_balance = i64::MIN` no longer panics.
        ///
        /// Previously, `(-v0.value_balance) as u64` with i64::MIN caused an
        /// overflow panic. Now uses `checked_neg()` which returns a consensus
        /// error instead.
        #[test]
        fn test_value_balance_i64_min_returns_consensus_error() {
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
                i64::MIN, // -9223372036854775808 — would overflow on negation
                vec![0u8; 100],
                [0u8; 64],
                AddressFundsFeeStrategy::from(vec![AddressFundsFeeStrategyStep::DeductFromInput(
                    0,
                )]),
            );

            // Should return a consensus error, not panic
            let processing_result = process_transition(&platform, transition, platform_version);
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidShieldedProofError(_))
                )]
            );
        }

        /// AUDIT FIX VERIFICATION: Mutated value_balance is now rejected.
        ///
        /// Previously, the binding signature was not verified so mutating
        /// value_balance from -5000 to -100000 was accepted. Now with
        /// BatchValidator, the changed value_balance produces a different
        /// bundle commitment (sighash), causing signature verification to fail.
        #[test]
        fn test_valid_proof_with_mutated_value_balance_is_rejected() {
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
            let mut builder = Builder::new(
                BundleType::Transactional {
                    flags: OrchardFlags::SPENDS_DISABLED,
                    bundle_required: false,
                },
                anchor,
            );

            builder
                .add_output(None, recipient, NoteValue::from_raw(5_000), [0u8; 512])
                .unwrap();

            let (unauthorized, _) = builder.build::<i64>(&mut rng).unwrap().unwrap();
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &[]);
            let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
            let bundle = proven.apply_signatures(rng, sighash, &[]).unwrap();

            let (actions, flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                serialize_authorized_bundle(&bundle);

            assert!(value_balance < 0);
            let honest_shield_amount = (-value_balance) as u64;
            assert_eq!(honest_shield_amount, 5_000);

            // ATTACK: Mutate value_balance to claim shielding 100,000 instead of 5,000
            let mutated_value_balance = -100_000i64;

            // Input only provides enough for a small amount, but shield_amount
            // comes from value_balance, not from inputs
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
                flags,
                value_balance: mutated_value_balance, // MUTATED
                anchor: anchor_bytes, // Must match the proof's anchor (circuit instance)
                proof: proof_bytes,
                binding_signature: binding_sig,
                fee_strategy: AddressFundsFeeStrategy::from(vec![
                    AddressFundsFeeStrategyStep::DeductFromInput(0),
                ]),
                user_fee_increase: 0,
                input_witnesses: vec![],
            }));

            let signable_bytes = st.signable_bytes().expect("should compute signable bytes");
            let witnesses: Vec<AddressWitness> = inputs
                .keys()
                .map(|address| {
                    signer
                        .sign_create_witness(address, &signable_bytes)
                        .expect("should sign")
                })
                .collect();

            if let StateTransition::Shield(ShieldTransition::V0(ref mut v0)) = st {
                v0.input_witnesses = witnesses;
            }

            let processing_result = process_transition(&platform, st, platform_version);

            // FIXED: BatchValidator now verifies binding signature and spend auth sigs.
            // Mutated value_balance changes the sighash, causing signature verification to fail.
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
