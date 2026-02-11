#[cfg(test)]
mod tests {
    use crate::config::{PlatformConfig, PlatformTestConfig};
    use crate::execution::validation::state_transition::state_transitions::shielded_common::compute_platform_sighash;
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use assert_matches::assert_matches;
    use dpp::block::block_info::BlockInfo;
    use dpp::consensus::basic::BasicError;
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;
    use dpp::serialization::PlatformSerializable;
    use dpp::shielded::SerializedAction;
    use dpp::state_transition::shielded_transfer_transition::v0::ShieldedTransferTransitionV0;
    use dpp::state_transition::shielded_transfer_transition::ShieldedTransferTransition;
    use dpp::state_transition::StateTransition;
    use drive::drive::shielded::paths::{
        shielded_anchors_credit_pool_path, shielded_credit_pool_nullifiers_path,
        shielded_credit_pool_path, SHIELDED_TOTAL_BALANCE_KEY,
    };
    use drive::grovedb::Element;
    use platform_version::version::PlatformVersion;

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
            spend_auth_sig: [6u8; 64],
        }
    }

    /// Builds a `ShieldedTransferTransition` state transition.
    /// No signing needed since shielded transfers have no witnesses.
    fn create_shielded_transfer_transition(
        actions: Vec<SerializedAction>,
        flags: u8,
        value_balance: i64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        user_fee_increase: u16,
    ) -> StateTransition {
        StateTransition::ShieldedTransfer(ShieldedTransferTransition::V0(
            ShieldedTransferTransitionV0 {
                actions,
                flags,
                value_balance,
                anchor,
                proof,
                binding_signature,
                user_fee_increase,
            },
        ))
    }

    /// Shorthand for creating a structurally valid (but cryptographically invalid) shielded
    /// transfer transition. Has a non-zero anchor, valid field sizes, but random data.
    fn create_default_shielded_transfer_transition() -> StateTransition {
        create_shielded_transfer_transition(
            vec![create_dummy_serialized_action()],
            0x03,           // spends_enabled | outputs_enabled
            0,              // zero fee
            [42u8; 32],     // non-zero anchor
            vec![0u8; 100], // dummy proof bytes
            [0u8; 64],  // dummy binding signature
            0,
        )
    }

    /// Insert a fake anchor into the shielded anchors tree via GroveDB.
    fn insert_anchor_into_state(
        platform: &crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        anchor: &[u8; 32],
    ) {
        let platform_version = PlatformVersion::latest();
        let grove_version = &platform_version.drive.grove_version;
        let transaction = platform.drive.grove.start_transaction();
        let anchors_path = shielded_anchors_credit_pool_path();

        platform
            .drive
            .grove
            .insert(
                &anchors_path,
                anchor,
                Element::Item(vec![], None),
                None,
                Some(&transaction),
                grove_version,
            )
            .unwrap()
            .expect("should insert anchor");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("should commit transaction");
    }

    /// Insert a nullifier into the nullifiers tree via GroveDB.
    fn insert_nullifier_into_state(
        platform: &crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        nullifier: &[u8; 32],
    ) {
        let platform_version = PlatformVersion::latest();
        let grove_version = &platform_version.drive.grove_version;
        let transaction = platform.drive.grove.start_transaction();
        let nullifiers_path = shielded_credit_pool_nullifiers_path();

        platform
            .drive
            .grove
            .insert(
                &nullifiers_path,
                nullifier,
                Element::Item(vec![], None),
                None,
                Some(&transaction),
                grove_version,
            )
            .unwrap()
            .expect("should insert nullifier");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("should commit transaction");
    }

    /// Set the shielded pool total balance in GroveDB.
    fn set_pool_total_balance(
        platform: &crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        balance: u64,
    ) {
        let platform_version = PlatformVersion::latest();
        let grove_version = &platform_version.drive.grove_version;
        let transaction = platform.drive.grove.start_transaction();
        let pool_path = shielded_credit_pool_path();

        platform
            .drive
            .grove
            .insert(
                &pool_path,
                &[SHIELDED_TOTAL_BALANCE_KEY],
                Element::new_sum_item(balance as i64),
                None,
                Some(&transaction),
                grove_version,
            )
            .unwrap()
            .expect("should set total balance");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("should commit transaction");
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
            let platform = setup_platform();

            let transition = create_shielded_transfer_transition(
                vec![], // Empty actions — invalid
                0x03,
                0,
                [42u8; 32],
                vec![0u8; 100],
                [0u8; 64],
                0,
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
        fn test_negative_value_balance_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let transition = create_shielded_transfer_transition(
                vec![create_dummy_serialized_action()],
                0x03,
                -1000, // Negative — invalid for shielded transfer (must be >= 0)
                [42u8; 32],
                vec![0u8; 100],
                [0u8; 64],
                0,
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
                0x03,
                0,
                [42u8; 32],
                vec![], // Empty proof — invalid
                [0u8; 64],
                0,
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
                0x03,
                0,
                [0u8; 32], // All zeros — invalid
                vec![0u8; 100],
                [0u8; 64],
                0,
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

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidAnchorError(_))
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

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::NullifierAlreadySpentError(_))
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
            CommitmentTree, ExtractedNoteCommitment, FullViewingKey, MerklePath, Note,
            NoteValue, Position, ProvingKey, Retention, Rho, Scope, SpendAuthorizingKey,
            SpendingKey, new_memory_store,
        };
        use orchard::note::RandomSeed;
        use rand::rngs::OsRng;
        use std::sync::OnceLock;

        static TEST_PROVING_KEY: OnceLock<ProvingKey> = OnceLock::new();
        fn get_proving_key() -> &'static ProvingKey {
            TEST_PROVING_KEY.get_or_init(ProvingKey::build)
        }

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
            let binding_sig =
                <[u8; 64]>::from(bundle.authorization().binding_signature());
            (actions, flags, value_balance, anchor, proof, binding_sig)
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

        #[test]
        fn test_valid_shielded_transfer_proof_succeeds() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();
            let mut rng = OsRng;
            let pk = get_proving_key();

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
            let note = Note::from_parts(recipient, NoteValue::from_raw(10_000), rho, rseed)
                .unwrap();

            // --- Build commitment tree and get anchor + merkle path ---
            let cmx = ExtractedNoteCommitment::from(note.commitment());
            let mut tree = CommitmentTree::new(new_memory_store(), 100);
            tree.append(cmx, Retention::Marked).unwrap();
            tree.checkpoint(0u32).unwrap();
            let anchor = tree.anchor().unwrap();
            let merkle_path = tree
                .orchard_witness(Position::from(0u64))
                .unwrap()
                .unwrap();

            // --- Build bundle: spend 10_000 → output 10_000 (value_balance = 0) ---
            let mut builder = Builder::new(BundleType::DEFAULT, anchor);
            builder
                .add_spend(fvk.clone(), note, merkle_path)
                .unwrap();
            builder
                .add_output(None, recipient, NoteValue::from_raw(10_000), [0u8; 512])
                .unwrap();

            let (unauthorized, _) = builder.build::<i64>(&mut rng).unwrap().unwrap();
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &[]);
            let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
            let bundle = proven.apply_signatures(rng, sighash, &[ask]).unwrap();

            // --- Extract serialized fields ---
            let (actions, flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                serialize_authorized_bundle(&bundle);

            // value_balance should be 0 (equal spend and output)
            assert_eq!(value_balance, 0);

            // --- Insert anchor into platform state ---
            insert_anchor_into_state(&platform, &anchor_bytes);

            // --- Create and process transition ---
            let transition = create_shielded_transfer_transition(
                actions,
                flags,
                value_balance,
                anchor_bytes,
                proof_bytes,
                binding_sig,
                0,
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
            bad_action.encrypted_note = vec![0u8; 100]; // 100 bytes instead of 692

            let transition = create_shielded_transfer_transition(
                vec![bad_action],
                0x03,
                0,
                anchor,
                vec![0u8; 100],
                [0u8; 64],
                0,
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
    // "AUDIT FINDING" comments and document the expected correct behavior.

    mod security_audit {
        use super::*;
        use grovedb_commitment_tree::{
            Anchor, Authorized as OrchardAuthorized, Builder, Bundle, BundleType,
            CommitmentTree, ExtractedNoteCommitment, FullViewingKey, MerklePath, Note,
            NoteValue, Position, ProvingKey, Retention, Rho, Scope, SpendAuthorizingKey,
            SpendingKey, new_memory_store,
        };
        use orchard::note::RandomSeed;
        use rand::rngs::OsRng;
        use std::sync::OnceLock;

        static TEST_PROVING_KEY: OnceLock<ProvingKey> = OnceLock::new();
        fn get_proving_key() -> &'static ProvingKey {
            TEST_PROVING_KEY.get_or_init(ProvingKey::build)
        }

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
            let binding_sig =
                <[u8; 64]>::from(bundle.authorization().binding_signature());
            (actions, flags, value_balance, anchor, proof, binding_sig)
        }

        /// Build a valid Orchard bundle for shielded transfer tests.
        /// Returns (actions, flags, value_balance, anchor_bytes, proof_bytes, binding_sig).
        fn build_valid_shielded_transfer_bundle() -> (Vec<SerializedAction>, u8, i64, [u8; 32], Vec<u8>, [u8; 64]) {
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
            let note = Note::from_parts(recipient, NoteValue::from_raw(10_000), rho, rseed)
                .unwrap();

            let cmx = ExtractedNoteCommitment::from(note.commitment());
            let mut tree = CommitmentTree::new(new_memory_store(), 100);
            tree.append(cmx, Retention::Marked).unwrap();
            tree.checkpoint(0u32).unwrap();
            let anchor = tree.anchor().unwrap();
            let merkle_path = tree
                .orchard_witness(Position::from(0u64))
                .unwrap()
                .unwrap();

            let mut builder = Builder::new(BundleType::DEFAULT, anchor);
            builder
                .add_spend(fvk.clone(), note, merkle_path)
                .unwrap();
            builder
                .add_output(None, recipient, NoteValue::from_raw(10_000), [0u8; 512])
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

            let (actions, flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                build_valid_shielded_transfer_bundle();
            assert_eq!(value_balance, 0);

            // ATTACK: Mutate value_balance from 0 to 5000
            let mutated_value_balance = 5000i64;

            // Set pool balance so fee deduction doesn't underflow
            set_pool_total_balance(&platform, 10_000);
            insert_anchor_into_state(&platform, &anchor_bytes);

            let transition = create_shielded_transfer_transition(
                actions,
                flags,
                mutated_value_balance, // MUTATED: was 0, now 5000
                anchor_bytes,
                proof_bytes,
                binding_sig,
                0,
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

            let (actions, flags, value_balance, anchor_bytes, proof_bytes, _binding_sig) =
                build_valid_shielded_transfer_bundle();

            insert_anchor_into_state(&platform, &anchor_bytes);

            let transition = create_shielded_transfer_transition(
                actions,
                flags,
                value_balance,
                anchor_bytes,
                proof_bytes,
                [0u8; 64], // ZEROED binding signature
                0,
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

            let (mut actions, flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                build_valid_shielded_transfer_bundle();

            // ATTACK: Zero out all spend auth signatures
            for action in &mut actions {
                action.spend_auth_sig = [0u8; 64];
            }

            insert_anchor_into_state(&platform, &anchor_bytes);

            let transition = create_shielded_transfer_transition(
                actions,
                flags,
                value_balance,
                anchor_bytes,
                proof_bytes,
                binding_sig,
                0,
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

        /// AUDIT FINDING: No intra-bundle duplicate nullifier check.
        ///
        /// The nullifier validation loop only checks the state database for
        /// each nullifier. It does not check for duplicates within the same
        /// bundle's action list. This is mitigated by the ZK proof (which
        /// can't produce duplicate nullifiers from a valid circuit), and by
        /// GroveDB's insert_only_op (which would fail on the second insert).
        ///
        /// Severity: LOW (defense-in-depth gap)
        /// For a fabricated bundle with duplicate nullifiers, the proof
        /// verification catches the invalid data.
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
                0x03,
                0,
                anchor,
                vec![0u8; 100],
                [0u8; 64],
                0,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            // The duplicate nullifiers are NOT caught by the application-level
            // nullifier check (which only checks state). They are caught by
            // proof verification (the fabricated data produces an invalid proof).
            // Ideally, intra-bundle nullifier dedup should be added as defense-in-depth.
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidShieldedProofError(_))
                )]
            );
        }
    }
}
