use super::*;
mod token_transfer_tests {
    use dpp::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
    use dpp::data_contract::change_control_rules::ChangeControlRules;
    use dpp::data_contract::change_control_rules::v0::ChangeControlRulesV0;
    use dpp::data_contract::group::Group;
    use dpp::state_transition::batch_transition::TokenMintTransition;
    use dpp::data_contract::group::v0::GroupV0;
    use dpp::group::{GroupStateTransitionInfo, GroupStateTransitionInfoStatus};
    use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
    use dpp::state_transition::batch_transition::batched_transition::token_transition::TokenTransition;
    use dpp::state_transition::{GetDataContractSecurityLevelRequirementFn, StateTransition};
    use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionMutRef;
    use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
    use dpp::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
    use dpp::tokens::emergency_action::TokenEmergencyAction;
    use dpp::tokens::status::TokenStatus;
    use dpp::tokens::status::v0::TokenStatusV0;
    use super::*;

    #[test]
    fn test_token_transfer() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(49853);

        let platform_state = platform.state.load();

        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (recipient, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            None::<fn(&mut TokenConfiguration)>,
            None,
            None,
            None,
            platform_version,
        );

        let token_transfer_transition = BatchTransition::new_token_transfer_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            1337,
            recipient.id(),
            None,
            None,
            None,
            &key,
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .expect("expect to create documents batch transition");

        let token_transfer_serialized_transition = token_transfer_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[token_transfer_serialized_transition.clone()],
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
            [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        let expected_amount = 100000 - 1337;
        assert_eq!(token_balance, Some(expected_amount));

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                recipient.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        let expected_amount = 1337;
        assert_eq!(token_balance, Some(expected_amount));
    }

    #[test]
    fn test_token_transfer_to_non_existing_identity() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(49853);

