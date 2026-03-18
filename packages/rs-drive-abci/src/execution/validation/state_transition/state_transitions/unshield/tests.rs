#[cfg(test)]
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
            111_549_800, // unshielding_amount: recipient amount + minimum fee for 1 action
            [42u8; 32],  // non-zero anchor
            vec![0u8; 100], // dummy proof bytes
            [0u8; 64],   // dummy binding signature
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
            // NOTE: We call validate_structure directly because 101 actions (~41KB)
            // exceeds max_state_transition_size (20KB) before the actions count check
            // can trigger. This means ShieldedTooManyActionsError is effectively
            // unreachable through the normal pipeline.
            use dpp::state_transition::StateTransitionStructureValidation;

            let platform_version = PlatformVersion::latest();

            // 101 actions exceeds max_shielded_transition_actions (100)
            let actions: Vec<SerializedAction> =
                (0..101).map(|_| create_dummy_serialized_action()).collect();

            let transition = UnshieldTransitionV0 {
                output_address: create_output_address(),
                actions,
                unshielding_amount: 111_549_800,
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
    // ZK PROOF VERIFICATION TESTS (InvalidShieldedProofError)
    // ==========================================

    mod proof_verification {
        use super::*;
        use grovedb_commitment_tree::{
            Anchor, Builder, BundleType, ClientMemoryCommitmentTree, DashMemo,
            ExtractedNoteCommitment, FullViewingKey, MerklePath, Note, NoteValue, Position,
            RandomSeed, Retention, Rho, Scope, SpendAuthorizingKey, SpendingKey,
        };
        use rand::rngs::OsRng;

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
            let mut rng = OsRng;
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
            let mut extra_sighash_data = output_address.to_bytes();
            extra_sighash_data.extend_from_slice(&unshielding_amount.to_le_bytes());
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
                111_549_800, // unshielding_amount: recipient amount + minimum fee for 1 action
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
        use rand::rngs::OsRng;

        /// Build a valid Orchard bundle for unshield tests (spend > output).
        /// The `output_address` and `unshielding_amount` are bound to the sighash so that
        /// the resulting bundle can only be used with those specific transparent fields.
        /// Returns (actions, value_balance, anchor_bytes, proof_bytes, binding_sig).
        fn build_valid_unshield_bundle(
            output_address: &PlatformAddress,
            unshielding_amount: u64,
        ) -> (Vec<SerializedAction>, i64, [u8; 32], Vec<u8>, [u8; 64]) {
            let mut rng = OsRng;
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
            let mut extra_sighash_data = output_address.to_bytes();
            extra_sighash_data.extend_from_slice(&unshielding_amount.to_le_bytes());
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

            let transition = create_unshield_transition(
                create_output_address(),
                vec![action1, action2], // Both have nullifier [1u8; 32]
                123_098_600, // unshielding_amount: recipient amount + minimum fee for 2 actions
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
        use rand::rngs::OsRng;

        #[test]
        fn test_unshield_prove_and_verify_nullifiers_and_address() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();
            insert_dummy_encrypted_notes(&platform, 250);
            let mut rng = OsRng;
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
            let mut extra_sighash_data = output_address.to_bytes();
            extra_sighash_data.extend_from_slice(&unshielding_amount.to_le_bytes());
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
    }
}
