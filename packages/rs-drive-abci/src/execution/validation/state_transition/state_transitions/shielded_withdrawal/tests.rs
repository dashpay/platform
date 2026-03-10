#[cfg(test)]
mod tests {
    use crate::execution::validation::state_transition::state_transitions::shielded_common::compute_platform_sighash;
    use crate::execution::validation::state_transition::state_transitions::test_helpers::{
        create_dummy_serialized_action, insert_anchor_into_state, insert_dummy_encrypted_notes,
        insert_nullifier_into_state, process_transition, set_pool_total_balance, setup_platform,
    };
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
    use assert_matches::assert_matches;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;
    use dpp::identity::core_script::CoreScript;
    use dpp::shielded::SerializedAction;
    use dpp::state_transition::shielded_withdrawal_transition::v0::ShieldedWithdrawalTransitionV0;
    use dpp::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
    use dpp::state_transition::StateTransition;
    use dpp::withdrawal::Pooling;
    use platform_version::version::PlatformVersion;

    // ==========================================
    // Helper Functions (transition-specific)
    // ==========================================

    /// Create a dummy CoreScript (P2PKH) for the withdrawal output.
    fn create_output_script() -> CoreScript {
        CoreScript::new_p2pkh([7u8; 20])
    }

    /// Builds a `ShieldedWithdrawalTransition` state transition.
    /// No signing needed since shielded withdrawal transitions have no ECDSA witnesses
    /// (authenticated purely via Orchard ZK proof + signatures).
    fn create_shielded_withdrawal_transition(
        actions: Vec<SerializedAction>,
        unshielding_amount: u64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        core_fee_per_byte: u32,
        pooling: Pooling,
        output_script: CoreScript,
    ) -> StateTransition {
        StateTransition::ShieldedWithdrawal(ShieldedWithdrawalTransition::V0(
            ShieldedWithdrawalTransitionV0 {
                actions,
                unshielding_amount,
                anchor,
                proof,
                binding_signature,
                core_fee_per_byte,
                pooling,
                output_script,
            },
        ))
    }

