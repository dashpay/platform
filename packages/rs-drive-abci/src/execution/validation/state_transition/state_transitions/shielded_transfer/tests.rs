#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::execution::validation::state_transition::state_transitions::shielded_common::compute_platform_sighash;
    use crate::execution::validation::state_transition::state_transitions::test_helpers::{
        create_dummy_serialized_action, get_proving_key, insert_anchor_into_state,
        insert_nullifier_into_state, process_transition, serialize_authorized_bundle_u64,
        set_pool_total_balance, setup_platform,
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

    /// Minimum shielded fee for `num_actions`, sourced from the canonical
    /// `dpp::shielded::compute_minimum_shielded_fee` (against `PlatformVersion::latest()`, which is
    /// what every test in this module uses) so the test fixtures track the per-action fee constants
    /// and can never go stale relative to consensus.
    fn minimum_fee(num_actions: usize) -> u64 {
        dpp::shielded::compute_minimum_shielded_fee(num_actions, PlatformVersion::latest())
            .expect("minimum shielded fee computation")
    }

    /// Shorthand for creating a structurally valid (but cryptographically invalid) shielded
    /// transfer transition. Has a non-zero anchor, valid field sizes, but random data.
    /// Includes sufficient fee to pass the minimum shielded fee check (the 1-action minimum).
    fn create_default_shielded_transfer_transition() -> StateTransition {
        create_shielded_transfer_transition(
            vec![create_dummy_serialized_action()],
            minimum_fee(1),
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

            let transition = ShieldedTransferTransitionV0 {
                actions,
                value_balance: minimum_fee(1),
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
            Anchor, Builder, BundleType, ClientMemoryCommitmentTree, DashMemo,
            ExtractedNoteCommitment, FullViewingKey, MerklePath, Note, NoteValue, Position,
            RandomSeed, Retention, Rho, Scope, SpendAuthorizingKey, SpendingKey,
        };
        use rand::rngs::StdRng;
        use rand::SeedableRng;

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

        #[test]
        fn test_valid_shielded_transfer_proof_succeeds() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();
            let mut rng = StdRng::seed_from_u64(0);
            let pk = get_proving_key();

            // Minimum fee for 2 actions (Orchard builder always produces ≥2).
            let minimum_fee_2_actions = minimum_fee(2);
            let spend_amount = 200_000_000u64;
            let output_amount = spend_amount - minimum_fee_2_actions;

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
                serialize_authorized_bundle_u64(&bundle);

            assert_eq!(value_balance, minimum_fee_2_actions);

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

            // CheckTx root-invariance guard (devnet paloma h788): `check_tx` asserts under
            // cfg(test) that it never mutates committed grovedb state, so running the
            // canonical valid fixture through it pins the invariant for this transition type.
            {
                use dpp::serialization::PlatformSerializable;
                let guard_serialized_transition = transition
                    .serialize_to_bytes()
                    .expect("expected to serialize transition for the check_tx guard");
                crate::test::helpers::state_mutation_guard::assert_check_tx_valid_at_all_levels(
                    &platform,
                    &guard_serialized_transition,
                    "shielded transfer",
                );
            }

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
                minimum_fee(1), // minimum fee for 1 action (fee check runs before proof reconstruction)
                anchor,
                vec![0u8; 100],
                [0u8; 64],
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            // DPP structure validation now catches this before proof verification
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
    // FEE VALIDATION TESTS (InsufficientShieldedFeeError)
    // ==========================================
    //
    // The minimum shielded fee is:
    //   min_fee = proof_verification_fee + num_actions × (processing_fee + storage_fee)
    //
    // The exact per-action / per-bundle constants live in `dpp` and evolve across protocol versions
    // (e.g. the storage allowance changed when `cv_net` was added). Rather than
    // hardcode the resulting numbers (which silently go stale when a constant changes), these tests
    // source the threshold from the canonical `dpp::shielded::compute_minimum_shielded_fee` via the
    // module-level `minimum_fee(num_actions)` helper, so the fixture fee always matches the consensus
    // gate.

    mod fee_validation {
        use super::*;
        use grovedb_commitment_tree::{
            Anchor, Builder, BundleType, ClientMemoryCommitmentTree, DashMemo,
            ExtractedNoteCommitment, FullViewingKey, MerklePath, Note, NoteValue, Position,
            RandomSeed, Retention, Rho, Scope, SpendAuthorizingKey, SpendingKey,
        };
        use rand::rngs::StdRng;
        use rand::SeedableRng;

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
                minimum_fee(2) - 1, // one credit below the 2-action minimum
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
                minimum_fee(3) - 1, // one credit below the 3-action minimum
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
                minimum_fee(4) - 1, // one credit below the 4-action minimum
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
            let mut rng = StdRng::seed_from_u64(0);
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

            serialize_authorized_bundle_u64(&bundle)
        }

        #[test]
        fn test_exact_minimum_fee_for_2_actions_succeeds() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let (actions, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                build_bundle_with_fee(minimum_fee(2));

            // Verify the bundle has exactly 2 actions and the expected fee
            assert_eq!(actions.len(), 2);
            assert_eq!(value_balance, minimum_fee(2));

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
        fn test_fee_above_minimum_for_2_actions_is_rejected() {
            // A shielded transfer's `value_balance` IS the fee (no recipient amount), so it
            // must equal the minimum exactly. Paying even 1 credit above the minimum is
            // rejected — overpayment buys nothing and would leak a fee fingerprint.
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            // Pay 1 credit more than the minimum
            let (actions, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                build_bundle_with_fee(minimum_fee(2) + 1);

            assert_eq!(actions.len(), 2);
            assert_eq!(value_balance, minimum_fee(2) + 1);

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
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::ShieldedInvalidValueBalanceError(_))
                )]
            );
        }

        /// Builds a REAL spend+output Orchard bundle whose output policy mirrors the dpp SDK
        /// builder `build_shielded_transfer_transition`: `num_spends` spends, then 1 recipient
        /// output and (when there is change left over) 1 change output — i.e. at most 2 outputs.
        /// Returns the serialized on-wire actions plus the fee that bundle's `value_balance`
        /// encodes.
        ///
        /// The total spent is sized so `value_balance == compute_minimum_shielded_fee(num_spends
        /// .max(2))` exactly, and a positive change output is emitted for `num_spends >= 2` (so the
        /// `> 2 spends` case exercises the change-output branch too) — the same shape the SDK
        /// builder produces.
        fn build_transfer_like_bundle(num_spends: usize) -> (Vec<SerializedAction>, u64) {
            assert!(num_spends >= 1, "need at least one spend");
            let platform_version = PlatformVersion::latest();

            // The SDK builder carves the fee from spends.len().max(2), BEFORE Orchard padding.
            let carved_actions = num_spends.max(2);
            let fee = dpp::shielded::compute_minimum_shielded_fee(carved_actions, platform_version)
                .expect("fee computation");

            let mut rng = StdRng::seed_from_u64(0);
            let pk = get_proving_key();

            let sk = SpendingKey::from_bytes([0u8; 32]).unwrap();
            let fvk = FullViewingKey::from(&sk);
            let recipient = fvk.address_at(0u32, Scope::External);
            let ask = SpendAuthorizingKey::from(&sk);

            // Each note holds a generous, equal value so the bundle is value-balanced.
            let value_each = 200_000_000u64;
            let total_spent = value_each * num_spends as u64;

            // Build every note, append every commitment into ONE tree (single shared anchor —
            // the builder uses exactly one anchor), then witness each note at its position.
            let notes: Vec<Note> = (0..num_spends)
                .map(|i| {
                    let mut rho_bytes = [0u8; 32];
                    rho_bytes[0] = (i as u8).wrapping_add(1); // distinct, non-zero, valid pallas
                    let rho = Rho::from_bytes(&rho_bytes).unwrap();
                    let rseed = RandomSeed::from_bytes([42u8; 32], &rho).unwrap();
                    Note::from_parts(recipient, NoteValue::from_raw(value_each), rho, rseed)
                        .unwrap()
                })
                .collect();

            let mut tree = ClientMemoryCommitmentTree::new(100);
            for note in &notes {
                let cmx = ExtractedNoteCommitment::from(note.commitment());
                tree.append(cmx.to_bytes(), Retention::Marked).unwrap();
            }
            tree.checkpoint(0u32).unwrap();
            let anchor = tree.anchor().unwrap();

            let mut builder = Builder::<DashMemo>::new(BundleType::DEFAULT, anchor);
            for (i, note) in notes.into_iter().enumerate() {
                let merkle_path: MerklePath =
                    tree.witness(Position::from(i as u64), 0).unwrap().unwrap();
                builder.add_spend(fvk.clone(), note, merkle_path).unwrap();
            }

            // Mirror the SDK builder's output policy: 1 recipient output + optional 1 change.
            // recipient_amount + fee + change == total_spent. Send a small recipient amount and
            // route the rest to change (positive whenever num_spends >= 2 with these values).
            let recipient_amount = 1_000u64;
            let change_amount = total_spent - recipient_amount - fee;
            builder
                .add_output(
                    None,
                    recipient,
                    NoteValue::from_raw(recipient_amount),
                    [0u8; 36],
                )
                .unwrap();
            if change_amount > 0 {
                builder
                    .add_output(
                        None,
                        recipient,
                        NoteValue::from_raw(change_amount),
                        [0u8; 36],
                    )
                    .unwrap();
            }

            let (unauthorized, _) = builder.build::<i64>(&mut rng).unwrap().unwrap();
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &[]);
            let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
            let bundle = proven.apply_signatures(rng, sighash, &[ask]).unwrap();

            let (actions, value_balance, _anchor, _proof, _binding) =
                serialize_authorized_bundle_u64(&bundle);
            (actions, value_balance)
        }

        /// Pins the dpp SDK builder's fee-vs-actions coupling end-to-end.
        ///
        /// The SDK builder `build_shielded_transfer_transition` carves the fee from
        /// `num_actions = spends.len().max(2)` BEFORE Orchard padding, then emits 1 recipient
        /// output + (optionally) 1 change output. Consensus prices the shielded fee from the
        /// on-wire `actions.len()` and pins `value_balance == compute_minimum_shielded_fee(actions
        /// .len())` EXACTLY. The carved-fee count and the on-wire count agree ONLY because the
        /// builder emits at most 2 outputs, so `max(spends, outputs, 2) == max(spends, 2)`.
        ///
        /// We build REAL bundles whose output policy mirrors the SDK builder (1 recipient + optional
        /// 1 change) for spend counts {1, 3} and assert the serialized `actions.len()` equals
        /// `spends.len().max(2)`, and that the bundle's `value_balance` equals
        /// `compute_minimum_shielded_fee(actions.len())`. If a future change makes the carved-fee
        /// action count diverge from the on-wire action count (e.g. a 3rd output added with <=2
        /// spends), this fails loudly — without it, every such transfer would be silently rejected by
        /// consensus. This is the spend-side analogue of the output-only Shield invariant pinned by
        /// `dpp`'s `test_output_only_bundle_serializes_to_min_actions`.
        #[test]
        fn test_builder_output_policy_actions_match_carved_fee_count() {
            let platform_version = PlatformVersion::latest();

            // 1 spend  -> on-wire actions padded to 2 (Orchard MIN_ACTIONS), fee carved for 2.
            // 3 spends -> on-wire actions = 3, fee carved for 3.
            for (num_spends, expected_actions) in [(1usize, 2usize), (3, 3)] {
                let (actions, value_balance) = build_transfer_like_bundle(num_spends);

                assert_eq!(
                    actions.len(),
                    expected_actions,
                    "on-wire actions.len() ({}) must equal spends.len().max(2) = {expected_actions} \
                     for {num_spends} spends; the SDK builder carves the fee for {expected_actions} \
                     actions and consensus pins value_balance to exactly \
                     compute_minimum_shielded_fee(actions.len())",
                    actions.len()
                );

                let expected_fee =
                    dpp::shielded::compute_minimum_shielded_fee(expected_actions, platform_version)
                        .expect("fee computation");
                assert_eq!(
                    value_balance, expected_fee,
                    "value_balance must equal compute_minimum_shielded_fee(on-wire actions.len())"
                );
            }
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
            Builder, BundleType, ClientMemoryCommitmentTree, DashMemo, ExtractedNoteCommitment,
            FullViewingKey, MerklePath, Note, NoteValue, Position, RandomSeed, Retention, Rho,
            Scope, SpendAuthorizingKey, SpendingKey,
        };
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        /// Build a valid Orchard bundle for shielded transfer tests.
        /// Includes sufficient fee (value_balance = the 2-action minimum fee).
        /// Returns (actions, value_balance, anchor_bytes, proof_bytes, binding_sig).
        fn build_valid_shielded_transfer_bundle(
        ) -> (Vec<SerializedAction>, u64, [u8; 32], Vec<u8>, [u8; 64]) {
            let mut rng = StdRng::seed_from_u64(0);
            let pk = get_proving_key();

            // Minimum fee for 2 actions (Orchard builder always produces ≥2).
            let spend_amount = 200_000_000u64;
            let output_amount = spend_amount - minimum_fee(2);

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

            serialize_authorized_bundle_u64(&bundle)
        }

        /// AUDIT REGRESSION: Mutating value_balance is rejected.
        ///
        /// The binding signature cryptographically binds value_balance to the value
        /// commitments (cv_net), so `BatchValidator` rejects any mutation. With the
        /// exact-fee rule for shielded transfers (`value_balance == min_fee`), a mutation
        /// that bumps value_balance off the minimum *also* fails the fee check — which runs
        /// before proof verification — so the mutation is now caught there first. Either way
        /// the attack is rejected; this asserts the earlier (fee-check) rejection.
        ///
        /// Original severity: CRITICAL — now FIXED.
        #[test]
        fn test_valid_proof_with_mutated_value_balance_is_rejected() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let (actions, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                build_valid_shielded_transfer_bundle();
            assert_eq!(value_balance, minimum_fee(2));

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

            // The exact-fee rule rejects this before proof verification: the mutation bumps
            // value_balance above the minimum, and a transfer must pay exactly the minimum.
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::ShieldedInvalidValueBalanceError(_))
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
                minimum_fee(2),         // sufficient fee so we reach proof verification
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
            Builder, BundleType, ClientMemoryCommitmentTree, DashMemo, ExtractedNoteCommitment,
            FullViewingKey, Note, NoteValue, Position, RandomSeed, Retention, Rho, Scope,
            SpendAuthorizingKey, SpendingKey,
        };
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        #[test]
        fn test_shielded_transfer_prove_and_verify_nullifiers() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();
            let mut rng = StdRng::seed_from_u64(0);
            let pk = get_proving_key();

            let spend_amount = 200_000_000u64;
            let output_amount = spend_amount - minimum_fee(2);

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
                serialize_authorized_bundle_u64(&bundle);

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
            .map(|(root_hash, outcome)| (root_hash, outcome.into_result()))
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
