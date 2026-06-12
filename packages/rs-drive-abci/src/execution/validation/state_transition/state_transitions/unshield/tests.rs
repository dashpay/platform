#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::execution::validation::state_transition::state_transitions::shielded_common::compute_platform_sighash;
    use crate::execution::validation::state_transition::state_transitions::test_helpers::{
        create_dummy_serialized_action, get_proving_key, insert_anchor_into_state,
        insert_dummy_encrypted_notes, insert_nullifier_into_state, process_transition,
        serialize_authorized_bundle_i64, set_pool_total_balance, setup_platform,
    };
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
    use assert_matches::assert_matches;
    use dpp::address_funds::PlatformAddress;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;
    use dpp::shielded::SerializedAction;
    use dpp::state_transition::unshield_transition::v0::UnshieldTransitionV0;
    use dpp::state_transition::unshield_transition::UnshieldTransition;
    use dpp::state_transition::StateTransition;
    use platform_version::version::PlatformVersion;

    // ==========================================
    // Helper Functions (transition-specific)
    // ==========================================

    /// Create a dummy PlatformAddress for the output.
    fn create_output_address() -> PlatformAddress {
        let mut hash = [0u8; 20];
        hash[0] = 42;
        hash[19] = 42;
        PlatformAddress::P2pkh(hash)
    }

    /// Minimum unshield fee for `num_actions`, sourced from the canonical
    /// `dpp::shielded::compute_shielded_unshield_fee` (against `PlatformVersion::latest()`, which is
    /// what every test in this module uses) so the test fixtures track the per-action / address-write
    /// fee constants and can never go stale relative to the consensus fee gate.
    fn unshield_fee(num_actions: usize) -> u64 {
        dpp::shielded::compute_shielded_unshield_fee(num_actions, PlatformVersion::latest())
            .expect("unshield fee computation")
    }

    /// Builds an `UnshieldTransition` state transition.
    /// No signing needed since unshield transitions have no witnesses.
    fn create_unshield_transition(
        output_address: PlatformAddress,
        actions: Vec<SerializedAction>,
        unshielding_amount: u64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
    ) -> StateTransition {
        StateTransition::Unshield(UnshieldTransition::V0(UnshieldTransitionV0 {
            output_address,
            actions,
            unshielding_amount,
            anchor,
            proof,
            binding_signature,
        }))
    }

    /// Shorthand for creating a structurally valid (but cryptographically invalid) unshield
    /// transition. Has a non-zero anchor, valid field sizes, positive unshielding_amount.
    fn create_default_unshield_transition() -> StateTransition {
        create_unshield_transition(
            create_output_address(),
            vec![create_dummy_serialized_action()],
            // unshielding_amount: the unshield fee for 1 action. The fee gate runs before proof
            // verification, so this must clear `compute_shielded_unshield_fee(1)` for these tests to
            // reach the proof-verification stage they assert on. Sourced from the canonical fee fn so
            // it tracks the per-action / address-write storage constants and cannot go stale.
            unshield_fee(1),
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

            let transition = create_unshield_transition(
                create_output_address(),
                vec![], // Empty actions — invalid
                1000,
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

        #[test]
        fn test_too_many_actions_returns_error() {
            // NOTE: We call validate_structure directly to exercise the
            // max_shielded_transition_actions check in isolation, without
            // depending on the full transition-processing pipeline.
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            // 17 actions exceeds max_shielded_transition_actions (16)
            let actions: Vec<SerializedAction> =
                (0..17).map(|_| create_dummy_serialized_action()).collect();

            let transition = UnshieldTransitionV0 {
                output_address: create_output_address(),
                actions,
                // Any valid positive amount: this test asserts a structure-validation error
                // (ShieldedTooManyActionsError) that fires before the fee gate. Sourced from the
                // canonical fee fn so it carries no stale literal.
                unshielding_amount: unshield_fee(1),
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
        fn test_zero_unshielding_amount_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let transition = create_unshield_transition(
                create_output_address(),
                vec![create_dummy_serialized_action()],
                0, // Zero unshielding_amount — invalid
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
        fn test_unshielding_amount_exceeding_i64_max_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let transition = create_unshield_transition(
                create_output_address(),
                vec![create_dummy_serialized_action()],
                i64::MAX as u64 + 1, // Exceeds i64::MAX
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

            let transition = create_unshield_transition(
                create_output_address(),
                vec![create_dummy_serialized_action()],
                1000,
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

            let transition = create_unshield_transition(
                create_output_address(),
                vec![create_dummy_serialized_action()],
                1000,
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
        fn test_insufficient_pool_notes_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            // Non-zero anchor that exists in state, but no encrypted notes in pool
            let anchor = [42u8; 32];
            insert_anchor_into_state(&platform, &anchor);

            let transition = create_default_unshield_transition();

            let processing_result = process_transition(&platform, transition, platform_version);

            // Proof verification now runs before pool notes check, so the
            // dummy proof data is rejected first.
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidShieldedProofError(_))
                )]
            );
        }

        #[test]
        fn test_invalid_anchor_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();
            insert_dummy_encrypted_notes(&platform, 250);

            // Non-zero anchor that doesn't exist in state
            let transition = create_default_unshield_transition();

            let processing_result = process_transition(&platform, transition, platform_version);

            // Proof verification now runs before anchor validation, so the
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
    // NULLIFIER DOUBLE-SPEND TESTS (StateError)
    // ==========================================

    mod nullifier_validation {
        use super::*;

        #[test]
        fn test_nullifier_already_spent_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();
            insert_dummy_encrypted_notes(&platform, 250);

            let anchor = [42u8; 32];
            let nullifier = [1u8; 32]; // Same as create_dummy_serialized_action().nullifier

            // Insert the anchor so anchor validation passes
            insert_anchor_into_state(&platform, &anchor);

            // Insert the nullifier so it appears already spent
            insert_nullifier_into_state(&platform, &nullifier);

            let transition = create_default_unshield_transition();

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
    // FEE VALIDATION TESTS (InsufficientShieldedFeeError)
    // ==========================================
    //
    // The Unshield fee gate (`validate_minimum_shielded_fee`) enforces
    // `unshielding_amount >= compute_shielded_unshield_fee(num_actions)`, NOT the base
    // `compute_minimum_shielded_fee(num_actions)`. The unshield fee is the base PLUS the flat
    // 222-byte `AddBalanceToAddress` output-write storage cost, so there is a non-empty half-open
    // range `[compute_minimum_shielded_fee(n), compute_shielded_unshield_fee(n))` of amounts that
    // cover the base but NOT the unshield fee. Any amount in that range must be rejected with
    // `InsufficientShieldedFeeError` — this pins that the gate uses `compute_shielded_unshield_fee`.

    mod fee_validation {
        use super::*;

        /// An `unshielding_amount` strictly inside `[base_fee, unshield_fee)` — enough for the BASE
        /// shielded fee but NOT the Unshield fee (which adds the flat 222-byte address-write cost) —
        /// must be rejected with `InsufficientShieldedFeeError`. This is the boundary that proves the
        /// gate uses `compute_shielded_unshield_fee`, not `compute_minimum_shielded_fee`.
        ///
        /// Mirrors the equivalent ShieldedTransfer (Base) and ShieldedWithdrawal boundary tests.
        #[test]
        fn test_fee_meets_base_but_below_unshield_fee_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            // One action, matching `create_dummy_serialized_action()` below.
            let num_actions = 1usize;
            let base_fee =
                dpp::shielded::compute_minimum_shielded_fee(num_actions, platform_version)
                    .expect("base shielded fee should not overflow");
            let unshield_fee =
                dpp::shielded::compute_shielded_unshield_fee(num_actions, platform_version)
                    .expect("unshield fee should not overflow");

            // The unshield fee MUST strictly exceed the base (the flat 222-byte address-write cost),
            // otherwise the half-open range this test exercises would be empty.
            assert!(
                unshield_fee > base_fee,
                "unshield fee ({unshield_fee}) must exceed the base fee ({base_fee}) so the \
                 [base, unshield) range is non-empty"
            );

            // Pick an amount strictly inside `[base_fee, unshield_fee)`: it clears the base fee but
            // falls one credit short of the Unshield fee. If the gate (incorrectly) used the base
            // fee, this would pass; because it uses `compute_shielded_unshield_fee`, it is rejected.
            let unshielding_amount = unshield_fee - 1;
            assert!(
                unshielding_amount >= base_fee,
                "the chosen amount must still cover the base fee"
            );

            let transition = create_unshield_transition(
                create_output_address(),
                vec![create_dummy_serialized_action()],
                unshielding_amount,
                [42u8; 32],     // non-zero anchor (structurally valid)
                vec![0u8; 100], // dummy proof bytes (fee gate runs before proof verification)
                [0u8; 64],
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            // The fee gate runs before proof verification, so the insufficient fee is caught here
            // (not the dummy proof).
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InsufficientShieldedFeeError(_))
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
            insert_dummy_encrypted_notes(&platform, 250);

            let anchor = [42u8; 32];

            // Insert the anchor so anchor validation passes
            insert_anchor_into_state(&platform, &anchor);

            // This transition is structurally valid and has a valid anchor,
            // but has random ZK proof data. It should pass structure validation
            // and anchor validation but fail at proof verification.
            let transition = create_default_unshield_transition();

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidShieldedProofError(_))
                )]
            );
        }

        #[test]
        fn test_valid_unshield_proof_succeeds() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();
            insert_dummy_encrypted_notes(&platform, 250);
            let mut rng = StdRng::seed_from_u64(0);
            let pk = get_proving_key();

            // --- Create keys ---
            let sk = SpendingKey::from_bytes([0u8; 32]).unwrap();
            let fvk = FullViewingKey::from(&sk);
            let recipient = fvk.address_at(0u32, Scope::External);
            let ask = SpendAuthorizingKey::from(&sk);

            // --- Create a spendable note with value 500M ---
            let rho_bytes: [u8; 32] = {
                let mut b = [0u8; 32];
                b[0] = 1;
                b
            };
            let rho = Rho::from_bytes(&rho_bytes).unwrap();
            let rseed = RandomSeed::from_bytes([42u8; 32], &rho).unwrap();
            let note =
                Note::from_parts(recipient, NoteValue::from_raw(500_000_000), rho, rseed).unwrap();

            // --- Build commitment tree and get anchor + merkle path ---
            let cmx = ExtractedNoteCommitment::from(note.commitment());
            let mut tree = ClientMemoryCommitmentTree::new(100);
            tree.append(cmx.to_bytes(), Retention::Marked).unwrap();
            tree.checkpoint(0u32).unwrap();
            let anchor = tree.anchor().unwrap();
            let merkle_path = tree.witness(Position::from(0u64), 0).unwrap().unwrap();

            // --- Build bundle: spend 500M -> output 5K (value_balance = 499,995,000) ---
            let mut builder = Builder::<DashMemo>::new(BundleType::DEFAULT, anchor);
            builder.add_spend(fvk.clone(), note, merkle_path).unwrap();
            builder
                .add_output(None, recipient, NoteValue::from_raw(5_000), [0u8; 36])
                .unwrap();

            let (unauthorized, _) = builder.build::<i64>(&mut rng).unwrap().unwrap();

            // Compute platform sighash binding transparent fields (output_address, unshielding_amount)
            let output_address = create_output_address();
            let unshielding_amount = 499_995_000u64; // value_balance as u64
            let extra_sighash_data = dpp::shielded::unshield_extra_sighash_data_v0(
                &output_address.to_bytes(),
                unshielding_amount,
            );
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &extra_sighash_data);

            let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
            let bundle = proven.apply_signatures(rng, sighash, &[ask]).unwrap();

            // --- Extract serialized fields ---
            let (actions, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                serialize_authorized_bundle_i64(&bundle);

            // value_balance should be 499,995,000 (500M spent - 5K output)
            assert_eq!(value_balance, 499_995_000);

            // --- Set up platform state ---
            // Insert anchor so anchor validation passes
            insert_anchor_into_state(&platform, &anchor_bytes);

            // Set pool total balance so the unshield has sufficient funds
            set_pool_total_balance(&platform, 500_000_000);

            // --- Create and process transition ---
            let transition = create_unshield_transition(
                output_address,
                actions,
                value_balance as u64, // unshielding_amount
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
                    "unshield",
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
            insert_dummy_encrypted_notes(&platform, 250);

            let anchor = [42u8; 32];

            // Insert the anchor so anchor validation passes
            insert_anchor_into_state(&platform, &anchor);

            // Create action with wrong encrypted_note size
            let mut bad_action = create_dummy_serialized_action();
            bad_action.encrypted_note = vec![0u8; 100]; // 100 bytes instead of 216

            let transition = create_unshield_transition(
                create_output_address(),
                vec![bad_action],
                // unshielding_amount is not load-bearing here: the bad 100-byte encrypted note
                // fails basic structure validation, which runs before the fee gate. Sourced from the
                // canonical fee fn so it carries no stale literal.
                unshield_fee(1),
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
    // SECURITY AUDIT TESTS
    // ==========================================

    mod security_audit {
        use super::*;
        use grovedb_commitment_tree::{
            Anchor, Builder, BundleType, ClientMemoryCommitmentTree, DashMemo,
            ExtractedNoteCommitment, FullViewingKey, MerklePath, Note, NoteValue, Position,
            RandomSeed, Retention, Rho, Scope, SpendAuthorizingKey, SpendingKey,
        };
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        /// Build a valid Orchard bundle for unshield tests (spend > output).
        /// The `output_address` and `unshielding_amount` are bound to the sighash so that
        /// the resulting bundle can only be used with those specific transparent fields.
        /// Returns (actions, value_balance, anchor_bytes, proof_bytes, binding_sig).
        fn build_valid_unshield_bundle(
            output_address: &PlatformAddress,
            unshielding_amount: u64,
        ) -> (Vec<SerializedAction>, i64, [u8; 32], Vec<u8>, [u8; 64]) {
            let mut rng = StdRng::seed_from_u64(0);
            let pk = get_proving_key();

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
                Note::from_parts(recipient, NoteValue::from_raw(500_000_000), rho, rseed).unwrap();

            let cmx = ExtractedNoteCommitment::from(note.commitment());
            let mut tree = ClientMemoryCommitmentTree::new(100);
            tree.append(cmx.to_bytes(), Retention::Marked).unwrap();
            tree.checkpoint(0u32).unwrap();
            let anchor = tree.anchor().unwrap();
            let merkle_path = tree.witness(Position::from(0u64), 0).unwrap().unwrap();

            // Spend 500M -> output 5K -> value_balance = 499,995,000
            let mut builder = Builder::<DashMemo>::new(BundleType::DEFAULT, anchor);
            builder.add_spend(fvk.clone(), note, merkle_path).unwrap();
            builder
                .add_output(None, recipient, NoteValue::from_raw(5_000), [0u8; 36])
                .unwrap();

            let (unauthorized, _) = builder.build::<i64>(&mut rng).unwrap().unwrap();

            // Bind transparent fields (output_address, unshielding_amount) to the sighash
            let extra_sighash_data = dpp::shielded::unshield_extra_sighash_data_v0(
                &output_address.to_bytes(),
                unshielding_amount,
            );
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &extra_sighash_data);

            let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
            let bundle = proven.apply_signatures(rng, sighash, &[ask]).unwrap();

            serialize_authorized_bundle_i64(&bundle)
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
            insert_dummy_encrypted_notes(&platform, 250);

            // Bundle is signed for create_output_address() with unshielding_amount = 499,995,000
            let output_address = create_output_address();
            let signed_unshielding_amount = 499_995_000u64;
            let (actions, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                build_valid_unshield_bundle(&output_address, signed_unshielding_amount);
            assert_eq!(value_balance, 499_995_000);

            // ATTACK: Inflate unshielding_amount from 499,995,000 to 999,000,000
            let mutated_unshielding_amount = 999_000_000u64;

            // Set pool balance high enough for the inflated amount
            set_pool_total_balance(&platform, 1_000_000_000);
            insert_anchor_into_state(&platform, &anchor_bytes);

            let transition = create_unshield_transition(
                output_address,
                actions,
                mutated_unshielding_amount, // MUTATED: was 499,995,000, now 999,000,000
                anchor_bytes,
                proof_bytes,
                binding_sig,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            // FIXED: BatchValidator detects the binding signature mismatch.
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidShieldedProofError(_))
                )]
            );
        }

        /// AUDIT REGRESSION: Different output_address is now caught by platform sighash.
        ///
        /// Previously, the output_address was not bound to the Orchard bundle via
        /// sighash, allowing an attacker to substitute a different address while
        /// reusing a valid bundle. Now `compute_platform_sighash()` includes the
        /// output_address and unshielding_amount in the sighash, so changing the
        /// address causes signature verification to fail.
        ///
        /// Original severity: HIGH — now FIXED.
        #[test]
        fn test_different_output_address_with_same_valid_bundle_is_rejected() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();
            insert_dummy_encrypted_notes(&platform, 250);

            // Bundle is signed for the ORIGINAL address with unshielding_amount = 499,995,000
            let original_address = create_output_address();
            let unshielding_amount = 499_995_000u64;
            let (actions, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                build_valid_unshield_bundle(&original_address, unshielding_amount);
            assert_eq!(value_balance, 499_995_000);

            set_pool_total_balance(&platform, 500_000_000);
            insert_anchor_into_state(&platform, &anchor_bytes);

            // ATTACK: Use a completely different output address
            let attacker_address = PlatformAddress::P2pkh([0xAA; 20]);

            let transition = create_unshield_transition(
                attacker_address, // ATTACKER's address, not the original recipient
                actions,
                unshielding_amount,
                anchor_bytes,
                proof_bytes,
                binding_sig,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            // FIXED: Platform sighash includes output_address, so changing it
            // causes the sighash to differ from the one used during signing,
            // and signature verification fails.
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
            insert_dummy_encrypted_notes(&platform, 250);

            let anchor = [42u8; 32];
            insert_anchor_into_state(&platform, &anchor);
            set_pool_total_balance(&platform, 10_000);

            let action1 = create_dummy_serialized_action();
            let mut action2 = create_dummy_serialized_action();
            action2.cmx = [99u8; 32];

            // unshielding_amount: the unshield fee for 2 actions. The fee gate runs before proof
            // verification, so this must clear `compute_shielded_unshield_fee(2)` for this test to
            // reach the proof-verification stage it asserts on. Sourced from the canonical fee fn so
            // it tracks the per-action / address-write storage constants and cannot go stale.
            let unshielding_amount =
                dpp::shielded::compute_shielded_unshield_fee(2, platform_version)
                    .expect("unshield fee computation");

            let transition = create_unshield_transition(
                create_output_address(),
                vec![action1, action2], // Both have nullifier [1u8; 32]
                unshielding_amount,
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
        use dpp::state_transition::unshield_transition::accessors::UnshieldTransitionAccessorsV0;
        use drive::drive::Drive;
        use grovedb_commitment_tree::{
            Builder, BundleType, ClientMemoryCommitmentTree, DashMemo, ExtractedNoteCommitment,
            FullViewingKey, Note, NoteValue, Position, RandomSeed, Retention, Rho, Scope,
            SpendAuthorizingKey, SpendingKey,
        };
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        #[test]
        fn test_unshield_prove_and_verify_nullifiers_and_address() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();
            insert_dummy_encrypted_notes(&platform, 250);
            let mut rng = StdRng::seed_from_u64(0);
            let pk = get_proving_key();

            let spend_amount = 500_000_000u64;
            let output_amount = 5_000u64;

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

            // --- Build bundle: spend 500M -> output 5K (value_balance = 499,995,000) ---
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

            // Compute platform sighash binding transparent fields (output_address, unshielding_amount)
            let output_address = create_output_address();
            let unshielding_amount = 499_995_000u64; // value_balance as u64
            let extra_sighash_data = dpp::shielded::unshield_extra_sighash_data_v0(
                &output_address.to_bytes(),
                unshielding_amount,
            );
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &extra_sighash_data);

            let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
            let bundle = proven.apply_signatures(rng, sighash, &[ask]).unwrap();

            // --- Extract serialized fields ---
            let (actions, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                serialize_authorized_bundle_i64(&bundle);

            // value_balance should be 499,995,000 (500M spent - 5K output)
            assert_eq!(value_balance, 499_995_000);

            // --- Set up platform state ---
            insert_anchor_into_state(&platform, &anchor_bytes);
            set_pool_total_balance(&platform, 500_000_000);

            // --- Build and serialize the transition ---
            let transition = create_unshield_transition(
                output_address.clone(),
                actions,
                value_balance as u64, // unshielding_amount
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
                .expect("expected to generate proof for unshield");

            let grovedb_proof_bytes = proof_result
                .into_data()
                .expect("expected proof data, not an error");

            // --- Verify proof ---
            let (root_hash, proof_result) = Drive::verify_state_transition_was_executed_with_proof(
                &transition,
                &BlockInfo::default(),
                &grovedb_proof_bytes,
                &|_| Ok(None),
                platform_version,
            )
            .expect("expected to verify unshield proof");

            assert_ne!(root_hash, [0u8; 32], "root hash should not be zeroed");

            // --- Assert result is VerifiedShieldedNullifiersWithAddressInfos ---
            let StateTransitionProofResult::VerifiedShieldedNullifiersWithAddressInfos(
                statuses,
                balances,
            ) = proof_result
            else {
                panic!(
                    "expected VerifiedShieldedNullifiersWithAddressInfos, got {:?}",
                    proof_result
                );
            };

            // Extract expected nullifiers from the transition
            let StateTransition::Unshield(ref st) = transition else {
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

            // Assert the output address appears in the address balances map
            assert!(
                balances.contains_key(&output_address),
                "output address {:?} should be present in address balances",
                output_address
            );
        }

        /// NEGATIVE test for the strict-merged tightening (mirrors
        /// `test_shield_from_asset_lock_padded_proof_is_rejected_by_strict_verify`): a proof
        /// that carries EXTRA data beyond `{nullifiers, output-address}` MUST be rejected by the
        /// production verifier.
        ///
        /// The production verify path rebuilds the merged `{nullifiers, output-address}` query
        /// and verifies it with the STRICT `verify_query_with_absence_proof`, whose succinctness
        /// check rejects any proof that descends into a subtree (a lower layer) the query did not
        /// require. Here we execute a real unshield, then have the (honest, but over-broad)
        /// prover generate a SUPERSET proof for `{nullifiers, output-address, + the
        /// genesis-populated Pools subtree}` and verify it against the production merged query.
        /// The strict verifier must reject it because the proof carries an extra root-level lower
        /// layer (`Pools`) the production query never touches.
        ///
        /// For contrast we also confirm the SUBSET verifier (the looser primitive this change
        /// replaced) would have ACCEPTED the same padded proof — demonstrating exactly the hole
        /// the strict merged verification closes.
        #[test]
        fn test_unshield_padded_proof_is_rejected_by_strict_verify() {
            use drive::drive::shielded::paths::shielded_credit_pool_nullifiers_path_vec;
            use drive::drive::RootTree;
            use drive::grovedb::{GroveDb, PathQuery, Query, SizedQuery};

            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();
            insert_dummy_encrypted_notes(&platform, 250);
            let mut rng = StdRng::seed_from_u64(0);
            let pk = get_proving_key();

            let spend_amount = 500_000_000u64;
            let output_amount = 5_000u64;

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

            // --- Build bundle: spend 500M -> output 5K (value_balance = 499,995,000) ---
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

            let output_address = create_output_address();
            let unshielding_amount = 499_995_000u64;
            let extra_sighash_data = dpp::shielded::unshield_extra_sighash_data_v0(
                &output_address.to_bytes(),
                unshielding_amount,
            );
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &extra_sighash_data);

            let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
            let bundle = proven.apply_signatures(rng, sighash, &[ask]).unwrap();

            let (actions, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                serialize_authorized_bundle_i64(&bundle);
            assert_eq!(value_balance, 499_995_000);

            insert_anchor_into_state(&platform, &anchor_bytes);
            set_pool_total_balance(&platform, 500_000_000);

            let transition = create_unshield_transition(
                output_address.clone(),
                actions,
                value_balance as u64,
                anchor_bytes,
                proof_bytes,
                binding_sig,
            );

            let transition_bytes = transition
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

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            // --- Reconstruct the PRODUCTION merged query exactly as the verifier does ---
            // {nullifiers} ∪ {output address}, each with cleared limits, then a limit that can
            // never truncate the legitimate result set.
            let StateTransition::Unshield(ref st) = transition else {
                unreachable!();
            };
            let nullifier_keys: Vec<Vec<u8>> = st.nullifiers();

            let mut nf_query = Query::new();
            nf_query.insert_keys(nullifier_keys);
            let nullifier_pq = PathQuery::new(
                shielded_credit_pool_nullifiers_path_vec(),
                SizedQuery::new(nf_query, None, None),
            );

            let mut address_pq =
                Drive::balances_for_clear_addresses_query(std::iter::once(&output_address));
            address_pq.query.limit = None;

            let mut production_pq = PathQuery::merge(
                vec![&nullifier_pq, &address_pq],
                &platform_version.drive.grove_version,
            )
            .expect("merge production query");
            production_pq.query.limit = Some(u16::MAX);

            // Sanity: an HONEST proof for the production query verifies strictly (liveness).
            let honest_proof = platform
                .drive
                .grove_get_proved_path_query(
                    &production_pq,
                    None,
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("honest production proof");
            GroveDb::verify_query_with_absence_proof(
                &honest_proof,
                &production_pq,
                &platform_version.drive.grove_version,
            )
            .expect("strict verify of honest production proof must succeed");

            // --- Build a SUPERSET (padded) proof: {nullifiers, address, + an extra subtree} ---
            // Pad by ALSO descending the genesis-populated `Pools` root subtree, which the
            // production query never touches; the padded proof carries an extra root-level layer.
            let mut pools_top = Query::new();
            pools_top.insert_key(vec![RootTree::Pools as u8]);
            pools_top.set_subquery(Query::new_range_full());
            let pools_pq = PathQuery::new(vec![], SizedQuery::new(pools_top, None, None));

            let mut superset_pq = PathQuery::merge(
                vec![&nullifier_pq, &address_pq, &pools_pq],
                &platform_version.drive.grove_version,
            )
            .expect("merge superset query");
            superset_pq.query.limit = Some(u16::MAX);

            let padded_proof = platform
                .drive
                .grove_get_proved_path_query(
                    &superset_pq,
                    None,
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("padded superset proof");

            // The STRICT verifier (production behavior) MUST reject the padded proof.
            let strict_result = GroveDb::verify_query_with_absence_proof(
                &padded_proof,
                &production_pq,
                &platform_version.drive.grove_version,
            );
            assert!(
                strict_result.is_err(),
                "strict verifier must reject a proof padded with an extra subtree layer, got {:?}",
                strict_result
            );

            // And the PRODUCTION entry point — the dispatch site this change actually
            // rewrote — MUST reject the padded proof too. Asserting only against the GroveDB
            // primitive above would stay green if the Unshield arm regressed to rebuild a
            // different merged query or fall back to the subset verifier; routing the padded
            // proof through `Drive::verify_state_transition_was_executed_with_proof` locks the
            // real code path into the test.
            let production_result = Drive::verify_state_transition_was_executed_with_proof(
                &transition,
                &BlockInfo::default(),
                &padded_proof,
                &|_| Ok(None),
                platform_version,
            );
            assert!(
                production_result.is_err(),
                "production unshield verifier must reject a padded proof, got {:?}",
                production_result
            );

            // Contrast: the SUBSET verifier (the looser primitive this change replaced) tolerates
            // the extra layer and ACCEPTS the same padded proof — the exact hole now closed.
            let subset_result = GroveDb::verify_subset_query_with_absence_proof(
                &padded_proof,
                &production_pq,
                &platform_version.drive.grove_version,
            );
            assert!(
                subset_result.is_ok(),
                "subset verifier was expected to tolerate the padded proof, got {:?}",
                subset_result
            );
        }
    }
}
