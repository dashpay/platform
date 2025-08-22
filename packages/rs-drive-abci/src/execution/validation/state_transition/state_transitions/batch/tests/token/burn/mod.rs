use super::*;

mod token_burn_tests {
    use super::*;
    use dpp::state_transition::batch_transition::TokenBurnTransition;

    #[test]
    fn test_token_burn() {
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

        let documents_batch_create_transition = BatchTransition::new_token_burn_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            1337,
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

        let documents_batch_create_serialized_transition = documents_batch_create_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[documents_batch_create_serialized_transition.clone()],
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
    }

    #[test]
    fn test_token_burn_trying_to_burn_more_than_we_have() {
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

        let documents_batch_create_transition = BatchTransition::new_token_burn_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            200000,
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

        let documents_batch_create_serialized_transition = documents_batch_create_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[documents_batch_create_serialized_transition.clone()],
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
        assert_eq!(token_balance, Some(100000)); // nothing was burned
    }

    #[test]
    fn test_token_burn_gives_error_if_trying_to_burn_from_not_allowed_identity() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(49853);

        let platform_state = platform.state.load();

        let (contract_owner_identity, _, _) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            contract_owner_identity.id(),
            None::<fn(&mut TokenConfiguration)>,
            None,
            None,
            None,
            platform_version,
        );

        let documents_batch_create_transition = BatchTransition::new_token_burn_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            1337,
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

        let documents_batch_create_serialized_transition = documents_batch_create_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[documents_batch_create_serialized_transition.clone()],
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
                ConsensusError::StateError(StateError::UnauthorizedTokenActionError(_)),
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
                contract_owner_identity.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        assert_eq!(token_balance, Some(100000));

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        assert_eq!(token_balance, None);
    }

    #[test]
    fn test_token_burn_group_action_tokens_transferred_before_completion() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(49853);
        let platform_state = platform.state.load();

        let (identity1, signer1, key1) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (identity2, signer2, key2) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (recipient, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity1.id(),
            Some(|token_configuration: &mut TokenConfiguration| {
                token_configuration.set_manual_burning_rules(ChangeControlRules::V0(
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
                        members: [(identity1.id(), 1), (identity2.id(), 1)].into(),
                        required_power: 2,
                    }),
                )]
                .into(),
            ),
            None,
            platform_version,
        );

        // Step 1: Mint tokens
        add_tokens_to_identity(&platform, token_id, identity1.id(), 100000);

        // Step 2: Initiate burn as proposer
        let burn_transition = BatchTransition::new_token_burn_transition(
            token_id,
            identity1.id(),
            contract.id(),
            0,
            100000,
            None,
            Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
            &key1,
            2,
            0,
            &signer1,
            platform_version,
            None,
        )
        .expect("expected to create burn transition");

        let token_burn_serialized_transition = burn_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[token_burn_serialized_transition.clone()],
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

        // Step 3: Transfer tokens away

        let token_transfer_transition = BatchTransition::new_token_transfer_transition(
            token_id,
            identity1.id(),
            contract.id(),
            0,
            1337,
            recipient.id(),
            None,
            None,
            None,
            &key1,
            3,
            0,
            &signer1,
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
                identity1.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        assert_eq!(token_balance, Some(198663));

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

        // Step 4: Confirm burn by second group member
        let action_id = TokenBurnTransition::calculate_action_id_with_fields(
            token_id.as_bytes(),
            identity1.id().as_bytes(),
            2,
            100000,
        );

        let confirm_burn_transition = BatchTransition::new_token_burn_transition(
            token_id,
            identity2.id(),
            contract.id(),
            0,
            100000,
            None,
            Some(
                GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
                    GroupStateTransitionInfo {
                        group_contract_position: 0,
                        action_id,
                        action_is_proposer: false,
                    },
                ),
            ),
            &key2,
            2,
            0,
            &signer2,
            platform_version,
            None,
        )
        .expect("expected to create confirmation transition");

        let token_burn_confirm_serialized_transition = confirm_burn_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[token_burn_confirm_serialized_transition.clone()],
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

        // Validate the burn still succeeded even though tokens were transferred
        let balance1 = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity1.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected balance fetch");

        let balance2 = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                recipient.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected balance fetch");

        assert_eq!(balance1, Some(98663)); // Original identity should have no tokens
        assert_eq!(balance2, Some(1337)); // Recipient should not keep transferred tokens if burn was enforced
    }

    #[test]
    fn test_token_burn_group_action_tokens_transferred_before_completion_not_enough_balance() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(49853);
        let platform_state = platform.state.load();

        let (identity1, signer1, key1) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (identity2, signer2, key2) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (recipient, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity1.id(),
            Some(|token_configuration: &mut TokenConfiguration| {
                token_configuration.set_manual_burning_rules(ChangeControlRules::V0(
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
                        members: [(identity1.id(), 1), (identity2.id(), 1)].into(),
                        required_power: 2,
                    }),
                )]
                .into(),
            ),
            None,
            platform_version,
        );

        // Step 1: Initiate burn as proposer
        let burn_transition = BatchTransition::new_token_burn_transition(
            token_id,
            identity1.id(),
            contract.id(),
            0,
            100000,
            None,
            Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
            &key1,
            2,
            0,
            &signer1,
            platform_version,
            None,
        )
        .expect("expected to create burn transition");

        let token_burn_serialized_transition = burn_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[token_burn_serialized_transition.clone()],
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

        // Step 2: Transfer tokens away

        let token_transfer_transition = BatchTransition::new_token_transfer_transition(
            token_id,
            identity1.id(),
            contract.id(),
            0,
            1337,
            recipient.id(),
            None,
            None,
            None,
            &key1,
            3,
            0,
            &signer1,
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
                identity1.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        assert_eq!(token_balance, Some(98663));

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

        // Step 3: Confirm burn by second group member
        let action_id = TokenBurnTransition::calculate_action_id_with_fields(
            token_id.as_bytes(),
            identity1.id().as_bytes(),
            2,
            100000,
        );

        let confirm_burn_transition = BatchTransition::new_token_burn_transition(
            token_id,
            identity2.id(),
            contract.id(),
            0,
            100000,
            None,
            Some(
                GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
                    GroupStateTransitionInfo {
                        group_contract_position: 0,
                        action_id,
                        action_is_proposer: false,
                    },
                ),
            ),
            &key2,
            2,
            0,
            &signer2,
            platform_version,
            None,
        )
        .expect("expected to create confirmation transition");

        let token_burn_confirm_serialized_transition = confirm_burn_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[token_burn_confirm_serialized_transition.clone()],
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

        // Validate the burn still succeeded even though tokens were transferred
        let balance1 = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity1.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected balance fetch");

        let balance2 = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                recipient.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected balance fetch");

        assert_eq!(balance1, Some(98663));
        assert_eq!(balance2, Some(1337));
    }
}
