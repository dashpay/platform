use super::*;

mod token_burn_tests {
    use super::*;
    use crate::execution::validation::state_transition::tests::add_tokens_to_identity;
    use dpp::state_transition::batch_transition::TokenBurnTransition;
    use dpp::tokens::MAX_TOKEN_NOTE_LEN;

    #[tokio::test]
    async fn test_token_burn() {
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
        .await
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
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
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

    #[tokio::test]
    async fn test_token_burn_entire_balance() {
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

        // Burn the entire balance of 100000 tokens
        let burn_transition = BatchTransition::new_token_burn_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            100000,
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
        .expect("expect to create documents batch transition");

        let burn_serialized_transition = burn_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[burn_serialized_transition.clone()],
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

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        assert_eq!(token_balance, Some(0));
    }

    /// Pins the *tokens-always-pay* invariant for the
    /// [`ConsensusValidationResult::merge_many`] aggregator change
    /// (issue #2867): an all-failed single-token-transition batch must
    /// continue to land as `PaidConsensusError` on every protocol
    /// version, because the token sub-transformer
    /// (`try_from_borrowed_token_burn_transition_with_contract_lookup`)
    /// emits a `BumpIdentityDataContractNonce` action on
    /// base-validation failure, so each per-token result has
    /// `data: Some([bump])` and the v1 aggregator never collapses to
    /// `data: None`.
    ///
    /// If a future change drops the bump from the token sub-transformer,
    /// the v1 aggregator would route the failure to
    /// `UnpaidConsensusError` and the tx would be removed from the block
    /// by `prepare_proposal` — different state-root, different
    /// replay-protection behavior than every prior chain. The paired
    /// `_protocol_version_11` sibling pins the same invariant under v11
    /// (legacy aggregator, but the bump emission is identical).
    async fn run_token_burn_trying_to_burn_more_than_we_have_at_protocol_version(
        protocol_version: dpp::version::ProtocolVersion,
    ) {
        let platform_version = PlatformVersion::get(protocol_version)
            .expect("expected platform version for the requested protocol_version");
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(protocol_version)
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
        .await
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
            [PaidConsensusError {
                error: ConsensusError::StateError(
                    StateError::IdentityDoesNotHaveEnoughTokenBalanceError(_)
                ),
                ..
            }]
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

    /// PROTOCOL_VERSION_12+: pins the tokens-always-pay invariant under the
    /// new v1 aggregator.
    #[tokio::test]
    async fn test_token_burn_trying_to_burn_more_than_we_have() {
        run_token_burn_trying_to_burn_more_than_we_have_at_protocol_version(
            PlatformVersion::latest().protocol_version,
        )
        .await;
    }

    /// PROTOCOL_VERSION_11: pins the same invariant under the legacy v0
    /// aggregator (the bump emission is identical across versions for
    /// tokens, so both run paths must produce PaidConsensusError).
    #[tokio::test]
    async fn test_token_burn_trying_to_burn_more_than_we_have_protocol_version_11() {
        run_token_burn_trying_to_burn_more_than_we_have_at_protocol_version(11).await;
    }

    #[tokio::test]
    async fn test_token_burn_gives_error_if_trying_to_burn_from_not_allowed_identity() {
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
        .await
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
            [PaidConsensusError {
                error: ConsensusError::StateError(StateError::UnauthorizedTokenActionError(_)),
                ..
            }]
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

    #[tokio::test]
    async fn test_token_burn_group_action_tokens_transferred_before_completion() {
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
        .await
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
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
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
        .await
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
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
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
        .await
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
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
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

    #[tokio::test]
    async fn test_token_burn_group_action_tokens_transferred_before_completion_not_enough_balance()
    {
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
        .await
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
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
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
        .await
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
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
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
        .await
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
            [PaidConsensusError {
                error: ConsensusError::StateError(
                    StateError::IdentityDoesNotHaveEnoughTokenBalanceError(_)
                ),
                ..
            }]
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

    // --------------------------------------------------------------
    // Expanded coverage tests
    // --------------------------------------------------------------

    #[tokio::test]
    async fn test_token_burn_updates_total_supply_correctly() {
        // Burn should decrement the token's total supply by the same amount removed
        // from the identity's balance.
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

        // Sanity: total supply should start equal to the base_supply (100000).
        let total_supply_before = platform
            .drive
            .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");
        assert_eq!(total_supply_before, Some(100000));

        let burn_transition = BatchTransition::new_token_burn_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            25000,
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
        .expect("expect to create burn transition");

        let serialized = burn_transition
            .serialize_to_bytes()
            .expect("expected serialized state transition");

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

        let balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch balance");
        assert_eq!(balance, Some(75000));

        let total_supply_after = platform
            .drive
            .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");
        assert_eq!(total_supply_after, Some(75000));
    }

    #[tokio::test]
    async fn test_token_burn_below_base_supply_allowed() {
        // base_supply is purely the INITIAL mint at contract creation; there is no guard
        // anywhere preventing burns from dropping total_supply below base_supply. This is
        // allowed by design. This test locks in that invariant: burning
        // 60_000 from a base supply of 100_000 leaves a total of 40_000, below base.
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

        let total_supply_before = platform
            .drive
            .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");
        assert_eq!(total_supply_before, Some(100000));

        let burn_transition = BatchTransition::new_token_burn_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            60000,
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
        .expect("expect to create burn transition");

        let serialized = burn_transition
            .serialize_to_bytes()
            .expect("expected serialized state transition");

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

        // Supply dropped below the original base_supply (100_000) and that is allowed.
        let balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch balance");
        assert_eq!(balance, Some(40000));

        let total_supply_after = platform
            .drive
            .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");
        assert_eq!(total_supply_after, Some(40000));
    }

    #[tokio::test]
    async fn test_token_burn_entire_supply_then_mint_again() {
        // Burning the entire supply to zero must leave the supply entry at Some(0) (not
        // absent), so a subsequent mint resumes correctly rather than hitting a
        // CorruptedDriveState "total supply not found". Existing depletion coverage only
        // checks that a *further burn* fails; this checks that a mint after depletion
        // works.
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

        // Burn the entire base supply (100_000) to zero.
        let burn_transition = BatchTransition::new_token_burn_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            100000,
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
        .expect("expect to create burn transition");

        let serialized = burn_transition
            .serialize_to_bytes()
            .expect("expected serialized state transition");

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

        let balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch balance");
        assert_eq!(balance, Some(0));

        let total_supply = platform
            .drive
            .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");
        assert_eq!(total_supply, Some(0));

        // Now mint again from the depleted (but present) supply entry.
        let mint_transition = BatchTransition::new_token_mint_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            500,
            Some(identity.id()),
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
        .expect("expect to create mint transition");

        let serialized = mint_transition
            .serialize_to_bytes()
            .expect("expected serialized state transition");

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

        let total_supply = platform
            .drive
            .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");
        assert_eq!(total_supply, Some(500));

        let balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch balance");
        assert_eq!(balance, Some(500));
    }

    #[tokio::test]
    async fn test_token_burn_with_public_note_succeeds() {
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

        let burn_transition = BatchTransition::new_token_burn_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            500,
            Some("burning some old tokens".to_string()),
            None,
            &key,
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("expect to create burn transition");

        let serialized = burn_transition
            .serialize_to_bytes()
            .expect("expected serialized state transition");

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

        let balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch balance");
        assert_eq!(balance, Some(99500));
    }

    #[tokio::test]
    async fn test_token_burn_with_public_note_too_big_fails() {
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

        let oversized_note = "x".repeat(MAX_TOKEN_NOTE_LEN + 1);
        let burn_transition = BatchTransition::new_token_burn_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            500,
            Some(oversized_note),
            None,
            &key,
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("expect to create burn transition");

        let serialized = burn_transition
            .serialize_to_bytes()
            .expect("expected serialized state transition");

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
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::BasicError(BasicError::InvalidTokenNoteTooBigError(_))
            )]
        );

        // Balance unchanged
        let balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch balance");
        assert_eq!(balance, Some(100000));
    }

    #[tokio::test]
    async fn test_token_burn_zero_amount_fails() {
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

        let burn_transition = BatchTransition::new_token_burn_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            0,
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
        .expect("expect to create burn transition");

        let serialized = burn_transition
            .serialize_to_bytes()
            .expect("expected serialized state transition");

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

        // A zero burn is invalid and rejected at structure validation.
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::BasicError(BasicError::InvalidTokenAmountError(_))
            )]
        );

