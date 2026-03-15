#[cfg(test)]
mod tests {
    use crate::execution::validation::state_transition::state_transitions::shielded_common::compute_platform_sighash;
    use crate::execution::validation::state_transition::state_transitions::test_helpers::{
        create_dummy_serialized_action, insert_anchor_into_state, insert_nullifier_into_state,
        process_transition, set_pool_total_balance, setup_platform,
    };
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
    use assert_matches::assert_matches;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;
    use dpp::shielded::SerializedAction;
    use dpp::state_transition::shielded_transfer_transition::v0::ShieldedTransferTransitionV0;
    use dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition;
    use dpp::state_transition::StateTransition;
    use platform_version::version::PlatformVersion;

    // ==========================================
    // Helper Functions (transition-specific)
    // ==========================================

    /// Builds a `ShieldedTransferTransition` state transition.
    /// No signing needed since shielded transfers have no witnesses.
    fn create_shielded_transfer_transition(
        actions: Vec<SerializedAction>,
        value_balance: u64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
    ) -> StateTransition {
        StateTransition::ShieldedTransfer(ShieldedTransferTransition::V0(
            ShieldedTransferTransitionV0 {
                actions,
                value_balance,
                anchor,
                proof,
                binding_signature,
            },
        ))
    }

    /// Shorthand for creating a structurally valid (but cryptographically invalid) shielded
    /// transfer transition. Has a non-zero anchor, valid field sizes, but random data.
    /// Includes sufficient fee to pass the minimum shielded fee check (1 action = 111,548,800).
    fn create_default_shielded_transfer_transition() -> StateTransition {
        create_shielded_transfer_transition(
            vec![create_dummy_serialized_action()],
            111_548_800,    // minimum fee for 1 action
            [42u8; 32],     // non-zero anchor
            vec![0u8; 100], // dummy proof bytes
            [0u8; 64],      // dummy binding signature
        )
    }

    // ==========================================
    // STRUCTURE VALIDATION TESTS (BasicError)
    // ==========================================

    mod structure_validation {
        use super::*;