        let platform_state = platform.state.load();

        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let recipient_id: Identifier = rng.gen();
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            None::<fn(&mut TokenConfiguration)>,
            None,
            None,
            None,
            platform_version,
        );

        let token_transfer_transition = BatchTransition::new_token_transfer_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            1337,
            recipient_id,
            None,
            None,
            None,
            &key,
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .expect("expect to create documents batch transition");

        let token_transfer_serialized_transition = token_transfer_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[token_transfer_serialized_transition.clone()],
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
            [PaidConsensusError(
                ConsensusError::StateError(
                    StateError::TokenTransferRecipientIdentityNotExistError(_)
                ),
                _
            )]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        assert_eq!(token_balance, Some(100000));

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                recipient_id.to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        assert_eq!(token_balance, None);
    }

    #[test]
    fn test_token_transfer_should_fail_if_token_started_paused() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(49853);

        let platform_state = platform.state.load();

        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (recipient, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            Some(|token_configuration: &mut TokenConfiguration| {
                token_configuration.set_start_as_paused(true);
                token_configuration.set_emergency_action_rules(ChangeControlRules::V0(
                    ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                        admin_action_takers: AuthorizedActionTakers::ContractOwner,
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

        let token_transfer_transition = BatchTransition::new_token_transfer_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            1337,
            recipient.id(),
            None,
            None,
            None,
            &key,
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .expect("expect to create documents batch transition");

        let token_transfer_serialized_transition = token_transfer_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[token_transfer_serialized_transition.clone()],
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
            [PaidConsensusError(
                ConsensusError::StateError(StateError::TokenIsPausedError(_)),
                _
            )]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        assert_eq!(token_balance, Some(100000));

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                recipient.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        assert_eq!(token_balance, None);

        let token_status = platform
            .drive
            .fetch_token_status(token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch token status");
        assert_eq!(
            token_status,
            Some(TokenStatus::V0(TokenStatusV0 { paused: true }))
        );

        // now let's let the token be transferable with an emergency action

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
        .expect("expect to create documents batch transition");

        let resume_transition_transition = resume_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[resume_transition_transition.clone()],
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
            [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let token_status = platform
            .drive
            .fetch_token_status(token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch token status");
        assert_eq!(
            token_status,
            Some(TokenStatus::V0(TokenStatusV0 { paused: false }))
        );

        // the transfer should now work

        let token_transfer_transition = BatchTransition::new_token_transfer_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            1337,
            recipient.id(),
            None,
            None,
            None,
            &key,
            4,
            0,
            &signer,
            platform_version,
            None,
        )
        .expect("expect to create documents batch transition");

        let token_transfer_serialized_transition = token_transfer_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[token_transfer_serialized_transition.clone()],
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
            [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        let expected_amount = 100000 - 1337;
        assert_eq!(token_balance, Some(expected_amount));

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                recipient.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        let expected_amount = 1337;
        assert_eq!(token_balance, Some(expected_amount));
    }

    #[test]
    fn test_token_transfer_should_fail_if_token_becomes_paused() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(49853);

        let platform_state = platform.state.load();

        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (recipient, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            Some(|token_configuration: &mut TokenConfiguration| {
                token_configuration.set_start_as_paused(false);
                token_configuration.set_emergency_action_rules(ChangeControlRules::V0(
                    ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                        admin_action_takers: AuthorizedActionTakers::ContractOwner,
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

        let resume_transition = BatchTransition::new_token_emergency_action_transition(
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
        .expect("expect to create documents batch transition");

        let resume_transition_transition = resume_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[resume_transition_transition.clone()],
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
            [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let token_status = platform
            .drive
            .fetch_token_status(token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch token status");
        assert_eq!(
            token_status,
            Some(TokenStatus::V0(TokenStatusV0 { paused: true }))
        );

        let token_transfer_transition = BatchTransition::new_token_transfer_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            1337,
            recipient.id(),
            None,
            None,
            None,
            &key,
            3,
            0,
            &signer,
            platform_version,
            None,
        )
        .expect("expect to create documents batch transition");

        let token_transfer_serialized_transition = token_transfer_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[token_transfer_serialized_transition.clone()],
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
            [PaidConsensusError(
                ConsensusError::StateError(StateError::TokenIsPausedError(_)),
                _
            )]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        assert_eq!(token_balance, Some(100000));

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                recipient.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        assert_eq!(token_balance, None);

        // now let's make the token be transferable again

        let resume_transition = BatchTransition::new_token_emergency_action_transition(
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
        .expect("expect to create documents batch transition");

        let resume_transition_transition = resume_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[resume_transition_transition.clone()],
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
            [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let token_status = platform
            .drive
            .fetch_token_status(token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch token status");
        assert_eq!(
            token_status,
            Some(TokenStatus::V0(TokenStatusV0 { paused: false }))
        );

        // the transfer should now work

        let token_transfer_transition = BatchTransition::new_token_transfer_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            1337,
            recipient.id(),
            None,
            None,
            None,
            &key,
            5,
            0,
            &signer,
            platform_version,
            None,
        )
        .expect("expect to create documents batch transition");

        let token_transfer_serialized_transition = token_transfer_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[token_transfer_serialized_transition.clone()],
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
            [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        let expected_amount = 100000 - 1337;
        assert_eq!(token_balance, Some(expected_amount));

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                recipient.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        let expected_amount = 1337;
        assert_eq!(token_balance, Some(expected_amount));
    }

    #[test]
    fn test_token_transfer_to_ourself_should_fail() {
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
            None::<fn(&mut TokenConfiguration)>,
            None,
            None,
            None,
            platform_version,
        );

        let token_transfer_transition = BatchTransition::new_token_transfer_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            1337,
            identity.id(),
            None,
            None,
            None,
            &key,
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .expect("expect to create documents batch transition");

        let token_transfer_serialized_transition = token_transfer_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[token_transfer_serialized_transition.clone()],
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
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::BasicError(BasicError::TokenTransferToOurselfError(_))
            )]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        assert_eq!(token_balance, Some(100000));
    }

    #[test]
    fn test_token_transfer_trying_to_send_more_than_we_have() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(49853);

        let platform_state = platform.state.load();

        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (recipient, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            None::<fn(&mut TokenConfiguration)>,
            None,
            None,
            None,
            platform_version,
        );

        let token_transfer_transition = BatchTransition::new_token_transfer_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            200000,
            recipient.id(),
            None,
            None,
            None,
            &key,
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .expect("expect to create documents batch transition");

        let token_transfer_serialized_transition = token_transfer_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[token_transfer_serialized_transition.clone()],
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
            [PaidConsensusError(
                ConsensusError::StateError(StateError::IdentityDoesNotHaveEnoughTokenBalanceError(
                    _
                )),
                _
            )]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        let expected_amount = 100000;
        assert_eq!(token_balance, Some(expected_amount));

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                recipient.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        assert_eq!(token_balance, None);
    }

    #[test]
    fn test_token_transfer_adding_group_info_causes_error() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(49853);

        let platform_state = platform.state.load();

        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (recipient, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        // let's start by creating a real action

        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
            Some(|token_configuration: &mut TokenConfiguration| {
                token_configuration.set_manual_minting_rules(ChangeControlRules::V0(
                    ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::Group(0),
                        admin_action_takers: AuthorizedActionTakers::NoOne,
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_admin_action_takers_to_no_one_allowed: false,
                        self_changing_admin_action_takers_allowed: false,
                    },
                ));
            }),
            None,
            Some(
                [(
                    0,
                    Group::V0(GroupV0 {
                        members: [(identity.id(), 1), (recipient.id(), 1)].into(),
                        required_power: 2,
                    }),
                )]
                .into(),
            ),
            None,
            platform_version,
        );

        let token_mint_transition = BatchTransition::new_token_mint_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            1337,
            Some(identity.id()),
            None,
            Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
            &key,
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .expect("expect to create documents batch transition");

        let token_mint_serialized_transition = token_mint_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[token_mint_serialized_transition.clone()],
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
            [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let action_id = TokenMintTransition::calculate_action_id_with_fields(
            token_id.as_bytes(),
            identity.id().as_bytes(),
            2,
            1337,
        );

        let mut token_transfer_transition = BatchTransition::new_token_transfer_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            200000,
            recipient.id(),
            None,
            None,
            None,
            &key,
            3,
            0,
            &signer,
            platform_version,
            None,
        )
        .expect("expect to create documents batch transition");

        // here we add fake info
        if let StateTransition::Batch(batch) = &mut token_transfer_transition {
            let first_transition = batch
                .first_transition_mut()
                .expect("expected_first_transition");
            if let BatchedTransitionMutRef::Token(TokenTransition::Transfer(transfer)) =
                first_transition
            {
                transfer
                    .base_mut()
                    .set_using_group_info(Some(GroupStateTransitionInfo {
                        group_contract_position: 0,
                        action_id,
                        action_is_proposer: true,
                    }))
            }
        }

        token_transfer_transition
            .sign_external(
                &key,
                &signer,
                None::<GetDataContractSecurityLevelRequirementFn>,
            )
            .expect("expected to resign transaction");

        let token_transfer_serialized_transition = token_transfer_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[token_transfer_serialized_transition.clone()],
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
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::BasicError(BasicError::GroupActionNotAllowedOnTransitionError(_))
            )]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        let expected_amount = 100000;
        assert_eq!(token_balance, Some(expected_amount));

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                recipient.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        assert_eq!(token_balance, None);
    }
}
