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

    /// Minimum ShieldedWithdrawal fee for `num_actions` (base + flat withdrawal-document storage
    /// cost), sourced from the canonical `dpp::shielded::compute_shielded_withdrawal_fee` (against
    /// `PlatformVersion::latest()`, which is what every test in this module uses) so the test
    /// fixtures track the fee constants and can never go stale relative to the consensus fee gate.
    fn withdrawal_fee(num_actions: usize) -> u64 {
        dpp::shielded::compute_shielded_withdrawal_fee(num_actions, PlatformVersion::latest())
            .expect("withdrawal fee computation")
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
        // unshielding_amount = the 1-action ShieldedWithdrawal fee (base + flat withdrawal-document
        // storage cost) + the min_withdrawal_amount floor for the net. Sizing it from the live fee
        // function keeps the gross above the fee gate's minimum so these state-validation tests
        // (anchor/nullifier/proof) reach the stage they intend to exercise instead of short-circuiting
        // on InsufficientShieldedFeeError, and stays correct if the fee constants change.
        let platform_version = PlatformVersion::latest();
        let fee = dpp::shielded::compute_shielded_withdrawal_fee(1, platform_version)
            .expect("fee computation should not overflow");
        let unshielding_amount = fee + platform_version.system_limits.min_withdrawal_amount;
        create_shielded_withdrawal_transition(
            vec![create_dummy_serialized_action()],
            unshielding_amount,
            [42u8; 32],             // non-zero anchor
            vec![0u8; 100],         // dummy proof bytes
            [0u8; 64],              // dummy binding signature
            1,                      // core_fee_per_byte
            Pooling::Never,         // pooling strategy
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

            let transition = ShieldedWithdrawalTransitionV0 {
                actions,
                // Any valid positive amount: this test asserts a structure-validation error
                // (ShieldedTooManyActionsError) that fires before the fee gate. Sourced from the
                // canonical fee fn so it carries no stale literal.
                unshielding_amount: withdrawal_fee(1),
                anchor: [42u8; 32],
                proof: vec![0u8; 100],
                binding_signature: [0u8; 64],
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: create_output_script(),
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

            let transition = create_shielded_withdrawal_transition(
                vec![create_dummy_serialized_action()],
                0, // Zero unshielding_amount — invalid
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
                    ConsensusError::BasicError(BasicError::ShieldedInvalidValueBalanceError(_))
                )]
            );
        }

        #[test]
        fn test_unshielding_amount_exceeding_i64_max_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let transition = create_shielded_withdrawal_transition(
                vec![create_dummy_serialized_action()],
                i64::MAX as u64 + 1, // Exceeds i64::MAX
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
                    ConsensusError::BasicError(BasicError::ShieldedInvalidValueBalanceError(_))
                )]
            );
        }

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
            Builder, BundleType, ClientMemoryCommitmentTree, DashMemo, ExtractedNoteCommitment,
            FullViewingKey, Note, NoteValue, Position, RandomSeed, Retention, Rho, Scope,
            SpendAuthorizingKey, SpendingKey,
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

            // Compute platform sighash binding transparent fields (output_script, unshielding_amount)
            let output_script = create_output_script();
            let unshielding_amount = 499_995_000u64; // value_balance as u64
            let extra_sighash_data = dpp::shielded::shielded_withdrawal_extra_sighash_data_v0(
                output_script.as_bytes(),
                unshielding_amount,
                1,
                Pooling::Never,
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
                    "shielded withdrawal",
                );
            }

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
        }

        /// Gold-standard reproduction of TestFlight report B ("shielded→Core
        /// withdrawal never lands"). Identical to
        /// `test_valid_shielded_withdrawal_proof_succeeds` — a **real** Orchard
        /// proof, correct value balance, sufficient pool balance — EXCEPT the
        /// anchor is **not** recorded in state (`insert_anchor_into_state` is
        /// omitted). A legitimate spend is then refused with `InvalidAnchorError`
        /// purely because its anchor isn't in the recorded set.
        ///
        /// This is exactly the wallet's failure mode: it builds the proof
        /// against its depth-0 tree root, but the index-chunk sync leaves that
        /// root mid-block, so Platform (which records one anchor per block) never
        /// recorded it. See `platform-wallet`'s
        /// `depth0_spend_anchor_mid_block_is_not_a_recorded_block_boundary_anchor`,
        /// which proves the wallet produces such a non-recorded anchor.
        #[test]
        fn test_valid_proof_with_unrecorded_anchor_returns_invalid_anchor_error() {
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

            // --- Build bundle: spend 500M -> output 5K ---
            let mut builder = Builder::<DashMemo>::new(BundleType::DEFAULT, anchor);
            builder.add_spend(fvk.clone(), note, merkle_path).unwrap();
            builder
                .add_output(None, recipient, NoteValue::from_raw(5_000), [0u8; 36])
                .unwrap();
            let (unauthorized, _) = builder.build::<i64>(&mut rng).unwrap().unwrap();

            let output_script = create_output_script();
            let unshielding_amount = 499_995_000u64;
            let extra_sighash_data = dpp::shielded::shielded_withdrawal_extra_sighash_data_v0(
                output_script.as_bytes(),
                unshielding_amount,
                1,
                Pooling::Never,
            );
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &extra_sighash_data);
            let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
            let bundle = proven.apply_signatures(rng, sighash, &[ask]).unwrap();

            let (actions, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                serialize_authorized_bundle_i64(&bundle);
            assert_eq!(value_balance, 499_995_000);

            // Pool balance passes; the anchor is DELIBERATELY not recorded
            // (no `insert_anchor_into_state`) — the crux of the reproduction.
            set_pool_total_balance(&platform, 500_000_000);

            let transition = create_shielded_withdrawal_transition(
                actions,
                value_balance as u64,
                anchor_bytes,
                proof_bytes,
                binding_sig,
                1,
                Pooling::Never,
                output_script,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            // Real proof passes; anchor validation then rejects the unrecorded
            // anchor. This is why the withdrawal never lands.
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidAnchorError(_))
                )]
            );
        }

        /// Keystone acceptance test for the anchor-selection fix: a **real**
        /// Orchard proof built against an OLDER recorded anchor — with the
        /// commitment tree (and the recorded set) advanced past it — must
        /// validate successfully. This is exactly the spend shape
        /// `platform-wallet`'s `select_recorded_spends` produces after the
        /// fix: witnesses taken at a deeper checkpoint whose root Platform
        /// recorded, while newer commitments and a newer boundary anchor
        /// already exist. `test_valid_shielded_withdrawal_proof_succeeds`
        /// covers only the fresh-anchor (depth-0-recorded) case, and
        /// `test_valid_proof_with_unrecorded_anchor_returns_invalid_anchor_error`
        /// only the rejection side; this pins the acceptance side the whole
        /// fix rests on.
        #[test]
        fn test_valid_proof_with_older_recorded_anchor_succeeds() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();
            insert_dummy_encrypted_notes(&platform, 250);
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

            // The spent note lands in "block 1", whose boundary root is
            // recorded (anchor_old). Later commitments then arrive and a
            // newer boundary is recorded (anchor_new). The spend is built at
            // checkpoint depth 1 — the wallet fix's exact output when its
            // depth-0 root is unrecorded.
            let cmx = ExtractedNoteCommitment::from(note.commitment());
            let mut tree = ClientMemoryCommitmentTree::new(100);
            tree.append(cmx.to_bytes(), Retention::Marked).unwrap();
            tree.checkpoint(0u32).unwrap(); // block-1 boundary

            for later in 2u8..=4 {
                let mut b = [0u8; 32];
                b[0] = later;
                let rho_later = Rho::from_bytes(&b).unwrap();
                let rseed_later = RandomSeed::from_bytes([42u8; 32], &rho_later).unwrap();
                let note_later = Note::from_parts(
                    recipient,
                    NoteValue::from_raw(1_000),
                    rho_later,
                    rseed_later,
                )
                .unwrap();
                let cmx_later = ExtractedNoteCommitment::from(note_later.commitment());
                tree.append(cmx_later.to_bytes(), Retention::Ephemeral)
                    .unwrap();
            }
            tree.checkpoint(1u32).unwrap(); // block-2 boundary

            let anchor_new = tree.anchor().unwrap();
            // Witness at checkpoint depth 1 → path rooted at the OLDER
            // (block-1) recorded root, not the tip.
            let merkle_path = tree.witness(Position::from(0u64), 1).unwrap().unwrap();
            let anchor_old = merkle_path.root(cmx);
            assert_ne!(
                anchor_old.to_bytes(),
                anchor_new.to_bytes(),
                "tree must have advanced past the spend's anchor for this test to be meaningful"
            );

            // --- Build bundle against the older anchor ---
            let mut builder = Builder::<DashMemo>::new(BundleType::DEFAULT, anchor_old);
            builder.add_spend(fvk.clone(), note, merkle_path).unwrap();
            builder
                .add_output(None, recipient, NoteValue::from_raw(5_000), [0u8; 36])
                .unwrap();
            let (unauthorized, _) = builder.build::<i64>(&mut rng).unwrap().unwrap();

            let output_script = create_output_script();
            let unshielding_amount = 499_995_000u64;
            let extra_sighash_data = dpp::shielded::shielded_withdrawal_extra_sighash_data_v0(
                output_script.as_bytes(),
                unshielding_amount,
                1,
                Pooling::Never,
            );
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &extra_sighash_data);
            let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
            let bundle = proven.apply_signatures(rng, sighash, &[ask]).unwrap();

            let (actions, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                serialize_authorized_bundle_i64(&bundle);
            assert_eq!(value_balance, 499_995_000);
            assert_eq!(
                anchor_bytes,
                anchor_old.to_bytes(),
                "the bundle must carry the older anchor"
            );

            // The recorded set holds BOTH boundaries — like drive after two
            // block-ends — and the spend references the older one.
            insert_anchor_into_state(&platform, &anchor_bytes);
            insert_anchor_into_state(&platform, &anchor_new.to_bytes());
            set_pool_total_balance(&platform, 500_000_000);

            let transition = create_shielded_withdrawal_transition(
                actions,
                value_balance as u64,
                anchor_bytes,
                proof_bytes,
                binding_sig,
                1,
                Pooling::Never,
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
                // unshielding_amount is not load-bearing here: the bad 100-byte encrypted note fails
                // basic structure validation, which runs before the fee gate. Sourced from the
                // canonical fee fn so it carries no stale literal.
                withdrawal_fee(1),
                anchor,
                vec![0u8; 100],
                [0u8; 64],
                1,
                Pooling::Never,
                create_output_script(),
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
    //
    // These tests verify vulnerabilities and edge cases discovered
    // during a security audit of the shielded transaction system.
    // Tests that demonstrate actual vulnerabilities are marked with
    // "AUDIT FINDING" / "AUDIT REGRESSION" comments.

    mod security_audit {
        use super::*;
        use grovedb_commitment_tree::{
            Builder, BundleType, ClientMemoryCommitmentTree, DashMemo, ExtractedNoteCommitment,
            FullViewingKey, Note, NoteValue, Position, RandomSeed, Retention, Rho, Scope,
            SpendAuthorizingKey, SpendingKey,
        };
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        /// Build a valid Orchard bundle for shielded withdrawal tests (spend > output).
        /// The `output_script` and `unshielding_amount` are bound to the sighash so that
        /// the resulting bundle can only be used with those specific transparent fields.
        /// Returns (actions, value_balance, anchor_bytes, proof_bytes, binding_sig).
        fn build_valid_shielded_withdrawal_bundle(
            output_script: &CoreScript,
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

            // Bind transparent fields (output_script, unshielding_amount) to the sighash
            let extra_sighash_data = dpp::shielded::shielded_withdrawal_extra_sighash_data_v0(
                output_script.as_bytes(),
                unshielding_amount,
                1,
                Pooling::Never,
            );
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &extra_sighash_data);

            let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
            let bundle = proven.apply_signatures(rng, sighash, &[ask]).unwrap();

            serialize_authorized_bundle_i64(&bundle)
        }

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

        /// AUDIT REGRESSION: core_fee_per_byte is bound to the platform sighash.
        ///
        /// `core_fee_per_byte` is written verbatim by the transformer into the queued
        /// Core withdrawal document (the asset-unlock TxOut fee rate). Structure
        /// validation constrains it to the non-zero Fibonacci set, but that set spans
        /// 1..=2,971,215,073 — so without sighash binding a relay or block proposer could
        /// flip a user's `core_fee_per_byte = 1` to a much larger Fibonacci value,
        /// redirecting the withdrawn amount into L1 miner fees while keeping the Orchard
        /// proof valid (ShieldedWithdrawal has no identity-key signature). Binding it into
        /// the sighash makes the binding signature authorize it, so any change is rejected.
        #[test]
        fn test_different_core_fee_per_byte_with_same_valid_bundle_is_rejected() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();
            insert_dummy_encrypted_notes(&platform, 250);

            // Bundle is signed for core_fee_per_byte = 1 (build_valid_shielded_withdrawal_bundle).
            let output_script = create_output_script();
            let unshielding_amount = 499_995_000u64;
            let (actions, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                build_valid_shielded_withdrawal_bundle(&output_script, unshielding_amount);
            assert_eq!(value_balance, 499_995_000);

            set_pool_total_balance(&platform, 500_000_000);
            insert_anchor_into_state(&platform, &anchor_bytes);

            // ATTACK: bump core_fee_per_byte to a *different* Fibonacci value (2), so it still
            // passes structure validation but no longer matches the signed sighash.
            let transition = create_shielded_withdrawal_transition(
                actions,
                unshielding_amount,
                anchor_bytes,
                proof_bytes,
                binding_sig,
                2, // mutated core_fee_per_byte (bundle was signed with 1)
                Pooling::Never,
                output_script,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            // FIXED: core_fee_per_byte is in the platform sighash, so the mutated value
            // yields a different sighash than was signed and signature verification fails.
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

            // unshielding_amount = the 2-action ShieldedWithdrawal fee (base + flat
            // withdrawal-document storage cost) + the min_withdrawal_amount floor for the net, so the
            // gross clears the fee gate and the flow reaches the (earlier) proof-verification stage
            // this test exercises rather than short-circuiting on InsufficientShieldedFeeError.
            let fee = dpp::shielded::compute_shielded_withdrawal_fee(2, platform_version)
                .expect("fee computation should not overflow");
            let unshielding_amount = fee + platform_version.system_limits.min_withdrawal_amount;

            let transition = create_shielded_withdrawal_transition(
                vec![action1, action2], // Both have nullifier [1u8; 32]
                unshielding_amount,
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
            Builder, BundleType, ClientMemoryCommitmentTree, DashMemo, ExtractedNoteCommitment,
            FullViewingKey, Note, NoteValue, Position, RandomSeed, Retention, Rho, Scope,
            SpendAuthorizingKey, SpendingKey,
        };
        use rand::rngs::StdRng;
        use rand::SeedableRng;
        use std::collections::BTreeMap;
        use std::sync::Arc;

        #[test]
        fn test_shielded_withdrawal_prove_and_verify_nullifiers_and_document() {
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

            // Compute platform sighash binding transparent fields (output_script, unshielding_amount)
            let output_script = create_output_script();
            let unshielding_amount = 499_995_000u64; // value_balance as u64
            let extra_sighash_data = dpp::shielded::shielded_withdrawal_extra_sighash_data_v0(
                output_script.as_bytes(),
                unshielding_amount,
                1,
                Pooling::Never,
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
            .map(|(root_hash, outcome)| (root_hash, outcome.into_result()))
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

        /// NEGATIVE test for the strict-merged tightening (mirrors
        /// `test_shield_from_asset_lock_padded_proof_is_rejected_by_strict_verify`): a proof
        /// that carries EXTRA data beyond `{nullifiers, withdrawal-document}` MUST be rejected by
        /// the production verifier.
        ///
        /// The production verify path rebuilds the merged `{nullifiers, withdrawal-document}`
        /// query and verifies it with the STRICT `verify_query_with_absence_proof`, whose
        /// succinctness check rejects any proof that descends into a subtree (a lower layer) the
        /// query did not require. Here we execute a real shielded withdrawal, then have the
        /// (honest, but over-broad) prover generate a SUPERSET proof for `{nullifiers,
        /// withdrawal-document, + the genesis-populated Pools subtree}` and verify it against the
        /// production merged query. The strict verifier must reject it because the proof carries
        /// an extra root-level lower layer (`Pools`) the production query never touches.
        ///
        /// For contrast we also confirm the SUBSET verifier (the looser primitive this change
        /// replaced) would have ACCEPTED the same padded proof — demonstrating exactly the hole
        /// the strict merged verification closes.
        #[test]
        fn test_shielded_withdrawal_padded_proof_is_rejected_by_strict_verify() {
            use drive::drive::shielded::paths::shielded_credit_pool_nullifiers_path_vec;
            use drive::drive::RootTree;
            use drive::grovedb::{GroveDb, PathQuery, Query, SizedQuery};
            use drive::query::{SingleDocumentDriveQuery, SingleDocumentDriveQueryContestedStatus};

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

            let output_script = create_output_script();
            let unshielding_amount = 499_995_000u64;
            let extra_sighash_data = dpp::shielded::shielded_withdrawal_extra_sighash_data_v0(
                output_script.as_bytes(),
                unshielding_amount,
                1,
                Pooling::Never,
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

            let transition = create_shielded_withdrawal_transition(
                actions,
                value_balance as u64,
                anchor_bytes,
                proof_bytes,
                binding_sig,
                1,
                Pooling::Never,
                output_script.clone(),
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
            // {nullifiers} ∪ {withdrawal-document}, each with cleared limits, then a limit that
            // can never truncate the legitimate result set.
            let StateTransition::ShieldedWithdrawal(ref st) = transition else {
                unreachable!();
            };
            let nullifier_keys: Vec<Vec<u8>> = st.nullifiers();

            // Compute withdrawal document ID deterministically (same as prove/verify sides).
            let first_nullifier = nullifier_keys
                .first()
                .expect("should have at least one nullifier");
            let mut entropy = Vec::new();
            entropy.extend_from_slice(first_nullifier);
            entropy.extend_from_slice(output_script.as_bytes());
            let document_id = Document::generate_document_id_v0(
                &withdrawals_contract::ID,
                &withdrawals_contract::OWNER_ID,
                withdrawal::NAME,
                &entropy,
            );

            let mut nf_query = Query::new();
            nf_query.insert_keys(nullifier_keys);
            let nullifier_pq = PathQuery::new(
                shielded_credit_pool_nullifiers_path_vec(),
                SizedQuery::new(nf_query, None, None),
            );

            let doc_query = SingleDocumentDriveQuery {
                contract_id: withdrawals_contract::ID.to_buffer(),
                document_type_name: withdrawal::NAME.to_string(),
                document_type_keeps_history: false,
                document_id: document_id.to_buffer(),
                block_time_ms: None,
                contested_status: SingleDocumentDriveQueryContestedStatus::NotContested,
            };
            let mut doc_pq = doc_query
                .construct_path_query(platform_version)
                .expect("construct doc path query");
            doc_pq.query.limit = None;

            let mut production_pq = PathQuery::merge(
                vec![&nullifier_pq, &doc_pq],
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

            // --- Build a SUPERSET (padded) proof: {nullifiers, document, + an extra subtree} ---
            // Pad by ALSO descending the genesis-populated `Pools` root subtree, which the
            // production query never touches; the padded proof carries an extra root-level layer.
            let mut pools_top = Query::new();
            pools_top.insert_key(vec![RootTree::Pools as u8]);
            pools_top.set_subquery(Query::new_range_full());
            let pools_pq = PathQuery::new(vec![], SizedQuery::new(pools_top, None, None));

            let mut superset_pq = PathQuery::merge(
                vec![&nullifier_pq, &doc_pq, &pools_pq],
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
            // primitive above would stay green if the ShieldedWithdrawal arm regressed to
            // rebuild a different merged query or fall back to the subset verifier; routing the
            // padded proof through `Drive::verify_state_transition_was_executed_with_proof`
            // locks the real code path into the test.
            let withdrawals_data_contract = Arc::new(
                load_system_data_contract(SystemDataContract::Withdrawals, platform_version)
                    .expect("should load withdrawals contract"),
            );
            let production_result = Drive::verify_state_transition_was_executed_with_proof(
                &transition,
                &BlockInfo::default(),
                &padded_proof,
                &|id| {
                    if *id == withdrawals_contract::ID {
                        Ok(Some(Arc::clone(&withdrawals_data_contract)))
                    } else {
                        Ok(None)
                    }
                },
                platform_version,
            )
            .map(|(root_hash, outcome)| (root_hash, outcome.into_result()));
            assert!(
                production_result.is_err(),
                "production shielded withdrawal verifier must reject a padded proof, got {:?}",
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

    mod credit_conservation {
        use super::*;
        use crate::execution::validation::state_transition::tests::process_state_transitions;
        use dpp::block::block_info::BlockInfo;
        use grovedb_commitment_tree::{
            Builder, BundleType, ClientMemoryCommitmentTree, DashMemo, ExtractedNoteCommitment,
            FullViewingKey, Note, NoteValue, Position, RandomSeed, Retention, Rho, Scope,
            SpendAuthorizingKey, SpendingKey,
        };
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        /// Block-level conservation test for shielded withdrawal. Spends a 500M note
        /// and withdraws `value_balance` worth from the pool. Runs the FULL block
        /// pipeline (execute + fee distribution + sum-tree validation) and asserts:
        ///   - total platform credits stay conserved (the invariant whose failure
        ///     halts the chain),
        ///   - the shielded pool drops by exactly `unshielding_amount`, and
        ///   - the system-credit counter drops by the NET (`unshielding_amount - fee`)
        ///     — only the net leaves the platform to Core; the fee stays in the fee
        ///     pools. This exercises the `RemoveFromSystemCredits(net)` accounting the
        ///     conversion-level tests cannot fully validate end-to-end.
        #[test]
        fn test_shielded_withdrawal_conserves_credits() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();
            insert_dummy_encrypted_notes(&platform, 250);
            // Seeded RNG for deterministic, bisectable test randomness (repo convention).
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

            let mut builder = Builder::<DashMemo>::new(BundleType::DEFAULT, anchor);
            builder.add_spend(fvk.clone(), note, merkle_path).unwrap();
            builder
                .add_output(None, recipient, NoteValue::from_raw(5_000), [0u8; 36])
                .unwrap();
            let (unauthorized, _) = builder.build::<i64>(&mut rng).unwrap().unwrap();

            let output_script = create_output_script();
            let unshielding_amount = 499_995_000u64;
            let extra_sighash_data = dpp::shielded::shielded_withdrawal_extra_sighash_data_v0(
                output_script.as_bytes(),
                unshielding_amount,
                1,
                Pooling::Never,
            );
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &extra_sighash_data);
            let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
            let bundle = proven.apply_signatures(rng, sighash, &[ask]).unwrap();

            let (actions, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                serialize_authorized_bundle_i64(&bundle);
            assert_eq!(value_balance, 499_995_000);
            let num_actions = actions.len();

            insert_anchor_into_state(&platform, &anchor_bytes);
            set_pool_total_balance(&platform, 500_000_000);

            // Capture a balanced starting state (set_pool_total_balance adds matching
            // system credits, so the counter and the sum trees agree).
            let credits_before = platform
                .drive
                .calculate_total_credits_balance(None, &platform_version.drive)
                .expect("should calculate total credits before withdrawal");
            assert!(
                credits_before
                    .ok()
                    .expect("credit balance check should not overflow"),
                "credits must be balanced before the withdrawal: {}",
                credits_before
            );

            let transition = create_shielded_withdrawal_transition(
                actions,
                unshielding_amount,
                anchor_bytes,
                proof_bytes,
                binding_sig,
                1,
                Pooling::Never,
                output_script,
            );

            let platform_state = platform.state.load();
            let (fee_results, _processed_block_fees) = process_state_transitions(
                &platform,
                &[transition],
                BlockInfo::default(),
                &platform_state,
            );

            // The shielded fee is split like every other transition: the (permanent) storage
            // cost is routed to `storage_fee` (storage pool, epoch fee multiplier applied at
            // payout), not booked entirely as processing. So a successful shielded withdrawal
            // reports a non-zero storage_fee and a non-zero processing fee that together equal
            // the carved minimum shielded fee.
            let fee = &fee_results[0];
            let expected_total =
                dpp::shielded::compute_shielded_withdrawal_fee(num_actions, platform_version)
                    .expect("fee computation should not overflow");
            assert!(
                fee.storage_fee > 0,
                "shielded storage must be charged as storage_fee, got {}",
                fee.storage_fee
            );
            assert!(
                fee.processing_fee > 0,
                "proof + processing must be charged as processing_fee, got {}",
                fee.processing_fee
            );
            assert_eq!(
                fee.storage_fee + fee.processing_fee,
                expected_total,
                "storage + processing must equal the carved minimum shielded fee"
            );

            let credits_after = platform
                .drive
                .calculate_total_credits_balance(None, &platform_version.drive)
                .expect("should calculate total credits after withdrawal");
            assert!(
                credits_after
                    .ok()
                    .expect("credit balance check should not overflow"),
                "credits must remain balanced after a shielded withdrawal: {}",
                credits_after
            );

            // The shielded pool dropped by exactly the full unshielding_amount.
            assert_eq!(
                credits_before.total_in_shielded_balances
                    - credits_after.total_in_shielded_balances,
                unshielding_amount as i64,
                "shielded pool must drop by the full unshielding amount"
            );

            // The system-credit counter dropped by the NET (unshielding_amount - fee):
            // only the net leaves the platform to Core; the fee stays in the fee pools.
            let fee = dpp::shielded::compute_shielded_withdrawal_fee(num_actions, platform_version)
                .expect("fee computation should not overflow");
            assert_eq!(
                credits_before.total_credits_in_platform - credits_after.total_credits_in_platform,
                unshielding_amount - fee,
                "system credits must drop by exactly the net withdrawn to Core (amount - fee)"
            );
        }
    }
}