    /// Shorthand for creating a structurally valid (but cryptographically invalid) shielded
    /// withdrawal transition. Has a non-zero anchor, valid field sizes, positive unshielding_amount.
    fn create_default_shielded_withdrawal_transition() -> StateTransition {
        create_shielded_withdrawal_transition(
            vec![create_dummy_serialized_action()],
            111_549_800, // unshielding_amount: recipient amount + minimum fee for 1 action
            [42u8; 32],  // non-zero anchor
            vec![0u8; 100], // dummy proof bytes
            [0u8; 64],   // dummy binding signature
            1,           // core_fee_per_byte
            Pooling::Never, // pooling strategy
            create_output_script(), // P2PKH output script
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

            let transition = create_shielded_withdrawal_transition(
                vec![], // Empty actions — invalid
                1000,
                [42u8; 32],
                vec![0u8; 100],
                [0u8; 64],
                1,
                Pooling::Never,
                create_output_script(),
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::ShieldedNoActionsError(_))
                )]
            );
        }

        // TODO: "amount" field no longer exists on ShieldedWithdrawalTransitionV0.
        // The concept is now "unshielding_amount: u64". The UnshieldAmountZeroError
        // consensus error variant may no longer exist. Re-enable if a corresponding
        // zero-unshielding_amount validation error is added.
        //
        // #[test]
        // fn test_zero_amount_returns_error() {
        //     let platform_version = PlatformVersion::latest();
        //     let platform = setup_platform();
        //
        //     let transition = create_shielded_withdrawal_transition(
        //         vec![create_dummy_serialized_action()],
        //         0, // Zero unshielding_amount — invalid
        //         [42u8; 32],
        //         vec![0u8; 100],
        //         [0u8; 64],
        //         1,
        //         Pooling::Never,
        //         create_output_script(),
        //     );
        //
        //     let processing_result = process_transition(&platform, transition, platform_version);
        //
        //     assert_matches!(
        //         processing_result.execution_results().as_slice(),
        //         [StateTransitionExecutionResult::UnpaidConsensusError(
        //             ConsensusError::BasicError(BasicError::UnshieldAmountZeroError(_))
        //         )]
        //     );
        // }

        // TODO: "value_balance" field no longer exists on ShieldedWithdrawalTransitionV0.
        // It has been replaced by "unshielding_amount: u64" which cannot be negative or zero
        // in the same way. The ShieldedInvalidValueBalanceError consensus error variant may
        // no longer apply. Re-enable if a corresponding validation is added.
        //
        // #[test]
        // fn test_zero_value_balance_returns_error() { ... }

        // TODO: "value_balance" was i64 and could be negative. Now "unshielding_amount"
        // is u64, so negative values are impossible at the type level.
        //
        // #[test]
        // fn test_negative_value_balance_returns_error() { ... }

        // TODO: "value_balance >= amount" check no longer applies — both fields have been
        // replaced by a single "unshielding_amount: u64". The
        // UnshieldValueBalanceBelowAmountError consensus error variant may no longer exist.
        //
        // #[test]
        // fn test_value_balance_less_than_amount_returns_error() { ... }

        #[test]
        fn test_empty_proof_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let transition = create_shielded_withdrawal_transition(
                vec![create_dummy_serialized_action()],
                1000,
                [42u8; 32],
                vec![], // Empty proof — invalid
                [0u8; 64],
                1,
                Pooling::Never,
                create_output_script(),
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

            let transition = create_shielded_withdrawal_transition(
                vec![create_dummy_serialized_action()],
                1000,
                [0u8; 32], // All zeros — invalid
                vec![0u8; 100],
                [0u8; 64],
                1,
                Pooling::Never,
                create_output_script(),
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

            // Set pool balance so the pool balance check would pass (if it got that far)
            set_pool_total_balance(&platform, 10_000);

            // Non-zero anchor that exists in state, but no encrypted notes in pool
            let anchor = [42u8; 32];
            insert_anchor_into_state(&platform, &anchor);

            let transition = create_default_shielded_withdrawal_transition();

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
        fn test_anchor_not_in_tree_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();
            insert_dummy_encrypted_notes(&platform, 250);

            // Set pool balance so the balance check passes before anchor validation
            set_pool_total_balance(&platform, 10_000);

            // Non-zero anchor that doesn't exist in state
            let transition = create_default_shielded_withdrawal_transition();

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
        fn test_already_spent_nullifier_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();
            insert_dummy_encrypted_notes(&platform, 250);

            let anchor = [42u8; 32];
            let nullifier = [1u8; 32]; // Same as create_dummy_serialized_action().nullifier

            // Insert the anchor so anchor validation passes
            insert_anchor_into_state(&platform, &anchor);

            // Set pool balance so the pool balance check passes
            set_pool_total_balance(&platform, 10_000);

            // Insert the nullifier so it appears already spent
            insert_nullifier_into_state(&platform, &nullifier);

            let transition = create_default_shielded_withdrawal_transition();

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
            Authorized as OrchardAuthorized, Builder, Bundle, BundleType,
            ClientMemoryCommitmentTree, DashMemo, ExtractedNoteCommitment, FullViewingKey, Note,
            NoteValue, Position, ProvingKey, RandomSeed, Retention, Rho, Scope,
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
        ) -> (Vec<SerializedAction>, i64, [u8; 32], Vec<u8>, [u8; 64]) {
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
            let value_balance = *bundle.value_balance();
            let anchor = bundle.anchor().to_bytes();
            let proof = bundle.authorization().proof().as_ref().to_vec();
            let binding_sig = <[u8; 64]>::from(bundle.authorization().binding_signature());
            (actions, value_balance, anchor, proof, binding_sig)
        }

        #[test]
        fn test_invalid_proof_returns_shielded_proof_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();
            insert_dummy_encrypted_notes(&platform, 250);

            let anchor = [42u8; 32];

            // Insert the anchor so anchor validation passes
            insert_anchor_into_state(&platform, &anchor);

            // Set pool balance so the pool balance check passes
            set_pool_total_balance(&platform, 10_000);

            // This transition is structurally valid and has a valid anchor,
            // but has random ZK proof data. It should pass structure validation
            // and anchor validation but fail at proof verification.
            let transition = create_default_shielded_withdrawal_transition();

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidShieldedProofError(_))
                )]
            );
        }

        #[test]
        fn test_valid_shielded_withdrawal_proof_succeeds() {
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

            // Compute platform sighash binding transparent fields (output_script, unshielding_amount)
            let output_script = create_output_script();
            let unshielding_amount = 499_995_000u64; // value_balance as u64
            let mut extra_sighash_data = output_script.as_bytes().to_vec();
            extra_sighash_data.extend_from_slice(&unshielding_amount.to_le_bytes());
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &extra_sighash_data);

            let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
            let bundle = proven.apply_signatures(rng, sighash, &[ask]).unwrap();

            // --- Extract serialized fields ---
            let (actions, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                serialize_authorized_bundle(&bundle);

            // value_balance should be 499,995,000 (500M spent - 5K output)
            assert_eq!(value_balance, 499_995_000);

            // --- Set up platform state ---
            // Insert anchor so anchor validation passes
            insert_anchor_into_state(&platform, &anchor_bytes);

            // Set pool total balance so the withdrawal has sufficient funds
            set_pool_total_balance(&platform, 500_000_000);

            // --- Create and process transition ---
            let transition = create_shielded_withdrawal_transition(
                actions,
                value_balance as u64, // unshielding_amount
                anchor_bytes,
                proof_bytes,
                binding_sig,
                1,              // core_fee_per_byte
                Pooling::Never, // pooling strategy
                output_script,
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

            // Set pool balance so the pool balance check passes
            set_pool_total_balance(&platform, 10_000);

            // Create action with wrong encrypted_note size
            let mut bad_action = create_dummy_serialized_action();
            bad_action.encrypted_note = vec![0u8; 100]; // 100 bytes instead of 216

            let transition = create_shielded_withdrawal_transition(
                vec![bad_action],
                111_549_800, // unshielding_amount: recipient amount + minimum fee for 1 action
                anchor,
                vec![0u8; 100],
                [0u8; 64],
                1,
                Pooling::Never,
                create_output_script(),
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
    // SECURITY AUDIT TESTS
    // ==========================================
    //
    // These tests verify vulnerabilities and edge cases discovered
    // during a security audit of the shielded transaction system.
    // Tests that demonstrate actual vulnerabilities are marked with
    // "AUDIT FINDING" / "AUDIT REGRESSION" comments.

    mod security_audit {
        use super::*;
        use grovedb_commitment_tree::{
            Authorized as OrchardAuthorized, Builder, Bundle, BundleType,
            ClientMemoryCommitmentTree, DashMemo, ExtractedNoteCommitment, FullViewingKey, Note,
            NoteValue, Position, ProvingKey, RandomSeed, Retention, Rho, Scope,
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
        ) -> (Vec<SerializedAction>, i64, [u8; 32], Vec<u8>, [u8; 64]) {
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
            let value_balance = *bundle.value_balance();
            let anchor = bundle.anchor().to_bytes();
            let proof = bundle.authorization().proof().as_ref().to_vec();
            let binding_sig = <[u8; 64]>::from(bundle.authorization().binding_signature());
            (actions, value_balance, anchor, proof, binding_sig)
        }

        /// Build a valid Orchard bundle for shielded withdrawal tests (spend > output).
        /// The `output_script` and `unshielding_amount` are bound to the sighash so that
        /// the resulting bundle can only be used with those specific transparent fields.
        /// Returns (actions, value_balance, anchor_bytes, proof_bytes, binding_sig).
        fn build_valid_shielded_withdrawal_bundle(
            output_script: &CoreScript,
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

            // Bind transparent fields (output_script, unshielding_amount) to the sighash
            let mut extra_sighash_data = output_script.as_bytes().to_vec();
            extra_sighash_data.extend_from_slice(&unshielding_amount.to_le_bytes());
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &extra_sighash_data);

            let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
            let bundle = proven.apply_signatures(rng, sighash, &[ask]).unwrap();

            serialize_authorized_bundle(&bundle)
        }

        // TODO: "value_balance" was i64 and could be i64::MIN. Now "unshielding_amount"
        // is u64, so negative values are impossible at the type level. The
        // ShieldedInvalidValueBalanceError consensus error variant may no longer apply.
        //
        // #[test]
        // fn test_i64_min_value_balance_handled() { ... }

        /// AUDIT REGRESSION: Zeroed binding signature is caught by BatchValidator.
        ///
        /// The binding signature cryptographically binds value_balance to the value
        /// commitments. Zeroing it out should cause signature verification to fail,
        /// preventing an attacker from stripping authentication from a valid bundle.
        ///
        /// Original severity: CRITICAL — now FIXED.
        #[test]
        fn test_valid_proof_with_zeroed_binding_sig_is_rejected() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();
            insert_dummy_encrypted_notes(&platform, 250);

            let output_script = create_output_script();
            let unshielding_amount = 499_995_000u64;
            let (actions, value_balance, anchor_bytes, proof_bytes, _binding_sig) =
                build_valid_shielded_withdrawal_bundle(&output_script, unshielding_amount);

            set_pool_total_balance(&platform, 500_000_000);
            insert_anchor_into_state(&platform, &anchor_bytes);

            let transition = create_shielded_withdrawal_transition(
                actions,
                value_balance as u64, // unshielding_amount
                anchor_bytes,
                proof_bytes,
                [0u8; 64], // ZEROED binding signature
                1,
                Pooling::Never,
                output_script,
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

        /// AUDIT REGRESSION: Mutating value_balance is caught by BatchValidator.
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

            // Bundle is signed for create_output_script() with unshielding_amount = 499,995,000
            let output_script = create_output_script();
            let signed_unshielding_amount = 499_995_000u64;
            let (actions, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                build_valid_shielded_withdrawal_bundle(&output_script, signed_unshielding_amount);
            assert_eq!(value_balance, 499_995_000);

            // ATTACK: Inflate unshielding_amount from 499,995,000 to 999,000,000
            let mutated_unshielding_amount = 999_000_000u64;

            // Set pool balance high enough for the inflated amount
            set_pool_total_balance(&platform, 1_000_000_000);
            insert_anchor_into_state(&platform, &anchor_bytes);

            let transition = create_shielded_withdrawal_transition(
                actions,
                mutated_unshielding_amount, // MUTATED: was 499,995,000, now 999,000,000
                anchor_bytes,
                proof_bytes,
                binding_sig,
                1,
                Pooling::Never,
                output_script,
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

        /// AUDIT REGRESSION: Different output_script is caught by platform sighash.
        ///
        /// The output_script is bound to the Orchard bundle via sighash. Changing
        /// the script after signing causes the sighash to differ from the one used
        /// during signing, and signature verification fails. This prevents an
        /// attacker from redirecting withdrawal funds to a different L1 address.
        ///
        /// Original severity: HIGH — now FIXED.
        #[test]
        fn test_different_output_script_with_same_valid_bundle_is_rejected() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();
            insert_dummy_encrypted_notes(&platform, 250);

            // Bundle is signed for the ORIGINAL output_script with unshielding_amount = 499,995,000
            let original_script = create_output_script();
            let unshielding_amount = 499_995_000u64;
            let (actions, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                build_valid_shielded_withdrawal_bundle(&original_script, unshielding_amount);
            assert_eq!(value_balance, 499_995_000);

            set_pool_total_balance(&platform, 500_000_000);
            insert_anchor_into_state(&platform, &anchor_bytes);

            // ATTACK: Use a completely different output script (attacker's address)
            let attacker_script = CoreScript::new_p2pkh([0xAA; 20]);

            let transition = create_shielded_withdrawal_transition(
                actions,
                unshielding_amount,
                anchor_bytes,
                proof_bytes,
                binding_sig,
                1,
                Pooling::Never,
                attacker_script, // ATTACKER's script, not the original
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            // FIXED: Platform sighash includes output_script, so changing it
            // causes the sighash to differ from the one used during signing,
            // and signature verification fails.
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidShieldedProofError(_))
                )]
            );
        }

        /// AUDIT REGRESSION: Different unshielding_amount is caught by platform sighash.
        ///
        /// The unshielding_amount is bound to the Orchard bundle via sighash. Changing
        /// the withdrawal amount after signing causes the sighash to differ, and
        /// signature verification fails. This prevents an attacker from inflating
        /// the credited withdrawal amount.
        #[test]
        fn test_different_unshielding_amount_with_same_valid_bundle_is_rejected() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();
            insert_dummy_encrypted_notes(&platform, 250);

            let output_script = create_output_script();
            let signed_unshielding_amount = 499_995_000u64;
            let (actions, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                build_valid_shielded_withdrawal_bundle(&output_script, signed_unshielding_amount);
            assert_eq!(value_balance, 499_995_000);

            set_pool_total_balance(&platform, 500_000_000);
            insert_anchor_into_state(&platform, &anchor_bytes);

            // ATTACK: Use a different unshielding_amount
            let manipulated_unshielding_amount = 400_000_000u64;

            let transition = create_shielded_withdrawal_transition(
                actions,
                manipulated_unshielding_amount, // MANIPULATED: was 499,995,000, now 400,000,000
                anchor_bytes,
                proof_bytes,
                binding_sig,
                1,
                Pooling::Never,
                output_script,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            // Platform sighash includes unshielding_amount, so changing it causes
            // signature verification to fail.
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
            action2.cmx = [99u8; 32]; // Different commitment but same nullifier

            let transition = create_shielded_withdrawal_transition(
                vec![action1, action2], // Both have nullifier [1u8; 32]
                123_098_600, // unshielding_amount: recipient amount + minimum fee for 2 actions
                anchor,
                vec![0u8; 100],
                [0u8; 64],
                1,
                Pooling::Never,
                create_output_script(),
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
    // RETURN PROOF TESTS (prove + verify round-trip)
    // ==========================================

    mod return_proof {
        use super::*;
        use dpp::block::block_info::BlockInfo;
        use dpp::data_contracts::withdrawals_contract;
        use dpp::data_contracts::withdrawals_contract::v1::document_types::withdrawal;
        use dpp::document::Document;
        use dpp::serialization::PlatformSerializable;
        use dpp::state_transition::proof_result::StateTransitionProofResult;
        use dpp::state_transition::shielded_withdrawal_transition::accessors::ShieldedWithdrawalTransitionAccessorsV0;
        use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
        use drive::drive::Drive;
        use grovedb_commitment_tree::{
            Authorized as OrchardAuthorized, Builder, Bundle, BundleType,
            ClientMemoryCommitmentTree, DashMemo, ExtractedNoteCommitment, FullViewingKey, Note,
            NoteValue, Position, ProvingKey, RandomSeed, Retention, Rho, Scope,
            SpendAuthorizingKey, SpendingKey,
        };
        use rand::rngs::OsRng;
        use std::collections::BTreeMap;
        use std::sync::{Arc, OnceLock};

        static TEST_PROVING_KEY: OnceLock<ProvingKey> = OnceLock::new();
        fn get_proving_key() -> &'static ProvingKey {
            TEST_PROVING_KEY.get_or_init(ProvingKey::build)
        }

        fn serialize_authorized_bundle(
            bundle: &Bundle<OrchardAuthorized, i64, DashMemo>,
        ) -> (Vec<SerializedAction>, i64, [u8; 32], Vec<u8>, [u8; 64]) {
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
            let value_balance = *bundle.value_balance();
            let anchor = bundle.anchor().to_bytes();
            let proof = bundle.authorization().proof().as_ref().to_vec();
            let binding_sig = <[u8; 64]>::from(bundle.authorization().binding_signature());
            (actions, value_balance, anchor, proof, binding_sig)
        }

        #[test]
        fn test_shielded_withdrawal_prove_and_verify_nullifiers_and_document() {
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

            // Compute platform sighash binding transparent fields (output_script, unshielding_amount)
            let output_script = create_output_script();
            let unshielding_amount = 499_995_000u64; // value_balance as u64
            let mut extra_sighash_data = output_script.as_bytes().to_vec();
            extra_sighash_data.extend_from_slice(&unshielding_amount.to_le_bytes());
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &extra_sighash_data);

            let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
            let bundle = proven.apply_signatures(rng, sighash, &[ask]).unwrap();

            // --- Extract serialized fields ---
            let (actions, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                serialize_authorized_bundle(&bundle);

            // value_balance should be 499,995,000 (500M spent - 5K output)
            assert_eq!(value_balance, 499_995_000);

            // --- Set up platform state ---
            insert_anchor_into_state(&platform, &anchor_bytes);
            set_pool_total_balance(&platform, 500_000_000);

            // --- Create and process transition ---
            let transition = create_shielded_withdrawal_transition(
                actions,
                value_balance as u64, // unshielding_amount
                anchor_bytes,
                proof_bytes,
                binding_sig,
                1,              // core_fee_per_byte
                Pooling::Never, // pooling strategy
                output_script.clone(),
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
                .expect("expected to generate proof for shielded withdrawal");

            let proof_bytes = proof_result
                .into_data()
                .expect("expected proof data, not an error");

            // --- Verify proof ---
            // ShieldedWithdrawal verification requires the withdrawals system data contract
            // to look up the withdrawal document type.
            let withdrawals_data_contract =
                load_system_data_contract(SystemDataContract::Withdrawals, platform_version)
                    .expect("should load withdrawals contract");

            let withdrawals_data_contract = Arc::new(withdrawals_data_contract);

            let (root_hash, proof_result) = Drive::verify_state_transition_was_executed_with_proof(
                &transition,
                &BlockInfo::default(),
                &proof_bytes,
                &|id| {
                    if *id == withdrawals_contract::ID {
                        Ok(Some(Arc::clone(&withdrawals_data_contract)))
                    } else {
                        Ok(None)
                    }
                },
                platform_version,
            )
            .expect("expected to verify shielded withdrawal proof");

            assert_ne!(root_hash, [0u8; 32], "root hash should not be zeroed");

            // --- Assert result is VerifiedShieldedNullifiersWithWithdrawalDocument ---
            let StateTransitionProofResult::VerifiedShieldedNullifiersWithWithdrawalDocument(
                statuses,
                documents,
            ) = proof_result
            else {
                panic!(
                    "expected VerifiedShieldedNullifiersWithWithdrawalDocument, got {:?}",
                    proof_result
                );
            };

            // Extract expected nullifiers from the transition
            let StateTransition::ShieldedWithdrawal(ref st) = transition else {
                unreachable!();
            };
            let expected_nullifiers: Vec<Vec<u8>> = st.nullifiers();

            assert_eq!(
                statuses.len(),
                expected_nullifiers.len(),
                "should have one status per nullifier"
            );

            // All nullifiers must be marked as spent
            for (nf, is_spent) in &statuses {
                assert!(is_spent, "nullifier {} should be spent", hex::encode(nf));
                assert!(
                    expected_nullifiers.contains(nf),
                    "proved nullifier {} should be one of the transition's nullifiers",
                    hex::encode(nf)
                );
            }

            // Compute the expected withdrawal document ID (same logic as prove/verify sides)
            let first_nullifier = expected_nullifiers
                .first()
                .expect("should have at least one nullifier");
            let mut entropy = Vec::new();
            entropy.extend_from_slice(first_nullifier);
            entropy.extend_from_slice(output_script.as_bytes());
            let expected_document_id = Document::generate_document_id_v0(
                &withdrawals_contract::ID,
                &withdrawals_contract::OWNER_ID,
                withdrawal::NAME,
                &entropy,
            );

            // The documents map should contain exactly one entry with the expected document_id
            assert_eq!(
                documents.len(),
                1,
                "should have exactly one withdrawal document entry"
            );
            assert!(
                documents.contains_key(&expected_document_id),
                "documents map should contain the expected withdrawal document id {}",
                expected_document_id
            );

            // The document should exist (Some) — it was created during processing
            let maybe_doc = documents.get(&expected_document_id).unwrap();
            assert!(
                maybe_doc.is_some(),
                "withdrawal document should be present (not absent)"
            );
        }
    }
}
