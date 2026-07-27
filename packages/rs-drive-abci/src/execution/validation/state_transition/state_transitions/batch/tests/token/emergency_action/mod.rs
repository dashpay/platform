use super::*;

mod token_emergency_action_tests {
    use super::*;
    use dpp::tokens::emergency_action::TokenEmergencyAction;
    use dpp::tokens::status::v0::TokenStatusV0;
    use dpp::tokens::status::TokenStatus;

    #[tokio::test]
    async fn test_token_emergency_pause() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(49853);
        let platform_state = platform.state.load();

        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            Some(|token_configuration: &mut TokenConfiguration| {
                token_configuration.set_emergency_action_rules(ChangeControlRules::V0(
                    ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                        admin_action_takers: AuthorizedActionTakers::NoOne,
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_admin_action_takers_to_no_one_allowed: false,
                        self_changing_admin_action_takers_allowed: false,
                    },
                ));
            }),
            None,
            None,
            None,
            platform_version,
        );

        // Pause the token
        let pause_transition = BatchTransition::new_token_emergency_action_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            TokenEmergencyAction::Pause,
            None,
            None,
            &key,
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("expected to create emergency action transition");

        let serialized = pause_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[serialized],
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

        // Verify that the token is now paused
        let token_status = platform
            .drive
            .fetch_token_status(token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch token status");
        assert_eq!(
            token_status,
            Some(TokenStatus::V0(TokenStatusV0 { paused: true }))
        );
    }

    #[tokio::test]
    async fn test_token_emergency_resume() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(49853);
        let platform_state = platform.state.load();

        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            Some(|token_configuration: &mut TokenConfiguration| {
                token_configuration.set_emergency_action_rules(ChangeControlRules::V0(
                    ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                        admin_action_takers: AuthorizedActionTakers::NoOne,
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_admin_action_takers_to_no_one_allowed: false,
                        self_changing_admin_action_takers_allowed: false,
                    },
                ));
            }),
            None,
            None,
            None,
            platform_version,
        );

        // First pause the token
        let pause_transition = BatchTransition::new_token_emergency_action_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            TokenEmergencyAction::Pause,
            None,
            None,
            &key,
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("expected to create emergency action transition");

        let serialized = pause_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[serialized],
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

        // Verify the token is paused
        let token_status = platform
            .drive
            .fetch_token_status(token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch token status");
        assert_eq!(
            token_status,
            Some(TokenStatus::V0(TokenStatusV0 { paused: true }))
        );

        // Now resume the token
        let resume_transition = BatchTransition::new_token_emergency_action_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            TokenEmergencyAction::Resume,
            None,
            None,
            &key,
            3,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("expected to create emergency action transition");

        let serialized = resume_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[serialized],
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

        // Verify the token is now resumed (not paused)
        let token_status = platform
            .drive
            .fetch_token_status(token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch token status");
        assert_eq!(
            token_status,
            Some(TokenStatus::V0(TokenStatusV0 { paused: false }))
        );
    }

    #[tokio::test]
    async fn test_token_emergency_pause_already_paused_should_fail() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(49853);
        let platform_state = platform.state.load();

        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            Some(|token_configuration: &mut TokenConfiguration| {
                token_configuration.set_emergency_action_rules(ChangeControlRules::V0(
                    ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                        admin_action_takers: AuthorizedActionTakers::NoOne,
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_admin_action_takers_to_no_one_allowed: false,
                        self_changing_admin_action_takers_allowed: false,
                    },
                ));
            }),
            None,
            None,
            None,
            platform_version,
        );

        // First pause
        let pause_transition = BatchTransition::new_token_emergency_action_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            TokenEmergencyAction::Pause,
            None,
            None,
            &key,
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("expected to create emergency action transition");

        let serialized = pause_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[serialized],
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

        // Second pause should fail -- already paused
        let pause_again_transition = BatchTransition::new_token_emergency_action_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            TokenEmergencyAction::Pause,
            None,
            None,
            &key,
            3,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("expected to create emergency action transition");

        let serialized = pause_again_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[serialized],
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
            [PaidConsensusError {
                error: ConsensusError::StateError(StateError::TokenAlreadyPausedError(_)),
                ..
            }]
        );
    }

    #[tokio::test]
    async fn test_token_emergency_resume_not_paused_should_fail() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(49853);
        let platform_state = platform.state.load();

        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            Some(|token_configuration: &mut TokenConfiguration| {
                token_configuration.set_emergency_action_rules(ChangeControlRules::V0(
                    ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                        admin_action_takers: AuthorizedActionTakers::NoOne,
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_admin_action_takers_to_no_one_allowed: false,
                        self_changing_admin_action_takers_allowed: false,
                    },
                ));
            }),
            None,
            None,
            None,
            platform_version,
        );

        // First pause so that the token status record is created
        let pause_transition = BatchTransition::new_token_emergency_action_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            TokenEmergencyAction::Pause,
            None,
            None,
            &key,
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("expected to create emergency action transition");

        let serialized = pause_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[serialized],
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

        // Then resume so the status is "not paused"
        let resume_transition = BatchTransition::new_token_emergency_action_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            TokenEmergencyAction::Resume,
            None,
            None,
            &key,
            3,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("expected to create emergency action transition");

        let serialized = resume_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[serialized],
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

        // Now try to resume again - should fail because status exists and is not paused
        let resume_again_transition = BatchTransition::new_token_emergency_action_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            TokenEmergencyAction::Resume,
            None,
            None,
            &key,
            4,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("expected to create emergency action transition");

        let serialized = resume_again_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[serialized],
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
            [PaidConsensusError {
                error: ConsensusError::StateError(StateError::TokenNotPausedError(_)),
                ..
            }]
        );
    }
}
