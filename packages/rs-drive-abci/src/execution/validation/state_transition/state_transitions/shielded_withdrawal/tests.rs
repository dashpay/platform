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
    use dpp::identity::core_script::CoreScript;
    use dpp::serialization::PlatformSerializable;
    use dpp::shielded::SerializedAction;
    use dpp::state_transition::shielded_withdrawal_transition::v0::ShieldedWithdrawalTransitionV0;
    use dpp::state_transition::shielded_withdrawal_transition::ShieldedWithdrawalTransition;
    use dpp::state_transition::StateTransition;
    use dpp::withdrawal::Pooling;
    use drive::drive::shielded::paths::{
        shielded_anchors_credit_pool_path, shielded_credit_pool_encrypted_notes_path,
        shielded_credit_pool_nullifiers_path, shielded_credit_pool_path,
        SHIELDED_TOTAL_BALANCE_KEY,
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

    /// Create a dummy CoreScript (P2PKH) for the withdrawal output.
    fn create_output_script() -> CoreScript {
        CoreScript::new_p2pkh([7u8; 20])
    }

    /// Builds a `ShieldedWithdrawalTransition` state transition.
    /// No signing needed since shielded withdrawal transitions have no ECDSA witnesses
    /// (authenticated purely via Orchard ZK proof + signatures).
    fn create_shielded_withdrawal_transition(
        amount: u64,
        actions: Vec<SerializedAction>,
        flags: u8,
        value_balance: i64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        core_fee_per_byte: u32,
        pooling: Pooling,
        output_script: CoreScript,
        user_fee_increase: u16,
    ) -> StateTransition {
        StateTransition::ShieldedWithdrawal(ShieldedWithdrawalTransition::V0(
            ShieldedWithdrawalTransitionV0 {
                amount,
                actions,
                flags,
                value_balance,
                anchor,
                proof,
                binding_signature,
                core_fee_per_byte,
                pooling,
                output_script,
                user_fee_increase,
            },
        ))
    }

    /// Shorthand for creating a structurally valid (but cryptographically invalid) shielded
    /// withdrawal transition. Has a non-zero anchor, valid field sizes, positive amount and
    /// value_balance.
    fn create_default_shielded_withdrawal_transition() -> StateTransition {
        create_shielded_withdrawal_transition(
            1000, // amount in credits
            vec![create_dummy_serialized_action()],
            0x03,                   // spends_enabled | outputs_enabled
            1000,                   // value_balance = amount (no fee for simplicity)
            [42u8; 32],             // non-zero anchor
            vec![0u8; 100],         // dummy proof bytes
            [0u8; 64],              // dummy binding signature
            1,                      // core_fee_per_byte
            Pooling::Never,         // pooling strategy
            create_output_script(), // P2PKH output script
            0,                      // user_fee_increase
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

    /// Insert dummy encrypted notes into the shielded pool to meet the minimum
    /// notes threshold for outgoing transitions.
    fn insert_dummy_encrypted_notes(
        platform: &crate::test::helpers::setup::TempPlatform<crate::rpc::core::MockCoreRPCLike>,
        count: u64,
    ) {
        let platform_version = PlatformVersion::latest();
        let grove_version = &platform_version.drive.grove_version;
        let transaction = platform.drive.grove.start_transaction();
        let notes_path = shielded_credit_pool_encrypted_notes_path();

        for i in 0..count {
            platform
                .drive
                .grove
                .insert(
                    &notes_path,
                    &i.to_be_bytes(),
                    Element::Item(vec![0], None),
                    None,
                    Some(&transaction),
                    grove_version,
                )
                .unwrap()
                .expect("should insert dummy note");
        }

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

            let transition = create_shielded_withdrawal_transition(
                1000,
                vec![], // Empty actions — invalid
                0x03,
                1000,
                [42u8; 32],
                vec![0u8; 100],
                [0u8; 64],
                1,
                Pooling::Never,
                create_output_script(),
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
        fn test_zero_amount_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let transition = create_shielded_withdrawal_transition(
                0, // Zero amount — invalid
                vec![create_dummy_serialized_action()],
                0x03,
                1000,
                [42u8; 32],
                vec![0u8; 100],
                [0u8; 64],
                1,
                Pooling::Never,
                create_output_script(),
                0,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::UnshieldAmountZeroError(_))
                )]
            );
        }

        #[test]
        fn test_zero_value_balance_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let transition = create_shielded_withdrawal_transition(
                1000,
                vec![create_dummy_serialized_action()],
                0x03,
                0, // Zero value_balance — invalid (must be positive for withdrawal)
                [42u8; 32],
                vec![0u8; 100],
                [0u8; 64],
                1,
                Pooling::Never,
                create_output_script(),
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
        fn test_negative_value_balance_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let transition = create_shielded_withdrawal_transition(
                1000,
                vec![create_dummy_serialized_action()],
                0x03,
                -1000, // Negative value_balance — invalid (must be positive for withdrawal)
                [42u8; 32],
                vec![0u8; 100],
                [0u8; 64],
                1,
                Pooling::Never,
                create_output_script(),
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
        fn test_value_balance_less_than_amount_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let transition = create_shielded_withdrawal_transition(
                2000, // amount = 2000
                vec![create_dummy_serialized_action()],
                0x03,
                1000, // value_balance = 1000 < amount — invalid
                [42u8; 32],
                vec![0u8; 100],
                [0u8; 64],
                1,
                Pooling::Never,
                create_output_script(),
                0,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::UnshieldValueBalanceBelowAmountError(_))
                )]
            );
        }

        #[test]
        fn test_empty_proof_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let transition = create_shielded_withdrawal_transition(
                1000,
                vec![create_dummy_serialized_action()],
                0x03,
                1000,
                [42u8; 32],
                vec![], // Empty proof — invalid
                [0u8; 64],
                1,
                Pooling::Never,
                create_output_script(),
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

            let transition = create_shielded_withdrawal_transition(
                1000,
                vec![create_dummy_serialized_action()],
                0x03,
                1000,
                [0u8; 32], // All zeros — invalid
                vec![0u8; 100],
                [0u8; 64],
                1,
                Pooling::Never,
                create_output_script(),
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

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InsufficientPoolNotesError(_))
                )]
            );
        }

        #[test]
        fn test_anchor_not_in_tree_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();
            insert_dummy_encrypted_notes(&platform, 250);

            // Non-zero anchor that doesn't exist in state
            let transition = create_default_shielded_withdrawal_transition();

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
            new_memory_store, Authorized as OrchardAuthorized, Builder, Bundle, BundleType,
            CommitmentTree, ExtractedNoteCommitment, FullViewingKey, Note, NoteValue, Position,
            ProvingKey, Retention, Rho, Scope, SpendAuthorizingKey, SpendingKey,
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
            let binding_sig = <[u8; 64]>::from(bundle.authorization().binding_signature());
            (actions, flags, value_balance, anchor, proof, binding_sig)
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

            // --- Create a spendable note with value 10,000 ---
            let rho_bytes: [u8; 32] = {
                let mut b = [0u8; 32];
                b[0] = 1;
                b
            };
            let rho = Rho::from_bytes(&rho_bytes).unwrap();
            let rseed = RandomSeed::from_bytes([42u8; 32], &rho).unwrap();
            let note =
                Note::from_parts(recipient, NoteValue::from_raw(10_000), rho, rseed).unwrap();

            // --- Build commitment tree and get anchor + merkle path ---
            let cmx = ExtractedNoteCommitment::from(note.commitment());
            let mut tree = CommitmentTree::new(new_memory_store(), 100);
            tree.append(cmx, Retention::Marked).unwrap();
            tree.checkpoint(0u32).unwrap();
            let anchor = tree.anchor().unwrap();
            let merkle_path = tree.orchard_witness(Position::from(0u64)).unwrap().unwrap();

            // --- Build bundle: spend 10,000 -> output 5,000 (value_balance = 5,000) ---
            let mut builder = Builder::new(BundleType::DEFAULT, anchor);
            builder.add_spend(fvk.clone(), note, merkle_path).unwrap();
            builder
                .add_output(None, recipient, NoteValue::from_raw(5_000), [0u8; 512])
                .unwrap();

            let (unauthorized, _) = builder.build::<i64>(&mut rng).unwrap().unwrap();

            // Compute platform sighash binding transparent fields (output_script, amount)
            let output_script = create_output_script();
            let amount = 5_000u64; // = value_balance (no fee difference)
            let mut extra_sighash_data = output_script.as_bytes().to_vec();
            extra_sighash_data.extend_from_slice(&amount.to_le_bytes());
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &extra_sighash_data);

            let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
            let bundle = proven.apply_signatures(rng, sighash, &[ask]).unwrap();

            // --- Extract serialized fields ---
            let (actions, flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                serialize_authorized_bundle(&bundle);

            // value_balance should be 5000 (10,000 spent - 5,000 output)
            assert_eq!(value_balance, 5_000);

            // --- Set up platform state ---
            // Insert anchor so anchor validation passes
            insert_anchor_into_state(&platform, &anchor_bytes);

            // Set pool total balance so the withdrawal has sufficient funds
            set_pool_total_balance(&platform, 10_000);

            // --- Create and process transition ---
            let transition = create_shielded_withdrawal_transition(
                amount, // amount = 5000 credits
                actions,
                flags,
                value_balance,
                anchor_bytes,
                proof_bytes,
                binding_sig,
                1,              // core_fee_per_byte
                Pooling::Never, // pooling strategy
                output_script,
                0, // user_fee_increase
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
            bad_action.encrypted_note = vec![0u8; 100]; // 100 bytes instead of 692

            let transition = create_shielded_withdrawal_transition(
                1000,
                vec![bad_action],
                0x03,
                1000,
                anchor,
                vec![0u8; 100],
                [0u8; 64],
                1,
                Pooling::Never,
                create_output_script(),
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
    // "AUDIT FINDING" / "AUDIT REGRESSION" comments.

    mod security_audit {
        use super::*;
        use grovedb_commitment_tree::{
            new_memory_store, Authorized as OrchardAuthorized, Builder, Bundle, BundleType,
            CommitmentTree, ExtractedNoteCommitment, FullViewingKey, Note, NoteValue, Position,
            ProvingKey, Retention, Rho, Scope, SpendAuthorizingKey, SpendingKey,
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
            let binding_sig = <[u8; 64]>::from(bundle.authorization().binding_signature());
            (actions, flags, value_balance, anchor, proof, binding_sig)
        }

        /// Build a valid Orchard bundle for shielded withdrawal tests (spend > output).
        /// The `output_script` and `amount` are bound to the sighash so that
        /// the resulting bundle can only be used with those specific transparent fields.
        /// Returns (actions, flags, value_balance, anchor_bytes, proof_bytes, binding_sig).
        fn build_valid_shielded_withdrawal_bundle(
            output_script: &CoreScript,
            amount: u64,
        ) -> (Vec<SerializedAction>, u8, i64, [u8; 32], Vec<u8>, [u8; 64]) {
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
                Note::from_parts(recipient, NoteValue::from_raw(10_000), rho, rseed).unwrap();

            let cmx = ExtractedNoteCommitment::from(note.commitment());
            let mut tree = CommitmentTree::new(new_memory_store(), 100);
            tree.append(cmx, Retention::Marked).unwrap();
            tree.checkpoint(0u32).unwrap();
            let anchor = tree.anchor().unwrap();
            let merkle_path = tree.orchard_witness(Position::from(0u64)).unwrap().unwrap();

            // Spend 10,000 -> output 5,000 -> value_balance = 5,000
            let mut builder = Builder::new(BundleType::DEFAULT, anchor);
            builder.add_spend(fvk.clone(), note, merkle_path).unwrap();
            builder
                .add_output(None, recipient, NoteValue::from_raw(5_000), [0u8; 512])
                .unwrap();

            let (unauthorized, _) = builder.build::<i64>(&mut rng).unwrap().unwrap();

            // Bind transparent fields (output_script, amount) to the sighash
            let mut extra_sighash_data = output_script.as_bytes().to_vec();
            extra_sighash_data.extend_from_slice(&amount.to_le_bytes());
            let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
            let sighash = compute_platform_sighash(&bundle_commitment, &extra_sighash_data);

            let proven = unauthorized.create_proof(pk, &mut rng).unwrap();
            let bundle = proven.apply_signatures(rng, sighash, &[ask]).unwrap();

            serialize_authorized_bundle(&bundle)
        }

        /// Edge case: i64::MIN value_balance should be caught by structure validation
        /// (value_balance must be positive). This ensures no integer overflow or
        /// underflow issues occur when handling the most extreme negative i64 value.
        #[test]
        fn test_i64_min_value_balance_handled() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let transition = create_shielded_withdrawal_transition(
                1000,
                vec![create_dummy_serialized_action()],
                0x03,
                i64::MIN, // Most extreme negative value — must be rejected
                [42u8; 32],
                vec![0u8; 100],
                [0u8; 64],
                1,
                Pooling::Never,
                create_output_script(),
                0,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            // i64::MIN is negative, so structure validation rejects it as
            // "shielded withdrawal value_balance must be positive"
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::ShieldedInvalidValueBalanceError(_))
                )]
            );
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
            let amount = 5_000u64;
            let (actions, flags, value_balance, anchor_bytes, proof_bytes, _binding_sig) =
                build_valid_shielded_withdrawal_bundle(&output_script, amount);

            set_pool_total_balance(&platform, 20_000);
            insert_anchor_into_state(&platform, &anchor_bytes);

            let transition = create_shielded_withdrawal_transition(
                amount,
                actions,
                flags,
                value_balance,
                anchor_bytes,
                proof_bytes,
                [0u8; 64], // ZEROED binding signature
                1,
                Pooling::Never,
                output_script,
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

            // Bundle is signed for create_output_script() with amount = 5000
            let output_script = create_output_script();
            let signed_amount = 5_000u64;
            let (actions, flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                build_valid_shielded_withdrawal_bundle(&output_script, signed_amount);
            assert_eq!(value_balance, 5_000);

            // ATTACK: Inflate value_balance from 5000 to 9000
            let mutated_value_balance = 9_000i64;

            // Set pool balance high enough for the inflated amount
            set_pool_total_balance(&platform, 20_000);
            insert_anchor_into_state(&platform, &anchor_bytes);

            let transition = create_shielded_withdrawal_transition(
                mutated_value_balance as u64, // amount = 9000 (inflated)
                actions,
                flags,
                mutated_value_balance, // MUTATED: was 5000, now 9000
                anchor_bytes,
                proof_bytes,
                binding_sig,
                1,
                Pooling::Never,
                output_script,
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

            // Bundle is signed for the ORIGINAL output_script with amount = 5000
            let original_script = create_output_script();
            let amount = 5_000u64;
            let (actions, flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                build_valid_shielded_withdrawal_bundle(&original_script, amount);
            assert_eq!(value_balance, 5_000);

            set_pool_total_balance(&platform, 20_000);
            insert_anchor_into_state(&platform, &anchor_bytes);

            // ATTACK: Use a completely different output script (attacker's address)
            let attacker_script = CoreScript::new_p2pkh([0xAA; 20]);

            let transition = create_shielded_withdrawal_transition(
                amount,
                actions,
                flags,
                value_balance,
                anchor_bytes,
                proof_bytes,
                binding_sig,
                1,
                Pooling::Never,
                attacker_script, // ATTACKER's script, not the original
                0,
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

        /// AUDIT REGRESSION: Different amount is caught by platform sighash.
        ///
        /// The amount is bound to the Orchard bundle via sighash. Changing the
        /// withdrawal amount after signing causes the sighash to differ, and
        /// signature verification fails. This prevents an attacker from inflating
        /// the credited withdrawal amount while keeping a valid value_balance.
        #[test]
        fn test_different_amount_with_same_valid_bundle_is_rejected() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();
            insert_dummy_encrypted_notes(&platform, 250);

            let output_script = create_output_script();
            let signed_amount = 5_000u64;
            let (actions, flags, value_balance, anchor_bytes, proof_bytes, binding_sig) =
                build_valid_shielded_withdrawal_bundle(&output_script, signed_amount);
            assert_eq!(value_balance, 5_000);

            set_pool_total_balance(&platform, 20_000);
            insert_anchor_into_state(&platform, &anchor_bytes);

            // ATTACK: Use a smaller amount (4000) but same value_balance (5000)
            // to pocket the difference as "fee" that goes nowhere
            let manipulated_amount = 4_000u64;

            let transition = create_shielded_withdrawal_transition(
                manipulated_amount, // MANIPULATED: was 5000, now 4000
                actions,
                flags,
                value_balance, // still 5000 — passes value_balance >= amount check
                anchor_bytes,
                proof_bytes,
                binding_sig,
                1,
                Pooling::Never,
                output_script,
                0,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            // Platform sighash includes amount, so changing it causes
            // signature verification to fail.
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidShieldedProofError(_))
                )]
            );
        }

        /// AUDIT FINDING: No intra-bundle duplicate nullifier check.
        ///
        /// Same as the shielded_transfer and unshield findings. The nullifier
        /// check only queries state, not checking for duplicates within the
        /// bundle itself. Mitigated by ZK proof verification (fabricated data
        /// with duplicate nullifiers produces an invalid proof).
        ///
        /// Severity: LOW (defense-in-depth gap, caught by ZK proof verification)
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
                1000,
                vec![action1, action2], // Both have nullifier [1u8; 32]
                0x03,
                1000,
                anchor,
                vec![0u8; 100],
                [0u8; 64],
                1,
                Pooling::Never,
                create_output_script(),
                0,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            // Caught by proof verification, not by application-level dedup
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::StateError(StateError::InvalidShieldedProofError(_))
                )]
            );
        }
    }
}