        // No change to balance
        let balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch balance");
        assert_eq!(balance, Some(100000));

        // Total supply unchanged
        let total_supply = platform
            .drive
            .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");
        assert_eq!(total_supply, Some(100000));
    }

    #[tokio::test]
    async fn test_token_burn_allowed_when_rule_is_contract_owner_explicit() {
        // Explicitly set the burning rule to ContractOwner and verify the contract
        // owner can still burn (distinct from the default-rules happy path).
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
                token_configuration.set_manual_burning_rules(ChangeControlRules::V0(
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

        let burn_transition = BatchTransition::new_token_burn_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            1500,
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
        .expect("expect to create burn transition");

        let serialized = burn_transition
            .serialize_to_bytes()
            .expect("expected serialized state transition");

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

        let balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch balance");
        assert_eq!(balance, Some(98500));

        let total_supply = platform
            .drive
            .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");
        assert_eq!(total_supply, Some(98500));
    }

    #[tokio::test]
    async fn test_token_burn_allowed_by_specific_identity_rule() {
        // Allow burning only by a specific (non-owner) identity.
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(49853);
        let platform_state = platform.state.load();

        let (owner, _owner_signer, _owner_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (burner, burner_signer, burner_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let burner_id = burner.id();
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            owner.id(),
            Some(move |token_configuration: &mut TokenConfiguration| {
                token_configuration.set_manual_burning_rules(ChangeControlRules::V0(
                    ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::Identity(burner_id),
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

        // Give the burner some tokens to burn.
        add_tokens_to_identity(&platform, token_id, burner.id(), 40000);

        let burn_transition = BatchTransition::new_token_burn_transition(
            token_id,
            burner.id(),
            contract.id(),
            0,
            15000,
            None,
            None,
            &burner_key,
            2,
            0,
            &burner_signer,
            platform_version,
            None,
        )
        .await
        .expect("expect to create burn transition");

        let serialized = burn_transition
            .serialize_to_bytes()
            .expect("expected serialized state transition");

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

        let burner_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                burner.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch balance");
        assert_eq!(burner_balance, Some(25000));

        // Owner balance untouched (still base_supply).
        let owner_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                owner.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch balance");
        assert_eq!(owner_balance, Some(100000));

        // Total supply = 100000 (owner) + 40000 (added) - 15000 (burned) = 125000
        let total_supply = platform
            .drive
            .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");
        assert_eq!(total_supply, Some(125000));
    }

    #[tokio::test]
    async fn test_token_burn_specific_identity_rule_rejects_other_identity() {
        // Burning rule is Identity(burner); owner attempts to burn -> unauthorized.
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(49853);
        let platform_state = platform.state.load();

        let (owner, owner_signer, owner_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (burner, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let burner_id = burner.id();
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            owner.id(),
            Some(move |token_configuration: &mut TokenConfiguration| {
                token_configuration.set_manual_burning_rules(ChangeControlRules::V0(
                    ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::Identity(burner_id),
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

        let burn_transition = BatchTransition::new_token_burn_transition(
            token_id,
            owner.id(),
            contract.id(),
            0,
            1000,
            None,
            None,
            &owner_key,
            2,
            0,
            &owner_signer,
            platform_version,
            None,
        )
        .await
        .expect("expect to create burn transition");

        let serialized = burn_transition
            .serialize_to_bytes()
            .expect("expected serialized state transition");

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
                error: ConsensusError::StateError(StateError::UnauthorizedTokenActionError(_)),
                ..
            }]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let owner_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                owner.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch balance");
        assert_eq!(owner_balance, Some(100000));
    }

    #[tokio::test]
    async fn test_token_burn_rule_no_one_blocks_even_owner() {
        // With NoOne rule, even the contract owner cannot burn.
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
                token_configuration.set_manual_burning_rules(ChangeControlRules::V0(
                    ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::NoOne,
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

        let burn_transition = BatchTransition::new_token_burn_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            1,
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
        .expect("expect to create burn transition");

        let serialized = burn_transition
            .serialize_to_bytes()
            .expect("expected serialized state transition");

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
                error: ConsensusError::StateError(StateError::UnauthorizedTokenActionError(_)),
                ..
            }]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch balance");
        assert_eq!(balance, Some(100000));

        let total_supply = platform
            .drive
            .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");
        assert_eq!(total_supply, Some(100000));
    }

    #[tokio::test]
    async fn test_token_burn_from_frozen_account_succeeds() {
        // Burn validation does NOT check freeze status (only transfer does),
        // so a frozen account can still have its tokens burned. This test locks
        // in the current behavior and covers the removal path for frozen
        // identities.
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(49853);
        let platform_state = platform.state.load();

        let (owner, owner_signer, owner_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            owner.id(),
            Some(|token_configuration: &mut TokenConfiguration| {
                token_configuration.set_freeze_rules(ChangeControlRules::V0(
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

        // Freeze the contract owner's own balance.
        let freeze_transition = BatchTransition::new_token_freeze_transition(
            token_id,
            owner.id(),
            contract.id(),
            0,
            owner.id(),
            None,
            None,
            &owner_key,
            2,
            0,
            &owner_signer,
            platform_version,
            None,
        )
        .await
        .expect("expected to create freeze transition");

        let freeze_serialized = freeze_transition
            .serialize_to_bytes()
            .expect("expected to serialize freeze");

        let transaction = platform.drive.grove.start_transaction();
        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[freeze_serialized],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process freeze");
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit freeze");

        // Attempt the burn on the frozen identity.
        let burn_transition = BatchTransition::new_token_burn_transition(
            token_id,
            owner.id(),
            contract.id(),
            0,
            20000,
            None,
            None,
            &owner_key,
            3,
            0,
            &owner_signer,
            platform_version,
            None,
        )
        .await
        .expect("expected to create burn transition");

        let burn_serialized = burn_transition
            .serialize_to_bytes()
            .expect("expected to serialize burn");

        let transaction = platform.drive.grove.start_transaction();
        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[burn_serialized],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process burn");
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit burn");

        let balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                owner.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch balance");
        assert_eq!(balance, Some(80000));

        let total_supply = platform
            .drive
            .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");
        assert_eq!(total_supply, Some(80000));
    }

    #[tokio::test]
    async fn test_token_burn_from_identity_with_no_token_balance_record() {
        // Authorized by rule but the burning identity has no token balance record at all.
        // Should yield IdentityDoesNotHaveEnoughTokenBalanceError.
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(49853);
        let platform_state = platform.state.load();

        let (owner, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (burner, burner_signer, burner_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let burner_id = burner.id();
        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            owner.id(),
            Some(move |token_configuration: &mut TokenConfiguration| {
                token_configuration.set_manual_burning_rules(ChangeControlRules::V0(
                    ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::Identity(burner_id),
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

        // burner is authorized by the rule but has NO token balance record at all.
        let burn_transition = BatchTransition::new_token_burn_transition(
            token_id,
            burner.id(),
            contract.id(),
            0,
            1,
            None,
            None,
            &burner_key,
            2,
            0,
            &burner_signer,
            platform_version,
            None,
        )
        .await
        .expect("expect to create burn transition");

        let serialized = burn_transition
            .serialize_to_bytes()
            .expect("expected serialized state transition");

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
                error: ConsensusError::StateError(
                    StateError::IdentityDoesNotHaveEnoughTokenBalanceError(_)
                ),
                ..
            }]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        // Burner's balance still absent (None).
        let balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                burner.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch balance");
        assert_eq!(balance, None);

        // Owner supply intact.
        let total_supply = platform
            .drive
            .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");
        assert_eq!(total_supply, Some(100000));
    }

    #[tokio::test]
    async fn test_token_burn_sequential_depletes_balance_and_supply() {
        // Two successful burns in sequence should correctly decrement both
        // the identity's balance and the total supply.
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

        for (nonce, amount, expected_balance, expected_supply) in [
            (2u64, 30000u64, 70000u64, 70000u64),
            (3, 40000, 30000, 30000),
        ] {
            let burn_transition = BatchTransition::new_token_burn_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                amount,
                None,
                None,
                &key,
                nonce,
                0,
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create burn transition");

            let serialized = burn_transition
                .serialize_to_bytes()
                .expect("expected serialized state transition");

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

            let balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    identity.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch balance");
            assert_eq!(balance, Some(expected_balance));

            let total_supply = platform
                .drive
                .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
                .expect("expected to fetch total supply");
            assert_eq!(total_supply, Some(expected_supply));
        }
    }

    #[tokio::test]
    async fn test_token_burn_by_group_single_member_sufficient_power() {
        // Group rule, but the proposer alone has enough power to finalize the
        // action in one shot (mirrors the analogous mint test).
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(49853);
        let platform_state = platform.state.load();

        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (identity_2, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
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
                        members: [(identity.id(), 5), (identity_2.id(), 1)].into(),
                        required_power: 5,
                    }),
                )]
                .into(),
            ),
            None,
            platform_version,
        );

        let burn_transition = BatchTransition::new_token_burn_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            2500,
            None,
            Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
            &key,
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("expect to create burn transition");

        let serialized = burn_transition
            .serialize_to_bytes()
            .expect("expected serialized state transition");

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

        let balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch balance");
        assert_eq!(balance, Some(97500));

        let total_supply = platform
            .drive
            .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");
        assert_eq!(total_supply, Some(97500));
    }

    #[tokio::test]
    async fn test_token_burn_group_action_resubmit_same_signer_fails() {
        // Proposer + confirmer path, then proposer tries to submit the confirmation
        // again -> GroupActionAlreadySignedByIdentityError.
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(49853);
        let platform_state = platform.state.load();

        let (identity1, signer1, key1) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (identity2, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

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

        let burn_transition = BatchTransition::new_token_burn_transition(
            token_id,
            identity1.id(),
            contract.id(),
            0,
            1000,
            None,
            Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
            &key1,
            2,
            0,
            &signer1,
            platform_version,
            None,
        )
        .await
        .expect("expected to create burn transition");

        let serialized = burn_transition
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
            .expect("expected to process");
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        // Same signer tries to submit as OtherSigner -> already signed error.
        let action_id = TokenBurnTransition::calculate_action_id_with_fields(
            token_id.as_bytes(),
            identity1.id().as_bytes(),
            2,
            1000,
        );

        let resubmit_transition = BatchTransition::new_token_burn_transition(
            token_id,
            identity1.id(),
            contract.id(),
            0,
            1000,
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
            &key1,
            3,
            0,
            &signer1,
            platform_version,
            None,
        )
        .await
        .expect("expected to create resubmit transition");

        let serialized = resubmit_transition
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
            .expect("expected to process");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [PaidConsensusError {
                error: ConsensusError::StateError(
                    StateError::GroupActionAlreadySignedByIdentityError(_)
                ),
                ..
            }]
        );
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        // Proposer's balance is unchanged because the action has not yet been
        // finalized (only 1/2 power).
        let balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity1.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch balance");
        assert_eq!(balance, Some(100000));
    }

    #[tokio::test]
    async fn test_token_burn_group_action_submit_after_completion_fails() {
        // Three-member group with required power 2: proposer + one confirmer completes
        // the action, then a third member tries to confirm -> AlreadyCompleted.
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
        let (identity3, signer3, key3) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

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
                        members: [
                            (identity1.id(), 1),
                            (identity2.id(), 1),
                            (identity3.id(), 1),
                        ]
                        .into(),
                        required_power: 2,
                    }),
                )]
                .into(),
            ),
            None,
            platform_version,
        );

        // 1. Proposer
        let burn_transition = BatchTransition::new_token_burn_transition(
            token_id,
            identity1.id(),
            contract.id(),
            0,
            1000,
            None,
            Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
            &key1,
            2,
            0,
            &signer1,
            platform_version,
            None,
        )
        .await
        .expect("expected to create burn transition");

        let serialized = burn_transition
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
            .expect("expected to process");
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        let action_id = TokenBurnTransition::calculate_action_id_with_fields(
            token_id.as_bytes(),
            identity1.id().as_bytes(),
            2,
            1000,
        );

        // 2. Confirmer 2 completes (reaches required power 2).
        let confirm_transition = BatchTransition::new_token_burn_transition(
            token_id,
            identity2.id(),
            contract.id(),
            0,
            1000,
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
        .await
        .expect("expected to create confirm transition");

        let serialized = confirm_transition
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
            .expect("expected to process");
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        // Balance already burnt.
        let balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity1.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch balance");
        assert_eq!(balance, Some(99000));

        // 3. Third member tries to confirm an already-completed action.
        let late_transition = BatchTransition::new_token_burn_transition(
            token_id,
            identity3.id(),
            contract.id(),
            0,
            1000,
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
            &key3,
            2,
            0,
            &signer3,
            platform_version,
            None,
        )
        .await
        .expect("expected to create late transition");

        let serialized = late_transition
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
            .expect("expected to process");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [PaidConsensusError {
                error: ConsensusError::StateError(StateError::GroupActionAlreadyCompletedError(_)),
                ..
            }]
        );
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        // Total supply only decremented once.
        let total_supply = platform
            .drive
            .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");
        assert_eq!(total_supply, Some(99000));
    }

    #[tokio::test]
    async fn test_token_burn_group_proposer_not_in_group_fails() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(49853);
        let platform_state = platform.state.load();

        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (identity_2, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (identity_3, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
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
                        // identity is NOT a member
                        members: [(identity_2.id(), 1), (identity_3.id(), 1)].into(),
                        required_power: 2,
                    }),
                )]
                .into(),
            ),
            None,
            platform_version,
        );

        let burn_transition = BatchTransition::new_token_burn_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            1000,
            None,
            Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
            &key,
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("expected to create burn transition");

        let serialized = burn_transition
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
            .expect("expected to process");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [PaidConsensusError {
                error: ConsensusError::StateError(StateError::IdentityNotMemberOfGroupError(_)),
                ..
            }]
        );
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        let balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch balance");
        assert_eq!(balance, Some(100000));
    }

    #[tokio::test]
    async fn test_token_burn_group_other_signer_not_in_group_fails() {
        // Proposer succeeds, but a non-member tries to confirm.
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(49853);
        let platform_state = platform.state.load();

        let (identity, signer, key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (identity_2, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (outsider, outsider_signer, outsider_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity.id(),
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
                        members: [(identity.id(), 1), (identity_2.id(), 1)].into(),
                        required_power: 2,
                    }),
                )]
                .into(),
            ),
            None,
            platform_version,
        );

        // Proposer succeeds
        let burn_transition = BatchTransition::new_token_burn_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            1000,
            None,
            Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
            &key,
            2,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("expected to create burn transition");

        let serialized = burn_transition
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
            .expect("expected to process");
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        let action_id = TokenBurnTransition::calculate_action_id_with_fields(
            token_id.as_bytes(),
            identity.id().as_bytes(),
            2,
            1000,
        );

        // Outsider attempts to confirm.
        let confirm_transition = BatchTransition::new_token_burn_transition(
            token_id,
            outsider.id(),
            contract.id(),
            0,
            1000,
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
            &outsider_key,
            2,
            0,
            &outsider_signer,
            platform_version,
            None,
        )
        .await
        .expect("expected to create confirm transition");

        let serialized = confirm_transition
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
            .expect("expected to process");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [PaidConsensusError {
                error: ConsensusError::StateError(StateError::IdentityNotMemberOfGroupError(_)),
                ..
            }]
        );
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        // Proposer balance still intact - action is still pending.
        let balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch balance");
        assert_eq!(balance, Some(100000));
    }

    #[tokio::test]
    async fn test_token_burn_group_confirmer_with_note_fails() {
        // Only the proposer may attach a note. The confirmer attempting to attach
        // a note is rejected with TokenNoteOnlyAllowedWhenProposerError.
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

        // Proposer with a valid note.
        let burn_transition = BatchTransition::new_token_burn_transition(
            token_id,
            identity1.id(),
            contract.id(),
            0,
            1500,
            Some("proposer note".to_string()),
            Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
            &key1,
            2,
            0,
            &signer1,
            platform_version,
            None,
        )
        .await
        .expect("expected to create burn transition");

        let serialized = burn_transition
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
            .expect("expected to process");
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        let action_id = TokenBurnTransition::calculate_action_id_with_fields(
            token_id.as_bytes(),
            identity1.id().as_bytes(),
            2,
            1500,
        );

        // Confirmer attaches a note -> reject.
        let confirm_with_note = BatchTransition::new_token_burn_transition(
            token_id,
            identity2.id(),
            contract.id(),
            0,
            1500,
            Some("confirmer note not allowed".to_string()),
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
        .await
        .expect("expected to create confirm transition");

        let serialized = confirm_with_note
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
            .expect("expected to process");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::BasicError(BasicError::TokenNoteOnlyAllowedWhenProposerError(_))
            )]
        );
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        // Proposer balance still intact; action still pending.
        let balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity1.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch balance");
        assert_eq!(balance, Some(100000));
    }

    #[tokio::test]
    async fn test_token_burn_group_confirmer_modifies_amount_fails() {
        // Confirmer attempts to burn a different amount than the proposer -> modification
        // of main parameters is rejected.
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

        // 1. Proposer for burn_amount 2000
        let burn_transition = BatchTransition::new_token_burn_transition(
            token_id,
            identity1.id(),
            contract.id(),
            0,
            2000,
            None,
            Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
            &key1,
            2,
            0,
            &signer1,
            platform_version,
            None,
        )
        .await
        .expect("expected to create burn transition");

        let serialized = burn_transition
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
            .expect("expected to process");
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        // 2. Confirmer uses the proposer's action_id (2000) but submits burn_amount 3000
        let action_id = TokenBurnTransition::calculate_action_id_with_fields(
            token_id.as_bytes(),
            identity1.id().as_bytes(),
            2,
            2000,
        );

        let mismatched_confirm = BatchTransition::new_token_burn_transition(
            token_id,
            identity2.id(),
            contract.id(),
            0,
            3000, // different from proposer
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
        .await
        .expect("expected to create confirm transition");

        let serialized = mismatched_confirm
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
            .expect("expected to process");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [PaidConsensusError {
                error: ConsensusError::StateError(
                    StateError::ModificationOfGroupActionMainParametersNotPermittedError(_)
                ),
                ..
            }]
        );
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        // Balance still unaffected.
        let balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity1.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch balance");
        assert_eq!(balance, Some(100000));
    }

    #[tokio::test]
    async fn test_token_burn_by_main_group_rule() {
        // Verify the MainGroup authorization path: the rule says MainGroup, and the
        // main_control_group is set to 0. A member of group 0 proposes the burn.
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

        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            identity1.id(),
            Some(|token_configuration: &mut TokenConfiguration| {
                token_configuration.set_main_control_group(Some(0));
                token_configuration.set_manual_burning_rules(ChangeControlRules::V0(
                    ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::MainGroup,
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

        // Proposer
        let burn_transition = BatchTransition::new_token_burn_transition(
            token_id,
            identity1.id(),
            contract.id(),
            0,
            500,
            None,
            Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
            &key1,
            2,
            0,
            &signer1,
            platform_version,
            None,
        )
        .await
        .expect("expected to create burn transition");

        let serialized = burn_transition
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
            .expect("expected to process");
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        // Confirmer
        let action_id = TokenBurnTransition::calculate_action_id_with_fields(
            token_id.as_bytes(),
            identity1.id().as_bytes(),
            2,
            500,
        );

        let confirm_transition = BatchTransition::new_token_burn_transition(
            token_id,
            identity2.id(),
            contract.id(),
            0,
            500,
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
        .await
        .expect("expected to create confirm transition");

        let serialized = confirm_transition
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
            .expect("expected to process");
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        let balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity1.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch balance");
        assert_eq!(balance, Some(99500));

        let total_supply = platform
            .drive
            .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");
        assert_eq!(total_supply, Some(99500));
    }

    #[tokio::test]
    async fn test_token_burn_after_full_depletion_fails_with_insufficient_balance() {
        // Burn entire balance, then try to burn more -> insufficient balance.
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

        // Burn full balance
        let burn_all = BatchTransition::new_token_burn_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            100000,
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
        .expect("expect to create burn transition");

        let serialized = burn_all
            .serialize_to_bytes()
            .expect("expected serialized state transition");

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
            .expect("expected to process");
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        // Second burn at zero balance -> error.
        let burn_again = BatchTransition::new_token_burn_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            1,
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
        .expect("expect to create burn transition");

        let serialized = burn_again
            .serialize_to_bytes()
            .expect("expected serialized state transition");

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
            .expect("expected to process");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [PaidConsensusError {
                error: ConsensusError::StateError(
                    StateError::IdentityDoesNotHaveEnoughTokenBalanceError(_)
                ),
                ..
            }]
        );
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        let balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch balance");
        assert_eq!(balance, Some(0));

        let total_supply = platform
            .drive
            .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");
        assert_eq!(total_supply, Some(0));
    }

    #[tokio::test]
    async fn test_token_burn_by_one_then_by_another_both_independent() {
        // Two identities both authorized via ContractOwner + Identity rules? The
        // AuthorizedActionTakers enum only has a single variant per rule, so we
        // pick Identity() for a second holder and verify bursts from distinct
        // identities decrement their own balances independently.
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(49853);
        let platform_state = platform.state.load();

        let (owner, owner_signer, owner_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            owner.id(),
            None::<fn(&mut TokenConfiguration)>,
            None,
            None,
            None,
            platform_version,
        );

        // Seed a second identity with a balance. Default rule only allows contract
        // owner to burn, so the second identity only *holds* tokens here; only the
        // owner does burns — but the burn targets the owner's own balance.
        let (holder, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        add_tokens_to_identity(&platform, token_id, holder.id(), 7000);

        // Burn from owner (allowed).
        let burn1 = BatchTransition::new_token_burn_transition(
            token_id,
            owner.id(),
            contract.id(),
            0,
            4000,
            None,
            None,
            &owner_key,
            2,
            0,
            &owner_signer,
            platform_version,
            None,
        )
        .await
        .expect("expect to create burn transition");

        let serialized = burn1.serialize_to_bytes().expect("expected serialized");
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
            .expect("expected to process");
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        // The holder's balance is untouched.
        let holder_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                holder.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch balance");
        assert_eq!(holder_balance, Some(7000));

        // Owner's balance reduced.
        let owner_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                owner.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch balance");
        assert_eq!(owner_balance, Some(96000));

        // Total supply: 100000 + 7000 - 4000 = 103000
        let total_supply = platform
            .drive
            .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");
        assert_eq!(total_supply, Some(103000));
    }

    /// Pins the confirmer-step processing fee for a token group burn.
    ///
    /// The confirmer (action_is_proposer=false) triggers three drive reads
    /// inside `try_from_borrowed_base_transition_with_contract_lookup`:
    /// `fetch_action_is_closed`,
    /// `fetch_action_id_signers_power_and_add_operations`,
    /// `fetch_active_action_info_and_add_operations`. Their cost is
    /// accumulated into a `FeeResult` and added to the
    /// `execution_context`.
    ///
    /// Under `transform_into_action: 1` (PROTOCOL_VERSION_12+) the outer
    /// `execution_context` is threaded through the transformer, so this
    /// fee_result reaches the user's bill. Under v0 (PROTOCOL_VERSION_11
    /// and below) the fee_result lands in a dropped local ctx — verified
    /// empirically by toggling the version field and re-running this test
    /// (see commit message of the version bump for the recorded delta).
    #[tokio::test]
    async fn test_token_burn_group_action_confirmer_fee_includes_transformer_reads() {
        // PROTOCOL_VERSION_13: 4_367_880 (up from 4_319_240 at PV12) — the
        // PV13 genesis registers the document history contract and stores the
        // larger DPNS v2 schema, which changes the contracts-subtree node
        // sizes and therefore the byte-billed group-action contract reads.
        run_token_burn_group_action_confirmer_fee_includes_transformer_reads_at_protocol_version(
            PlatformVersion::latest().protocol_version,
            // PROTOCOL_VERSION_14: +400 — genesis system documents now carry
            // the contract-version stamp, shifting byte-billed subtree reads
            4_368_280,
        )
        .await;
    }

    /// PROTOCOL_VERSION_11: pre-B7 fee — the transformer's local execution
    /// context was dropped, so the three group-action drive reads
    /// (fetch_action_is_closed +
    /// fetch_action_id_signers_power_and_add_operations +
    /// fetch_active_action_info_and_add_operations) cost 30_820 credits
    /// that were not billed. Pinned so v11 chain history stays bit-for-bit
    /// reproducible.
    #[tokio::test]
    async fn test_token_burn_group_action_confirmer_fee_includes_transformer_reads_protocol_version_11(
    ) {
        run_token_burn_group_action_confirmer_fee_includes_transformer_reads_at_protocol_version(
            11, 4_288_420,
        )
        .await;
    }

    async fn run_token_burn_group_action_confirmer_fee_includes_transformer_reads_at_protocol_version(
        protocol_version: dpp::version::ProtocolVersion,
        expected_processing_fee: dpp::fee::Credits,
    ) {
        let platform_version = PlatformVersion::get(protocol_version)
            .expect("expected platform version for the requested protocol_version");
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(protocol_version)
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(49853);
        let platform_state = platform.state.load();

        let (identity1, signer1, key1) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
        let (identity2, signer2, key2) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

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

        add_tokens_to_identity(&platform, token_id, identity1.id(), 100000);

        // Step 1: identity1 proposes the burn — action_is_proposer=true, so
        // `try_from_borrowed_base_transition_with_contract_lookup` skips the
        // group-action drive reads (the only path that adds non-empty
        // fee_results inside the transformer).
        let propose_transition = BatchTransition::new_token_burn_transition(
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
        .await
        .expect("expected to create proposer burn transition");

        let propose_serialized = propose_transition
            .serialize_to_bytes()
            .expect("expected to serialize proposer burn");

        let transaction = platform.drive.grove.start_transaction();
        let proposer_result = platform
            .platform
            .process_raw_state_transitions(
                &[propose_serialized],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process proposer burn");
        assert_eq!(proposer_result.valid_count(), 1);
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit proposer burn");

        // Step 2: identity2 confirms the burn — action_is_proposer=false.
        // The confirmer's `try_from_borrowed_base_transition_with_contract_lookup`
        // does the three group-action drive reads whose cost we now bill.
        let action_id = TokenBurnTransition::calculate_action_id_with_fields(
            token_id.as_bytes(),
            identity1.id().as_bytes(),
            2,
            100000,
        );

        let confirm_transition = BatchTransition::new_token_burn_transition(
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
        .await
        .expect("expected to create confirmer burn transition");

        let confirm_serialized = confirm_transition
            .serialize_to_bytes()
            .expect("expected to serialize confirmer burn");

        let transaction = platform.drive.grove.start_transaction();
        let confirmer_result = platform
            .platform
            .process_raw_state_transitions(
                &[confirm_serialized],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process confirmer burn");
        assert_eq!(confirmer_result.valid_count(), 1);
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit confirmer burn");

        // Pin the confirmer's fee, which now includes the three
        // group-action read costs previously dropped via the local
        // execution_context.
        //
        // Empirical values captured during development:
        //   * `transform_into_action: 0` (legacy, dropped local ctx): 4_288_420
        //   * `transform_into_action: 1` (current, threaded outer ctx): 4_319_240
        //   * delta = 30_820 credits = the three transformer-phase reads
        //     (fetch_action_is_closed +
        //      fetch_action_id_signers_power_and_add_operations +
        //      fetch_active_action_info_and_add_operations) that were
        //     previously billed to a dropped context.
        assert_eq!(
            confirmer_result.aggregated_fees().processing_fee,
            expected_processing_fee,
            "PROTOCOL_VERSION_{}: confirmer step processing fee must match the version-specific baseline (transformer-phase group-action reads billed at PV12+, dropped at PV11)",
            protocol_version,
        );
    }
}