        #[test]
        fn test_empty_actions_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let transition = create_shielded_transfer_transition(
                vec![], // Empty actions — invalid
                0,
                [42u8; 32],
                vec![0u8; 100],
                [0u8; 64],
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::ShieldedNoActionsError(_))
                )]
            );
        }

        /// Tests validate_structure directly because 101 actions exceed the
        /// max_state_transition_size (20KB) before reaching the actions count check
        /// in the full pipeline.
        #[test]
        fn test_too_many_actions_returns_error() {
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            // 101 actions exceeds max_shielded_transition_actions (100)
            let actions: Vec<SerializedAction> =
                (0..101).map(|_| create_dummy_serialized_action()).collect();

            let transition = ShieldedTransferTransitionV0 {
                actions,
                value_balance: 111_548_800,
                anchor: [42u8; 32],
                proof: vec![0u8; 100],
                binding_signature: [0u8; 64],
            };

            let result = transition.validate_structure(platform_version);

            assert_matches!(
                result.errors.as_slice(),
                [ConsensusError::BasicError(
                    BasicError::ShieldedTooManyActionsError(_)
                )]
            );
        }

        #[test]
        fn test_value_balance_exceeding_i64_max_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let transition = create_shielded_transfer_transition(
                vec![create_dummy_serialized_action()],
                i64::MAX as u64 + 1, // Exceeds i64::MAX — invalid
                [42u8; 32],
                vec![0u8; 100],
                [0u8; 64],
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
            let platform = setup_platform();

            let transition = create_shielded_transfer_transition(
                vec![create_dummy_serialized_action()],
                1, // non-zero so we don't hit value_balance == 0 rejection first
                [42u8; 32],
                vec![], // Empty proof — invalid
                [0u8; 64],
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
        fn test_zero_anchor_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let transition = create_shielded_transfer_transition(
                vec![create_dummy_serialized_action()],
                1,         // non-zero so we don't hit value_balance == 0 rejection first
                [0u8; 32], // All zeros — invalid
                vec![0u8; 100],
                [0u8; 64],
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::ShieldedZeroAnchorError(_))
                )]
            );
        }
    }

    // ==========================================
    // ANCHOR VALIDATION TESTS (StateError)
    // ==========================================

    mod anchor_validation {
        use super::*;

        #[test]
        fn test_invalid_anchor_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            // Non-zero anchor that doesn't exist in state
            let transition = create_default_shielded_transfer_transition();

            let processing_result = process_transition(&platform, transition, platform_version);

            // Proof verification now runs before anchor validation, so the
            // dummy proof data is rejected before the anchor check is reached.
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidShieldedProofError(_))
                )]
            );
        }
    }

    // ==========================================
    // NULLIFIER DOUBLE-SPEND TESTS (StateError)
    // ==========================================

    mod nullifier_validation {
        use super::*;

        #[test]
        fn test_nullifier_already_spent_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let anchor = [42u8; 32];
            let nullifier = [1u8; 32]; // Same as create_dummy_serialized_action().nullifier

            // Insert the anchor so anchor validation passes
            insert_anchor_into_state(&platform, &anchor);

            // Insert the nullifier so it appears already spent
            insert_nullifier_into_state(&platform, &nullifier);

            let transition = create_default_shielded_transfer_transition();

            let processing_result = process_transition(&platform, transition, platform_version);

            // Proof verification now runs before nullifier validation, so the
            // dummy proof data is rejected before the nullifier check is reached.
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidShieldedProofError(_))
                )]
            );
        }
    }

    // ==========================================
    // ZK PROOF VERIFICATION TESTS (InvalidShieldedProofError)
    // ==========================================

    mod proof_verification {
        use super::*;
        use grovedb_commitment_tree::{
            Anchor, Authorized as OrchardAuthorized, Builder, Bundle, BundleType,
            ClientMemoryCommitmentTree, DashMemo, ExtractedNoteCommitment, FullViewingKey,
            MerklePath, Note, NoteValue, Position, ProvingKey, RandomSeed, Retention, Rho, Scope,
            SpendAuthorizingKey, SpendingKey,
        };
        use rand::rngs::OsRng;
        use std::sync::OnceLock;

        static TEST_PROVING_KEY: OnceLock<ProvingKey> = OnceLock::new();
        fn get_proving_key() -> &'static ProvingKey {
            TEST_PROVING_KEY.get_or_init(ProvingKey::build)
        }

        fn serialize_authorized_bundle(
            bundle: &Bundle<OrchardAuthorized, i64, DashMemo>,
        ) -> (Vec<SerializedAction>, u64, [u8; 32], Vec<u8>, [u8; 64]) {
            let actions: Vec<SerializedAction> = bundle
                .actions()
                .iter()
                .map(|action| {
                    let enc = action.encrypted_note();
                    let mut encrypted_note = Vec::with_capacity(216);
                    encrypted_note.extend_from_slice(&enc.epk_bytes);
                    encrypted_note.extend_from_slice(enc.enc_ciphertext.as_ref());
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
            let value_balance = *bundle.value_balance() as u64;
            let anchor = bundle.anchor().to_bytes();
            let proof = bundle.authorization().proof().as_ref().to_vec();
            let binding_sig = <[u8; 64]>::from(bundle.authorization().binding_signature());
            (actions, value_balance, anchor, proof, binding_sig)
        }

        #[test]
        fn test_invalid_proof_returns_shielded_proof_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let anchor = [42u8; 32];

            // Insert the anchor so anchor validation passes
            insert_anchor_into_state(&platform, &anchor);

            // This transition is structurally valid and has a valid anchor,
            // but has random ZK proof data. It should pass structure validation
            // and anchor validation but fail at proof verification.
            let transition = create_default_shielded_transfer_transition();

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidShieldedProofError(_))
                )]
            );
        }

        /// Minimum fee for 2 actions (Orchard builder always produces ≥2).
        const MINIMUM_FEE_2_ACTIONS: u64 = 123_097_600;

        #[test]
        fn test_valid_shielded_transfer_proof_succeeds() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();
            let mut rng = OsRng;
            let pk = get_proving_key();

            let spend_amount = 200_000_000u64;
            let output_amount = spend_amount - MINIMUM_FEE_2_ACTIONS;

            // --- Create keys ---
            let sk = SpendingKey::from_bytes([0u8; 32]).unwrap();
            let fvk = FullViewingKey::from(&sk);
            let recipient = fvk.address_at(0u32, Scope::External);
            let ask = SpendAuthorizingKey::from(&sk);

            // --- Create a spendable note ---
            let rho_bytes: [u8; 32] = {
                let mut b = [0u8; 32];
                b[0] = 1; // Non-zero valid Pallas field element
                b
            };
            let rho = Rho::from_bytes(&rho_bytes).unwrap();
            let rseed = RandomSeed::from_bytes([42u8; 32], &rho).unwrap();
            let note =
                Note::from_parts(recipient, NoteValue::from_raw(spend_amount), rho, rseed).unwrap();

            // --- Build commitment tree and get anchor + merkle path ---
            let cmx = ExtractedNoteCommitment::from(note.commitment());
            let mut tree = ClientMemoryCommitmentTree::new(100);
            tree.append(cmx.to_bytes(), Retention::Marked).unwrap();
            tree.checkpoint(0u32).unwrap();
            let anchor = tree.anchor().unwrap();
            let merkle_path = tree.witness(Position::from(0u64), 0).unwrap().unwrap();

            // --- Build bundle: spend 200M → output (200M - fee), value_balance = fee ---
            let mut builder = Builder::<DashMemo>::new(BundleType::DEFAULT, anchor);
            builder.add_spend(fvk.clone(), note, merkle_path).unwrap();
            builder
                .add_output(
                    None,
                    recipient,
                    NoteValue::from_raw(output_amount),
                    [0u8; 36],
                )
                .unwrap();

            let (unauthorized, _) = builder.build::<i64>(&mut rng).unwrap().unwrap();
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &[]);
            let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
            let bundle = proven.apply_signatures(rng, sighash, &[ask]).unwrap();

            // --- Extract serialized fields ---
            let (actions, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                serialize_authorized_bundle(&bundle);

            assert_eq!(value_balance, MINIMUM_FEE_2_ACTIONS);

            // --- Set pool balance and insert anchor ---
            set_pool_total_balance(&platform, 500_000_000);
            insert_anchor_into_state(&platform, &anchor_bytes);

            // --- Create and process transition ---
            let transition = create_shielded_transfer_transition(
                actions,
                value_balance,
                anchor_bytes,
                proof_bytes,
                binding_sig,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
        }

        #[test]
        fn test_wrong_encrypted_note_size_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let anchor = [42u8; 32];

            // Insert the anchor so anchor validation passes
            insert_anchor_into_state(&platform, &anchor);

            // Create action with wrong encrypted_note size
            let mut bad_action = create_dummy_serialized_action();
            bad_action.encrypted_note = vec![0u8; 100]; // 100 bytes instead of 216

            let transition = create_shielded_transfer_transition(
                vec![bad_action],
                111_548_800, // minimum fee for 1 action (fee check runs before proof reconstruction)
                anchor,
                vec![0u8; 100],
                [0u8; 64],
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidShieldedProofError(_))
                )]
            );
        }
    }

    // ==========================================
    // FEE VALIDATION TESTS (InsufficientShieldedFeeError)
    // ==========================================
    //
    // The minimum shielded fee is:
    //   min_fee = proof_verification_fee + num_actions × (processing_fee + storage_fee)
    //
    // With current constants:
    //   proof_verification_fee     = 100_000_000
    //   per_action_processing_fee  =   3_000_000
    //   per_action_storage_fee     = 312 × (27_000 + 400) = 8_548_800
    //   per_action_total           = 11_548_800
    //
    // Minimum fees by action count:
    //   2 actions: 100_000_000 + 2 × 11_548_800 = 123_097_600
    //   3 actions: 100_000_000 + 3 × 11_548_800 = 134_646_400
    //   4 actions: 100_000_000 + 4 × 11_548_800 = 146_195_200

    mod fee_validation {
        use super::*;
        use grovedb_commitment_tree::{
            Anchor, Authorized as OrchardAuthorized, Builder, Bundle, BundleType,
            ClientMemoryCommitmentTree, DashMemo, ExtractedNoteCommitment, FullViewingKey,
            MerklePath, Note, NoteValue, Position, ProvingKey, RandomSeed, Retention, Rho, Scope,
            SpendAuthorizingKey, SpendingKey,
        };
        use rand::rngs::OsRng;
        use std::sync::OnceLock;

        const MINIMUM_FEE_2_ACTIONS: u64 = 123_097_600;
        const MINIMUM_FEE_3_ACTIONS: u64 = 134_646_400;
        const MINIMUM_FEE_4_ACTIONS: u64 = 146_195_200;

        static TEST_PROVING_KEY: OnceLock<ProvingKey> = OnceLock::new();
        fn get_proving_key() -> &'static ProvingKey {
            TEST_PROVING_KEY.get_or_init(ProvingKey::build)
        }

        fn serialize_authorized_bundle(
            bundle: &Bundle<OrchardAuthorized, i64, DashMemo>,
        ) -> (Vec<SerializedAction>, u64, [u8; 32], Vec<u8>, [u8; 64]) {
            let actions: Vec<SerializedAction> = bundle
                .actions()
                .iter()
                .map(|action| {
                    let enc = action.encrypted_note();
                    let mut encrypted_note = Vec::with_capacity(216);
                    encrypted_note.extend_from_slice(&enc.epk_bytes);
                    encrypted_note.extend_from_slice(enc.enc_ciphertext.as_ref());
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
            let value_balance = *bundle.value_balance() as u64;
            let anchor = bundle.anchor().to_bytes();
            let proof = bundle.authorization().proof().as_ref().to_vec();
            let binding_sig = <[u8; 64]>::from(bundle.authorization().binding_signature());
            (actions, value_balance, anchor, proof, binding_sig)
        }

        /// Helper to create a dummy action with a unique seed (avoids duplicate nullifiers).
        fn create_dummy_action(seed: u8) -> SerializedAction {
            SerializedAction {
                nullifier: [seed; 32],
                rk: [seed.wrapping_add(10); 32],
                cmx: [seed.wrapping_add(20); 32],
                encrypted_note: vec![seed.wrapping_add(30); 216],
                cv_net: [seed.wrapping_add(40); 32],
                spend_auth_sig: [seed.wrapping_add(50); 64],
            }
        }

        // --- Insufficient fee tests (dummy bundles — fee check runs before proof verification) ---

        #[test]
        fn test_zero_fee_returns_invalid_value_balance_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            // 2 actions with zero fee — rejected at structure validation since
            // value_balance == 0 is invalid (it IS the fee for shielded transfers)
            let transition = create_shielded_transfer_transition(
                vec![create_dummy_action(1), create_dummy_action(2)],
                0, // zero fee
                [42u8; 32],
                vec![0u8; 100],
                [0u8; 64],
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
        fn test_fee_one_below_minimum_for_2_actions_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            // 2 actions with fee one credit below minimum
            let transition = create_shielded_transfer_transition(
                vec![create_dummy_action(1), create_dummy_action(2)],
                MINIMUM_FEE_2_ACTIONS - 1, // 121,343,999
                [42u8; 32],
                vec![0u8; 100],
                [0u8; 64],
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InsufficientShieldedFeeError(_))
                )]
            );
        }

        #[test]
        fn test_fee_one_below_minimum_for_3_actions_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            // 3 actions with fee one credit below minimum
            let transition = create_shielded_transfer_transition(
                vec![
                    create_dummy_action(1),
                    create_dummy_action(2),
                    create_dummy_action(3),
                ],
                MINIMUM_FEE_3_ACTIONS - 1, // 134,646,399
                [42u8; 32],
                vec![0u8; 100],
                [0u8; 64],
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InsufficientShieldedFeeError(_))
                )]
            );
        }

        #[test]
        fn test_fee_one_below_minimum_for_4_actions_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            // 4 actions with fee one credit below minimum
            let transition = create_shielded_transfer_transition(
                vec![
                    create_dummy_action(1),
                    create_dummy_action(2),
                    create_dummy_action(3),
                    create_dummy_action(4),
                ],
                MINIMUM_FEE_4_ACTIONS - 1, // 146,195,199
                [42u8; 32],
                vec![0u8; 100],
                [0u8; 64],
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InsufficientShieldedFeeError(_))
                )]
            );
        }

        // --- Exact minimum fee tests (real bundles with valid ZK proofs) ---

        /// Build a valid 2-action Orchard bundle where value_balance equals the desired fee.
        /// Spends `spend_amount` and outputs `spend_amount - fee`, so value_balance = fee.
        fn build_bundle_with_fee(
            fee: u64,
        ) -> (Vec<SerializedAction>, u64, [u8; 32], Vec<u8>, [u8; 64]) {
            let mut rng = OsRng;
            let pk = get_proving_key();

            let sk = SpendingKey::from_bytes([0u8; 32]).unwrap();
            let fvk = FullViewingKey::from(&sk);
            let recipient = fvk.address_at(0u32, Scope::External);
            let ask = SpendAuthorizingKey::from(&sk);

            let spend_amount = 200_000_000u64; // 200M credits
            let output_amount = spend_amount - fee;

            let rho_bytes: [u8; 32] = {
                let mut b = [0u8; 32];
                b[0] = 1;
                b
            };
            let rho = Rho::from_bytes(&rho_bytes).unwrap();
            let rseed = RandomSeed::from_bytes([42u8; 32], &rho).unwrap();
            let note =
                Note::from_parts(recipient, NoteValue::from_raw(spend_amount), rho, rseed).unwrap();

            let cmx = ExtractedNoteCommitment::from(note.commitment());
            let mut tree = ClientMemoryCommitmentTree::new(100);
            tree.append(cmx.to_bytes(), Retention::Marked).unwrap();
            tree.checkpoint(0u32).unwrap();
            let anchor = tree.anchor().unwrap();
            let merkle_path = tree.witness(Position::from(0u64), 0).unwrap().unwrap();

            let mut builder = Builder::<DashMemo>::new(BundleType::DEFAULT, anchor);
            builder.add_spend(fvk.clone(), note, merkle_path).unwrap();
            builder
                .add_output(
                    None,
                    recipient,
                    NoteValue::from_raw(output_amount),
                    [0u8; 36],
                )
                .unwrap();

            let (unauthorized, _) = builder.build::<i64>(&mut rng).unwrap().unwrap();
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &[]);
            let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
            let bundle = proven.apply_signatures(rng, sighash, &[ask]).unwrap();

            serialize_authorized_bundle(&bundle)
        }

        #[test]
        fn test_exact_minimum_fee_for_2_actions_succeeds() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let (actions, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                build_bundle_with_fee(MINIMUM_FEE_2_ACTIONS);

            // Verify the bundle has exactly 2 actions and the expected fee
            assert_eq!(actions.len(), 2);
            assert_eq!(value_balance, MINIMUM_FEE_2_ACTIONS);

            // Set pool balance large enough to cover the fee deduction
            set_pool_total_balance(&platform, 500_000_000);
            insert_anchor_into_state(&platform, &anchor_bytes);

            let transition = create_shielded_transfer_transition(
                actions,
                value_balance,
                anchor_bytes,
                proof_bytes,
                binding_sig,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
        }

        #[test]
        fn test_fee_above_minimum_for_2_actions_succeeds() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            // Pay 1 credit more than the minimum
            let (actions, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                build_bundle_with_fee(MINIMUM_FEE_2_ACTIONS + 1);

            assert_eq!(actions.len(), 2);
            assert_eq!(value_balance, MINIMUM_FEE_2_ACTIONS + 1);

            set_pool_total_balance(&platform, 500_000_000);
            insert_anchor_into_state(&platform, &anchor_bytes);

            let transition = create_shielded_transfer_transition(
                actions,
                value_balance,
                anchor_bytes,
                proof_bytes,
                binding_sig,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
        }
    }

    // ==========================================
    // SECURITY AUDIT TESTS
    // ==========================================
    //
    // These tests verify vulnerabilities and edge cases discovered
    // during a security audit of the shielded transaction system.
    // Tests that demonstrate actual vulnerabilities are marked with
    // "AUDIT FINDING" comments and document the expected correct behavior.

    mod security_audit {
        use super::*;
        use grovedb_commitment_tree::{
            Anchor, Authorized as OrchardAuthorized, Builder, Bundle, BundleType,
            ClientMemoryCommitmentTree, DashMemo, ExtractedNoteCommitment, FullViewingKey,
            MerklePath, Note, NoteValue, Position, ProvingKey, RandomSeed, Retention, Rho, Scope,
            SpendAuthorizingKey, SpendingKey,
        };
        use rand::rngs::OsRng;
        use std::sync::OnceLock;

        /// Minimum fee for 2 actions (Orchard builder always produces ≥2).
        const MINIMUM_FEE_2_ACTIONS: u64 = 123_097_600;

        static TEST_PROVING_KEY: OnceLock<ProvingKey> = OnceLock::new();
        fn get_proving_key() -> &'static ProvingKey {
            TEST_PROVING_KEY.get_or_init(ProvingKey::build)
        }

        fn serialize_authorized_bundle(
            bundle: &Bundle<OrchardAuthorized, i64, DashMemo>,
        ) -> (Vec<SerializedAction>, u64, [u8; 32], Vec<u8>, [u8; 64]) {
            let actions: Vec<SerializedAction> = bundle
                .actions()
                .iter()
                .map(|action| {
                    let enc = action.encrypted_note();
                    let mut encrypted_note = Vec::with_capacity(216);
                    encrypted_note.extend_from_slice(&enc.epk_bytes);
                    encrypted_note.extend_from_slice(enc.enc_ciphertext.as_ref());
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
            let value_balance = *bundle.value_balance() as u64;
            let anchor = bundle.anchor().to_bytes();
            let proof = bundle.authorization().proof().as_ref().to_vec();
            let binding_sig = <[u8; 64]>::from(bundle.authorization().binding_signature());
            (actions, value_balance, anchor, proof, binding_sig)
        }

        /// Build a valid Orchard bundle for shielded transfer tests.
        /// Includes sufficient fee (value_balance = MINIMUM_FEE_2_ACTIONS).
        /// Returns (actions, value_balance, anchor_bytes, proof_bytes, binding_sig).
        fn build_valid_shielded_transfer_bundle(
        ) -> (Vec<SerializedAction>, u64, [u8; 32], Vec<u8>, [u8; 64]) {
            let mut rng = OsRng;
            let pk = get_proving_key();

            let spend_amount = 200_000_000u64;
            let output_amount = spend_amount - MINIMUM_FEE_2_ACTIONS;

            let sk = SpendingKey::from_bytes([0u8; 32]).unwrap();
            let fvk = FullViewingKey::from(&sk);
            let recipient = fvk.address_at(0u32, Scope::External);
            let ask = SpendAuthorizingKey::from(&sk);

            let rho_bytes: [u8; 32] = {
                let mut b = [0u8; 32];
                b[0] = 1;
                b
            };
            let rho = Rho::from_bytes(&rho_bytes).unwrap();
            let rseed = RandomSeed::from_bytes([42u8; 32], &rho).unwrap();
            let note =
                Note::from_parts(recipient, NoteValue::from_raw(spend_amount), rho, rseed).unwrap();

            let cmx = ExtractedNoteCommitment::from(note.commitment());
            let mut tree = ClientMemoryCommitmentTree::new(100);
            tree.append(cmx.to_bytes(), Retention::Marked).unwrap();
            tree.checkpoint(0u32).unwrap();
            let anchor = tree.anchor().unwrap();
            let merkle_path = tree.witness(Position::from(0u64), 0).unwrap().unwrap();

            let mut builder = Builder::<DashMemo>::new(BundleType::DEFAULT, anchor);
            builder.add_spend(fvk.clone(), note, merkle_path).unwrap();
            builder
                .add_output(
                    None,
                    recipient,
                    NoteValue::from_raw(output_amount),
                    [0u8; 36],
                )
                .unwrap();

            let (unauthorized, _) = builder.build::<i64>(&mut rng).unwrap().unwrap();
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &[]);
            let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
            let bundle = proven.apply_signatures(rng, sighash, &[ask]).unwrap();

            serialize_authorized_bundle(&bundle)
        }

        /// AUDIT REGRESSION: Mutating value_balance is now caught by BatchValidator.
        ///
        /// Previously, the code only called `bundle.verify_proof(vk)` which did not
        /// check the binding signature. Now `BatchValidator` verifies the Halo 2 proof
        /// AND the binding signature, which cryptographically binds value_balance to
        /// the value commitments (cv_net). Mutating value_balance changes the bundle
        /// commitment (sighash), causing signature verification to fail.
        ///
        /// Original severity: CRITICAL — now FIXED.
        #[test]
        fn test_valid_proof_with_mutated_value_balance_is_rejected() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let (actions, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                build_valid_shielded_transfer_bundle();
            assert_eq!(value_balance, MINIMUM_FEE_2_ACTIONS);

            // ATTACK: Mutate value_balance (increase by 5000 so it still passes fee check)
            let mutated_value_balance = value_balance + 5000;

            insert_anchor_into_state(&platform, &anchor_bytes);

            let transition = create_shielded_transfer_transition(
                actions,
                mutated_value_balance, // MUTATED: different from signed value
                anchor_bytes,
                proof_bytes,
                binding_sig,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            // FIXED: BatchValidator detects the binding signature mismatch
            // because mutating value_balance changes the bundle commitment (sighash).
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidShieldedProofError(_))
                )]
            );
        }

        /// AUDIT REGRESSION: Zeroed binding signature is now caught by BatchValidator.
        ///
        /// Previously accepted because only the Halo 2 proof was verified.
        /// Now `BatchValidator` verifies the binding signature as well.
        ///
        /// Original severity: CRITICAL — now FIXED.
        #[test]
        fn test_valid_proof_with_zeroed_binding_sig_is_rejected() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let (actions, value_balance, anchor_bytes, proof_bytes, _binding_sig) =
                build_valid_shielded_transfer_bundle();

            insert_anchor_into_state(&platform, &anchor_bytes);

            let transition = create_shielded_transfer_transition(
                actions,
                value_balance,
                anchor_bytes,
                proof_bytes,
                [0u8; 64], // ZEROED binding signature
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            // FIXED: BatchValidator detects the invalid binding signature.
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidShieldedProofError(_))
                )]
            );
        }

        /// AUDIT REGRESSION: Zeroed spend auth signatures are now caught by BatchValidator.
        ///
        /// Previously accepted because only the Halo 2 proof was verified.
        /// Now `BatchValidator` verifies spend authorization signatures, proving
        /// that the spender controls the spending key.
        ///
        /// Original severity: CRITICAL — now FIXED.
        #[test]
        fn test_valid_proof_with_zeroed_spend_auth_sig_is_rejected() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let (mut actions, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                build_valid_shielded_transfer_bundle();

            // ATTACK: Zero out all spend auth signatures
            for action in &mut actions {
                action.spend_auth_sig = [0u8; 64];
            }

            insert_anchor_into_state(&platform, &anchor_bytes);

            let transition = create_shielded_transfer_transition(
                actions,
                value_balance,
                anchor_bytes,
                proof_bytes,
                binding_sig,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            // FIXED: BatchValidator detects the invalid spend auth signatures.
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidShieldedProofError(_))
                )]
            );
        }

        /// Duplicate nullifiers within the same bundle — proof verification now
        /// runs before the intra-bundle dedup check, so the invalid proof is
        /// rejected first.
        #[test]
        fn test_duplicate_nullifiers_in_same_bundle() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let anchor = [42u8; 32];
            insert_anchor_into_state(&platform, &anchor);

            // Two actions with the same nullifier but different cmx
            let action1 = create_dummy_serialized_action();
            let mut action2 = create_dummy_serialized_action();
            action2.cmx = [99u8; 32]; // Different commitment

            let transition = create_shielded_transfer_transition(
                vec![action1, action2], // Both have nullifier [1u8; 32]
                MINIMUM_FEE_2_ACTIONS,  // sufficient fee so we reach proof verification
                anchor,
                vec![0u8; 100],
                [0u8; 64],
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            // Proof verification now runs before nullifier dedup, so the
            // dummy proof data is rejected first.
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidShieldedProofError(_))
                )]
            );
        }
    }

    // ==========================================
    // STATE VALIDATION TESTS (post-ZK-proof)
    // ==========================================
    //
    // These tests use valid Orchard ZK proofs but set up wrong state
    // conditions (insufficient pool balance, missing anchor, spent nullifier)
    // to exercise error branches that are only reachable after proof
    // verification passes in the processing pipeline.

    mod state_validation_with_valid_proof {
        use super::*;
        use grovedb_commitment_tree::{
            Anchor, Authorized as OrchardAuthorized, Builder, Bundle, BundleType,
            ClientMemoryCommitmentTree, DashMemo, ExtractedNoteCommitment, FullViewingKey,
            MerklePath, Note, NoteValue, Position, ProvingKey, RandomSeed, Retention, Rho, Scope,
            SpendAuthorizingKey, SpendingKey,
        };
        use rand::rngs::OsRng;
        use std::sync::OnceLock;

        const MINIMUM_FEE_2_ACTIONS: u64 = 123_097_600;

        static TEST_PROVING_KEY: OnceLock<ProvingKey> = OnceLock::new();
        fn get_proving_key() -> &'static ProvingKey {
            TEST_PROVING_KEY.get_or_init(ProvingKey::build)
        }

        fn serialize_authorized_bundle(
            bundle: &Bundle<OrchardAuthorized, i64, DashMemo>,
        ) -> (Vec<SerializedAction>, u64, [u8; 32], Vec<u8>, [u8; 64]) {
            let actions: Vec<SerializedAction> = bundle
                .actions()
                .iter()
                .map(|action| {
                    let enc = action.encrypted_note();
                    let mut encrypted_note = Vec::with_capacity(216);
                    encrypted_note.extend_from_slice(&enc.epk_bytes);
                    encrypted_note.extend_from_slice(enc.enc_ciphertext.as_ref());
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
            let value_balance = *bundle.value_balance() as u64;
            let anchor = bundle.anchor().to_bytes();
            let proof = bundle.authorization().proof().as_ref().to_vec();
            let binding_sig = <[u8; 64]>::from(bundle.authorization().binding_signature());
            (actions, value_balance, anchor, proof, binding_sig)
        }

        fn build_valid_bundle() -> (Vec<SerializedAction>, u64, [u8; 32], Vec<u8>, [u8; 64]) {
            let mut rng = OsRng;
            let pk = get_proving_key();
            let spend_amount = 200_000_000u64;
            let output_amount = spend_amount - MINIMUM_FEE_2_ACTIONS;

            let sk = SpendingKey::from_bytes([0u8; 32]).unwrap();
            let fvk = FullViewingKey::from(&sk);
            let recipient = fvk.address_at(0u32, Scope::External);
            let ask = SpendAuthorizingKey::from(&sk);

            let rho_bytes: [u8; 32] = {
                let mut b = [0u8; 32];
                b[0] = 1;
                b
            };
            let rho = Rho::from_bytes(&rho_bytes).unwrap();
            let rseed = RandomSeed::from_bytes([42u8; 32], &rho).unwrap();
            let note =
                Note::from_parts(recipient, NoteValue::from_raw(spend_amount), rho, rseed).unwrap();

            let cmx = ExtractedNoteCommitment::from(note.commitment());
            let mut tree = ClientMemoryCommitmentTree::new(100);
            tree.append(cmx.to_bytes(), Retention::Marked).unwrap();
            tree.checkpoint(0u32).unwrap();
            let anchor = tree.anchor().unwrap();
            let merkle_path = tree.witness(Position::from(0u64), 0).unwrap().unwrap();

            let mut builder = Builder::<DashMemo>::new(BundleType::DEFAULT, anchor);
            builder.add_spend(fvk.clone(), note, merkle_path).unwrap();
            builder
                .add_output(
                    None,
                    recipient,
                    NoteValue::from_raw(output_amount),
                    [0u8; 36],
                )
                .unwrap();

            let (unauthorized, _) = builder.build::<i64>(&mut rng).unwrap().unwrap();
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &[]);
            let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
            let bundle = proven.apply_signatures(rng, sighash, &[ask]).unwrap();

            serialize_authorized_bundle(&bundle)
        }

        /// Test that insufficient pool balance is caught AFTER ZK proof passes.
        /// This exercises the `current_total_balance < fee_amount` branch in
        /// transform_into_action_v0.
        #[test]
        fn test_insufficient_pool_balance_with_valid_proof() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let (actions, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                build_valid_bundle();

            // Insert the anchor but set pool balance to 0 (insufficient)
            insert_anchor_into_state(&platform, &anchor_bytes);
            // Do NOT set pool balance -- default is 0, which is less than the fee

            let transition = create_shielded_transfer_transition(
                actions,
                value_balance,
                anchor_bytes,
                proof_bytes,
                binding_sig,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidShieldedProofError(_))
                )]
            );
        }

        /// Test that a missing anchor is caught after ZK proof passes.
        /// This exercises the `validate_anchor_exists` error path in transform_into_action_v0.
        #[test]
        fn test_missing_anchor_with_valid_proof() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let (actions, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                build_valid_bundle();

            // Set sufficient pool balance but do NOT insert the anchor
            set_pool_total_balance(&platform, 500_000_000);

            let transition = create_shielded_transfer_transition(
                actions,
                value_balance,
                anchor_bytes,
                proof_bytes,
                binding_sig,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidAnchorError(_))
                )]
            );
        }

        /// Test that a spent nullifier is caught after ZK proof passes.
        /// This exercises the `validate_nullifiers` phase 2 error path in transform_into_action_v0.
        #[test]
        fn test_spent_nullifier_with_valid_proof() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let (actions, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                build_valid_bundle();

            // Set up valid state conditions
            set_pool_total_balance(&platform, 500_000_000);
            insert_anchor_into_state(&platform, &anchor_bytes);

            // Mark a nullifier from the bundle as already spent
            let nullifier = actions[0].nullifier;
            insert_nullifier_into_state(&platform, &nullifier);

            let transition = create_shielded_transfer_transition(
                actions,
                value_balance,
                anchor_bytes,
                proof_bytes,
                binding_sig,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::NullifierAlreadySpentError(_))
                )]
            );
        }
    }

    // ==========================================
    // PROOF GENERATION & VERIFICATION TESTS
    // ==========================================

    mod return_proof {
        use super::*;
        use dpp::block::block_info::BlockInfo;
        use dpp::serialization::PlatformSerializable;
        use dpp::state_transition::proof_result::StateTransitionProofResult;
        use dpp::state_transition::shielded_transfer_transition::accessors::ShieldedTransferTransitionAccessorsV0;
        use drive::drive::Drive;
        use grovedb_commitment_tree::{
            Anchor, Authorized as OrchardAuthorized, Builder, Bundle, BundleType,
            ClientMemoryCommitmentTree, DashMemo, ExtractedNoteCommitment, FullViewingKey, Note,
            NoteValue, Position, ProvingKey, RandomSeed, Retention, Rho, Scope,
            SpendAuthorizingKey, SpendingKey,
        };
        use rand::rngs::OsRng;
        use std::sync::OnceLock;

        const MINIMUM_FEE_2_ACTIONS: u64 = 123_097_600;

        static TEST_PROVING_KEY: OnceLock<ProvingKey> = OnceLock::new();
        fn get_proving_key() -> &'static ProvingKey {
            TEST_PROVING_KEY.get_or_init(ProvingKey::build)
        }

        fn serialize_authorized_bundle(
            bundle: &Bundle<OrchardAuthorized, i64, DashMemo>,
        ) -> (Vec<SerializedAction>, u64, [u8; 32], Vec<u8>, [u8; 64]) {
            let actions: Vec<SerializedAction> = bundle
                .actions()
                .iter()
                .map(|action| {
                    let enc = action.encrypted_note();
                    let mut encrypted_note = Vec::with_capacity(216);
                    encrypted_note.extend_from_slice(&enc.epk_bytes);
                    encrypted_note.extend_from_slice(enc.enc_ciphertext.as_ref());
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
            let value_balance = *bundle.value_balance() as u64;
            let anchor = bundle.anchor().to_bytes();
            let proof = bundle.authorization().proof().as_ref().to_vec();
            let binding_sig = <[u8; 64]>::from(bundle.authorization().binding_signature());
            (actions, value_balance, anchor, proof, binding_sig)
        }

        #[test]
        fn test_shielded_transfer_prove_and_verify_nullifiers() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();
            let mut rng = OsRng;
            let pk = get_proving_key();

            let spend_amount = 200_000_000u64;
            let output_amount = spend_amount - MINIMUM_FEE_2_ACTIONS;

            // --- Create keys ---
            let sk = SpendingKey::from_bytes([0u8; 32]).unwrap();
            let fvk = FullViewingKey::from(&sk);
            let recipient = fvk.address_at(0u32, Scope::External);
            let ask = SpendAuthorizingKey::from(&sk);

            // --- Create a spendable note ---
            let rho_bytes: [u8; 32] = {
                let mut b = [0u8; 32];
                b[0] = 1;
                b
            };
            let rho = Rho::from_bytes(&rho_bytes).unwrap();
            let rseed = RandomSeed::from_bytes([42u8; 32], &rho).unwrap();
            let note =
                Note::from_parts(recipient, NoteValue::from_raw(spend_amount), rho, rseed).unwrap();

            // --- Build commitment tree and get anchor + merkle path ---
            let cmx = ExtractedNoteCommitment::from(note.commitment());
            let mut tree = ClientMemoryCommitmentTree::new(100);
            tree.append(cmx.to_bytes(), Retention::Marked).unwrap();
            tree.checkpoint(0u32).unwrap();
            let anchor = tree.anchor().unwrap();
            let merkle_path = tree.witness(Position::from(0u64), 0).unwrap().unwrap();

            // --- Build bundle ---
            let mut builder = Builder::<DashMemo>::new(BundleType::DEFAULT, anchor);
            builder.add_spend(fvk.clone(), note, merkle_path).unwrap();
            builder
                .add_output(
                    None,
                    recipient,
                    NoteValue::from_raw(output_amount),
                    [0u8; 36],
                )
                .unwrap();

            let (unauthorized, _) = builder.build::<i64>(&mut rng).unwrap().unwrap();
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &[]);
            let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
            let bundle = proven.apply_signatures(rng, sighash, &[ask]).unwrap();

            let (actions, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                serialize_authorized_bundle(&bundle);

            // --- Set up pool state ---
            set_pool_total_balance(&platform, 500_000_000);
            insert_anchor_into_state(&platform, &anchor_bytes);

            // --- Build and serialize the transition ---
            let transition = create_shielded_transfer_transition(
                actions,
                value_balance,
                anchor_bytes,
                proof_bytes,
                binding_sig,
            );

            let transition_bytes = transition
                .serialize_to_bytes()
                .expect("should serialize transition");

            // --- Process with manual transaction so we can commit before proving ---
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
                .prove_state_transition(&transition, None, platform_version)
                .expect("expected to generate proof for shielded transfer");

            let proof_bytes = proof_result
                .into_data()
                .expect("expected proof data, not an error");

            // --- Verify proof ---
            let (root_hash, proof_result) = Drive::verify_state_transition_was_executed_with_proof(
                &transition,
                &BlockInfo::default(),
                &proof_bytes,
                &|_| Ok(None),
                platform_version,
            )
            .expect("expected to verify shielded transfer proof");

            assert_ne!(root_hash, [0u8; 32], "root hash should not be zeroed");

            // --- Assert result is VerifiedShieldedNullifiers with all spent ---
            let StateTransitionProofResult::VerifiedShieldedNullifiers(statuses) = proof_result
            else {
                panic!(
                    "expected VerifiedShieldedNullifiers, got {:?}",
                    proof_result
                );
            };

            // Extract expected nullifiers from the transition
            let StateTransition::ShieldedTransfer(ref st) = transition else {
                unreachable!();
            };
            let expected_nullifiers: Vec<Vec<u8>> = st.nullifiers();

            assert_eq!(
                statuses.len(),
                expected_nullifiers.len(),
                "should have one status per nullifier"
            );

            for (nf, is_spent) in &statuses {
                assert!(is_spent, "nullifier {} should be spent", hex::encode(nf));
                assert!(
                    expected_nullifiers.contains(nf),
                    "proved nullifier {} should be one of the transition's nullifiers",
                    hex::encode(nf)
                );
            }
        }
    }
}
