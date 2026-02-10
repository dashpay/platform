#[cfg(test)]
mod tests {
    use crate::config::{PlatformConfig, PlatformTestConfig};
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
            spend_auth_sig: vec![6u8; 64],
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
        binding_signature: Vec<u8>,
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
            vec![0u8; 64],  // dummy binding signature
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
                vec![0u8; 64],
                0,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::OverflowError(_))
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
                vec![0u8; 64],
                0,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::OverflowError(_))
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
                vec![0u8; 64],
                0,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::OverflowError(_))
                )]
            );
        }

        #[test]
        fn test_wrong_binding_sig_length_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let transition = create_shielded_transfer_transition(
                vec![create_dummy_serialized_action()],
                0x03,
                0,
                [42u8; 32],
                vec![0u8; 100],
                vec![0u8; 32], // 32 bytes instead of 64 — invalid
                0,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::OverflowError(_))
                )]
            );
        }

        #[test]
        fn test_wrong_spend_auth_sig_length_returns_error() {
            let platform_version = PlatformVersion::latest();
            let platform = setup_platform();

            let mut bad_action = create_dummy_serialized_action();
            bad_action.spend_auth_sig = vec![0u8; 32]; // 32 bytes instead of 64

            let transition = create_shielded_transfer_transition(
                vec![bad_action],
                0x03,
                0,
                [42u8; 32],
                vec![0u8; 100],
                vec![0u8; 64],
                0,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::OverflowError(_))
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
                vec![0u8; 64],
                0,
            );

            let processing_result = process_transition(&platform, transition, platform_version);

            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::UnpaidConsensusError(
                    ConsensusError::BasicError(BasicError::OverflowError(_))
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
                vec![0u8; 64],
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
}
