use super::*;
mod token_unfreeze_tests {
    use super::*;
    use dpp::tokens::info::v0::IdentityTokenInfoV0Accessors;

    mod token_unfreeze_basic_tests {
        use super::*;

        // ──────────────────────────────────────────────────────────
        //  Successfully unfreeze a previously-frozen identity's balance.
        // ──────────────────────────────────────────────────────────
        #[tokio::test]
        async fn test_token_unfreeze_previously_frozen_identity() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49853);

            let platform_state = platform.state.load();

            let (identity, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (identity_2, _, _) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                identity.id(),
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
                    token_configuration.set_unfreeze_rules(ChangeControlRules::V0(
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

            // First freeze identity_2
            let freeze_transition = BatchTransition::new_token_freeze_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                identity_2.id(),
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
            .expect("expect to create freeze transition");

            let freeze_serialized = freeze_transition
                .serialize_to_bytes()
                .expect("expected serialized state transition");

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

            // Confirm frozen before unfreezing
            let token_frozen = platform
                .drive
                .fetch_identity_token_info(
                    token_id.to_buffer(),
                    identity_2.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token info")
                .map(|info| info.frozen());
            assert_eq!(token_frozen, Some(true));

            // Now unfreeze
            let unfreeze_transition = BatchTransition::new_token_unfreeze_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                identity_2.id(),
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
            .expect("expect to create unfreeze transition");

            let unfreeze_serialized = unfreeze_transition
                .serialize_to_bytes()
                .expect("expected serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[unfreeze_serialized],
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

            let token_frozen = platform
                .drive
                .fetch_identity_token_info(
                    token_id.to_buffer(),
                    identity_2.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token info")
                .map(|info| info.frozen());
            assert_eq!(token_frozen, Some(false));
        }

        // ──────────────────────────────────────────────────────────
        //  Unfreeze when the identity was never frozen  →  Error.
        //  The validator returns IdentityTokenAccountNotFrozenError.
        // ──────────────────────────────────────────────────────────
        #[tokio::test]
        async fn test_token_unfreeze_never_frozen_identity_fails() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49853);

            let platform_state = platform.state.load();

            let (identity, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (identity_2, _, _) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                identity.id(),
                Some(|token_configuration: &mut TokenConfiguration| {
                    token_configuration.set_unfreeze_rules(ChangeControlRules::V0(
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

            let unfreeze_transition = BatchTransition::new_token_unfreeze_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                identity_2.id(),
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
            .expect("expect to create unfreeze transition");

            let unfreeze_serialized = unfreeze_transition
                .serialize_to_bytes()
                .expect("expected serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[unfreeze_serialized],
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
                        StateError::IdentityTokenAccountNotFrozenError(_)
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

            // State must not have been mutated.
            let token_frozen = platform
                .drive
                .fetch_identity_token_info(
                    token_id.to_buffer(),
                    identity_2.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token info")
                .map(|info| info.frozen());
            assert_eq!(token_frozen, None);
        }

        // ──────────────────────────────────────────────────────────
        //  Unfreezing an already-unfrozen identity  →  Error
        //  (no-op not allowed by the spec).
        // ──────────────────────────────────────────────────────────
        #[tokio::test]
        async fn test_token_unfreeze_already_unfrozen_fails() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49853);

            let platform_state = platform.state.load();

            let (identity, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (identity_2, _, _) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                identity.id(),
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
                    token_configuration.set_unfreeze_rules(ChangeControlRules::V0(
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

            // freeze, then unfreeze, then try to unfreeze again
            let freeze_transition = BatchTransition::new_token_freeze_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                identity_2.id(),
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
            .expect("expect to create freeze transition");

            let freeze_serialized = freeze_transition
                .serialize_to_bytes()
                .expect("expected serialized state transition");

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

            let unfreeze_transition = BatchTransition::new_token_unfreeze_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                identity_2.id(),
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
            .expect("expect to create unfreeze transition");

            let unfreeze_serialized = unfreeze_transition
                .serialize_to_bytes()
                .expect("expected serialized state transition");

            let transaction = platform.drive.grove.start_transaction();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[unfreeze_serialized],
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

            // Second unfreeze should now fail.
            let second_unfreeze = BatchTransition::new_token_unfreeze_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                identity_2.id(),
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
            .expect("expect to create unfreeze transition");

            let second_unfreeze_serialized = second_unfreeze
                .serialize_to_bytes()
                .expect("expected serialized state transition");

            let transaction = platform.drive.grove.start_transaction();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[second_unfreeze_serialized],
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
                        StateError::IdentityTokenAccountNotFrozenError(_)
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
        }

        // ──────────────────────────────────────────────────────────
        //  A non-authorized identity cannot unfreeze.
        //  Unfreeze is restricted to the contract owner, and a second
        //  identity tries to unfreeze → UnauthorizedTokenActionError.
        // ──────────────────────────────────────────────────────────
        #[tokio::test]
        async fn test_token_unfreeze_non_authorized_identity_fails() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49853);

            let platform_state = platform.state.load();

            let (owner, owner_signer, owner_key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (identity_2, _, _) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (stranger, stranger_signer, stranger_key) =
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
                    token_configuration.set_unfreeze_rules(ChangeControlRules::V0(
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

            // Owner freezes identity_2 first
            let freeze_transition = BatchTransition::new_token_freeze_transition(
                token_id,
                owner.id(),
                contract.id(),
                0,
                identity_2.id(),
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
            .expect("expect to create freeze transition");

            let freeze_serialized = freeze_transition
                .serialize_to_bytes()
                .expect("expected serialized state transition");

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

            // stranger tries to unfreeze
            let unfreeze_transition = BatchTransition::new_token_unfreeze_transition(
                token_id,
                stranger.id(),
                contract.id(),
                0,
                identity_2.id(),
                None,
                None,
                &stranger_key,
                2,
                0,
                &stranger_signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create unfreeze transition");

            let unfreeze_serialized = unfreeze_transition
                .serialize_to_bytes()
                .expect("expected serialized state transition");

            let transaction = platform.drive.grove.start_transaction();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[unfreeze_serialized],
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

            // identity_2 must remain frozen — unauthorized action must not mutate state.
            let token_frozen = platform
                .drive
                .fetch_identity_token_info(
                    token_id.to_buffer(),
                    identity_2.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token info")
                .map(|info| info.frozen());
            assert_eq!(token_frozen, Some(true));
        }

        // ──────────────────────────────────────────────────────────
        //  After unfreezing, a previously-frozen account can send tokens
        //  (verifying unfrozen state is truly effective).
        // ──────────────────────────────────────────────────────────
        #[tokio::test]
        async fn test_token_unfreeze_allows_subsequent_transfer() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49853);

            let platform_state = platform.state.load();

            let (identity, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (identity_2, signer2, key2) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                identity.id(),
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
                    token_configuration.set_unfreeze_rules(ChangeControlRules::V0(
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

            // Transfer tokens to identity_2 first.
            let transfer_in = BatchTransition::new_token_transfer_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                7000,
                identity_2.id(),
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
            .await
            .expect("expect to create transfer transition");

            let transfer_serialized = transfer_in
                .serialize_to_bytes()
                .expect("expected serialized state transition");

            let transaction = platform.drive.grove.start_transaction();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[transfer_serialized],
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

            // Freeze identity_2.
            let freeze_transition = BatchTransition::new_token_freeze_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                identity_2.id(),
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
            .expect("expect to create freeze transition");

            let freeze_serialized = freeze_transition
                .serialize_to_bytes()
                .expect("expected serialized state transition");

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

            // Confirm send-while-frozen fails.
            let blocked_send = BatchTransition::new_token_transfer_transition(
                token_id,
                identity_2.id(),
                contract.id(),
                0,
                100,
                identity.id(),
                None,
                None,
                None,
                &key2,
                2,
                0,
                &signer2,
                platform_version,
                None,
            )
            .await
            .expect("expect to create transfer transition");

            let blocked_send_serialized = blocked_send
                .serialize_to_bytes()
                .expect("expected serialized state transition");

            let transaction = platform.drive.grove.start_transaction();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[blocked_send_serialized],
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
                    error: ConsensusError::StateError(StateError::IdentityTokenAccountFrozenError(
                        _
                    )),
                    ..
                }]
            );
            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            // Unfreeze.
            let unfreeze_transition = BatchTransition::new_token_unfreeze_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                identity_2.id(),
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
            .expect("expect to create unfreeze transition");

            let unfreeze_serialized = unfreeze_transition
                .serialize_to_bytes()
                .expect("expected serialized state transition");

            let transaction = platform.drive.grove.start_transaction();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[unfreeze_serialized],
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

            // Transfer now succeeds.
            let allowed_send = BatchTransition::new_token_transfer_transition(
                token_id,
                identity_2.id(),
                contract.id(),
                0,
                250,
                identity.id(),
                None,
                None,
                None,
                &key2,
                3,
                0,
                &signer2,
                platform_version,
                None,
            )
            .await
            .expect("expect to create transfer transition");

            let allowed_send_serialized = allowed_send
                .serialize_to_bytes()
                .expect("expected serialized state transition");

            let transaction = platform.drive.grove.start_transaction();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[allowed_send_serialized],
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

            // Balances reflect: identity started 100000, sent 7000 out, received 250.
            let balance_identity = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    identity.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(balance_identity, Some(100000 - 7000 + 250));

            let balance_identity_2 = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    identity_2.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(balance_identity_2, Some(7000 - 250));
        }

        // ──────────────────────────────────────────────────────────
        //  Unfreeze authorized by a specific Identity (not ContractOwner).
        //  A dedicated "unfreezer" identity is the only party allowed to
        //  unfreeze. This exercises the Identity(...) branch of
        //  AuthorizedActionTakers during unfreeze.
        // ──────────────────────────────────────────────────────────
        #[tokio::test]
        async fn test_token_unfreeze_authorized_identity() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49853);
            let platform_state = platform.state.load();

            let (owner, owner_signer, owner_key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (unfreezer, unfreezer_signer, unfreezer_key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (target, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let unfreezer_id = unfreezer.id();
            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                owner.id(),
                Some(move |token_configuration: &mut TokenConfiguration| {
                    token_configuration.set_freeze_rules(ChangeControlRules::V0(
                        ChangeControlRulesV0 {
                            authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                            admin_action_takers: AuthorizedActionTakers::NoOne,
                            ..Default::default()
                        },
                    ));
                    // Only the specific `unfreezer` identity may unfreeze.
                    token_configuration.set_unfreeze_rules(ChangeControlRules::V0(
                        ChangeControlRulesV0 {
                            authorized_to_make_change: AuthorizedActionTakers::Identity(
                                unfreezer_id,
                            ),
                            admin_action_takers: AuthorizedActionTakers::NoOne,
                            ..Default::default()
                        },
                    ));
                }),
                None,
                None,
                None,
                platform_version,
            );

            // Owner freezes target (allowed because freeze_rules = ContractOwner).
            let freeze_transition = BatchTransition::new_token_freeze_transition(
                token_id,
                owner.id(),
                contract.id(),
                0,
                target.id(),
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
            .expect("expect to create freeze transition");
            let freeze_serialized = freeze_transition.serialize_to_bytes().unwrap();
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
                .unwrap();
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .unwrap();

            // Owner tries to unfreeze → rejected (owner isn't the authorized identity).
            let owner_unfreeze = BatchTransition::new_token_unfreeze_transition(
                token_id,
                owner.id(),
                contract.id(),
                0,
                target.id(),
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
            .expect("expect to create unfreeze transition");
            let owner_unfreeze_serialized = owner_unfreeze.serialize_to_bytes().unwrap();
            let transaction = platform.drive.grove.start_transaction();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[owner_unfreeze_serialized],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .unwrap();
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
                .unwrap();

            // Unfreezer identity now unfreezes — this should succeed.
            let unfreeze_transition = BatchTransition::new_token_unfreeze_transition(
                token_id,
                unfreezer.id(),
                contract.id(),
                0,
                target.id(),
                None,
                None,
                &unfreezer_key,
                2,
                0,
                &unfreezer_signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create unfreeze transition");
            let unfreeze_serialized = unfreeze_transition.serialize_to_bytes().unwrap();
            let transaction = platform.drive.grove.start_transaction();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[unfreeze_serialized],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .unwrap();
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .unwrap();

            let token_frozen = platform
                .drive
                .fetch_identity_token_info(
                    token_id.to_buffer(),
                    target.id().to_buffer(),
                    None,
                    platform_version,
                )
                .unwrap()
                .map(|i| i.frozen());
            assert_eq!(token_frozen, Some(false));
        }

        // ──────────────────────────────────────────────────────────
        //  Unfreeze with an attached public note.
        // ──────────────────────────────────────────────────────────
        #[tokio::test]
        async fn test_token_unfreeze_with_public_note() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49853);
            let platform_state = platform.state.load();

            let (identity, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (identity_2, _, _) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                identity.id(),
                Some(|cfg: &mut TokenConfiguration| {
                    cfg.set_freeze_rules(ChangeControlRules::V0(ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                        admin_action_takers: AuthorizedActionTakers::NoOne,
                        ..Default::default()
                    }));
                    cfg.set_unfreeze_rules(ChangeControlRules::V0(ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                        admin_action_takers: AuthorizedActionTakers::NoOne,
                        ..Default::default()
                    }));
                }),
                None,
                None,
                None,
                platform_version,
            );

            // freeze first
            let freeze_transition = BatchTransition::new_token_freeze_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                identity_2.id(),
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
            .expect("expect to create freeze transition");
            let freeze_serialized = freeze_transition.serialize_to_bytes().unwrap();
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
                .unwrap();
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .unwrap();

            // unfreeze with a public note
            let unfreeze_transition = BatchTransition::new_token_unfreeze_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                identity_2.id(),
                Some("Thawing the account per dispute resolution".to_string()),
                None,
                &key,
                3,
                0,
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create unfreeze transition");

            let unfreeze_serialized = unfreeze_transition.serialize_to_bytes().unwrap();
            let transaction = platform.drive.grove.start_transaction();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[unfreeze_serialized],
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
                .unwrap();

            let token_frozen = platform
                .drive
                .fetch_identity_token_info(
                    token_id.to_buffer(),
                    identity_2.id().to_buffer(),
                    None,
                    platform_version,
                )
                .unwrap()
                .map(|i| i.frozen());
            assert_eq!(token_frozen, Some(false));
        }

        // ──────────────────────────────────────────────────────────
        //  Unfreeze against a non-existent contract  →  Error.
        //  This confirms the contract lookup path rejects unknown contracts
        //  before hitting unfreeze logic.
        // ──────────────────────────────────────────────────────────
        #[tokio::test]
        async fn test_token_unfreeze_nonexistent_contract_fails() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49853);
            let platform_state = platform.state.load();

            let (identity, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (identity_2, _, _) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            // Random token / contract identifiers that were never created.
            let fake_contract_id = Identifier::random_with_rng(&mut rng);
            let fake_token_id = Identifier::random_with_rng(&mut rng);

            let unfreeze_transition = BatchTransition::new_token_unfreeze_transition(
                fake_token_id,
                identity.id(),
                fake_contract_id,
                0,
                identity_2.id(),
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
            .expect("expect to create unfreeze transition");

            let unfreeze_serialized = unfreeze_transition
                .serialize_to_bytes()
                .expect("expected serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[unfreeze_serialized],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // The only invariant we strictly require is that this is NOT a successful execution:
            // the validator should reject either because the contract is missing or the token is not frozen.
            assert!(!matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            ));

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");
        }
    }

    mod token_unfreeze_group_tests {
        use super::*;
        use dpp::data_contract::associated_token::token_keeps_history_rules::accessors::v0::TokenKeepsHistoryRulesV0Setters;
        use dpp::group::group_action_status::GroupActionStatus;
        use dpp::state_transition::batch_transition::TokenUnfreezeTransition;
        use dpp::state_transition::proof_result::StateTransitionProofResult;
        use dpp::tokens::info::v0::IdentityTokenInfoV0;
        use dpp::tokens::info::IdentityTokenInfo;
        use drive::drive::Drive;

        // ──────────────────────────────────────────────────────────
        //  Owner alone does NOT have enough group power  →  unfreeze fails
        //  with UnauthorizedTokenActionError.
        // ──────────────────────────────────────────────────────────
        #[tokio::test]
        async fn test_token_unfreeze_owner_not_authorized_group_required() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(42);
            let platform_state = platform.state.load();

            let (owner, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
            let (member, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
            let (target, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            // Contract where freeze by contract owner is allowed, but unfreeze
            // requires a 2-of-2 group.
            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                owner.id(),
                Some(|token_cfg: &mut TokenConfiguration| {
                    token_cfg.set_freeze_rules(ChangeControlRules::V0(ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                        admin_action_takers: AuthorizedActionTakers::NoOne,
                        ..Default::default()
                    }));
                    token_cfg.set_unfreeze_rules(ChangeControlRules::V0(ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::Group(0),
                        admin_action_takers: AuthorizedActionTakers::NoOne,
                        ..Default::default()
                    }));
                }),
                None,
                Some(
                    [(
                        0,
                        Group::V0(GroupV0 {
                            members: [(owner.id(), 1), (member.id(), 1)].into(),
                            required_power: 2,
                        }),
                    )]
                    .into(),
                ),
                None,
                platform_version,
            );

            // Freeze target first using owner.
            let freeze = BatchTransition::new_token_freeze_transition(
                token_id,
                owner.id(),
                contract.id(),
                0,
                target.id(),
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
            .expect("create freeze");

            let freeze_ser = freeze.serialize_to_bytes().unwrap();
            let tx = platform.drive.grove.start_transaction();
            let res = platform
                .platform
                .process_raw_state_transitions(
                    &[freeze_ser],
                    &platform_state,
                    &BlockInfo::default(),
                    &tx,
                    platform_version,
                    false,
                    None,
                )
                .unwrap();
            assert_matches!(
                res.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
            platform
                .drive
                .grove
                .commit_transaction(tx)
                .unwrap()
                .unwrap();

            // Owner proposes unfreeze outside of a group action → unauthorized.
            let unfreeze = BatchTransition::new_token_unfreeze_transition(
                token_id,
                owner.id(),
                contract.id(),
                0,
                target.id(),
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
            .expect("create unfreeze");

            let unfreeze_ser = unfreeze.serialize_to_bytes().unwrap();
            let tx = platform.drive.grove.start_transaction();
            let res = platform
                .platform
                .process_raw_state_transitions(
                    &[unfreeze_ser],
                    &platform_state,
                    &BlockInfo::default(),
                    &tx,
                    platform_version,
                    false,
                    None,
                )
                .unwrap();

            assert_matches!(
                res.execution_results().as_slice(),
                [PaidConsensusError {
                    error: ConsensusError::StateError(StateError::UnauthorizedTokenActionError(_)),
                    ..
                }]
            );

            platform
                .drive
                .grove
                .commit_transaction(tx)
                .unwrap()
                .unwrap();

            // target stays frozen.
            let frozen = platform
                .drive
                .fetch_identity_token_info(
                    token_id.to_buffer(),
                    target.id().to_buffer(),
                    None,
                    platform_version,
                )
                .unwrap()
                .map(|i| i.frozen());
            assert_eq!(frozen, Some(true));
        }

        #[tokio::test]
        async fn test_token_unfreeze_two_member_group_no_keeping_history() {
            test_token_unfreeze_two_member_group_with_keeps_history(false).await;
        }

        #[tokio::test]
        async fn test_token_unfreeze_two_member_group_keeping_history() {
            test_token_unfreeze_two_member_group_with_keeps_history(true).await;
        }

        // ──────────────────────────────────────────────────────────
        //  Two‑signer scenario: proposer + second member complete unfreeze.
        //  Verifies GroupStateTransitionInfo flow for unfreeze.
        // ──────────────────────────────────────────────────────────
        async fn test_token_unfreeze_two_member_group_with_keeps_history(
            keeps_freezing_history: bool,
        ) {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(44);
            let platform_state = platform.state.load();

            let (id1, sign1, key1) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
            let (id2, sign2, key2) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
            let (target, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            // Contract where both freeze and unfreeze require Group(0).
            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                id1.id(),
                Some(|cfg: &mut TokenConfiguration| {
                    cfg.keeps_history_mut()
                        .set_keeps_freezing_history(keeps_freezing_history);
                    cfg.set_freeze_rules(ChangeControlRules::V0(ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                        admin_action_takers: AuthorizedActionTakers::NoOne,
                        ..Default::default()
                    }));
                    cfg.set_unfreeze_rules(ChangeControlRules::V0(ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::Group(0),
                        admin_action_takers: AuthorizedActionTakers::NoOne,
                        ..Default::default()
                    }));
                }),
                None,
                Some(
                    [(
                        0,
                        Group::V0(GroupV0 {
                            members: [(id1.id(), 1), (id2.id(), 1)].into(),
                            required_power: 2,
                        }),
                    )]
                    .into(),
                ),
                None,
                platform_version,
            );

            // Contract owner freezes target first (allowed directly).
            let freeze = BatchTransition::new_token_freeze_transition(
                token_id,
                id1.id(),
                contract.id(),
                0,
                target.id(),
                None,
                None,
                &key1,
                2,
                0,
                &sign1,
                platform_version,
                None,
            )
            .await
            .expect("create freeze");
            let freeze_ser = freeze.serialize_to_bytes().unwrap();
            let tx = platform.drive.grove.start_transaction();
            let res = platform
                .platform
                .process_raw_state_transitions(
                    &[freeze_ser],
                    &platform_state,
                    &BlockInfo::default(),
                    &tx,
                    platform_version,
                    false,
                    None,
                )
                .unwrap();
            assert_matches!(
                res.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
            platform
                .drive
                .grove
                .commit_transaction(tx)
                .unwrap()
                .unwrap();

            // Proposer: id1 starts the group unfreeze action.
            let unfreeze_propose = BatchTransition::new_token_unfreeze_transition(
                token_id,
                id1.id(),
                contract.id(),
                0,
                target.id(),
                None,
                Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
                &key1,
                3,
                0,
                &sign1,
                platform_version,
                None,
            )
            .await
            .expect("expect to create batch transition");

            let unfreeze_propose_ser = unfreeze_propose
                .serialize_to_bytes()
                .expect("expected serialized state transition");

            let transaction = platform.drive.grove.start_transaction();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[unfreeze_propose_ser],
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

            // Prove & verify the proposer transition.
            let proof = platform
                .drive
                .prove_state_transition(&unfreeze_propose, None, platform_version)
                .expect("expect to prove state transition");
            let (_root_hash, result) = Drive::verify_state_transition_was_executed_with_proof(
                &unfreeze_propose,
                &BlockInfo::default(),
                proof.data.as_ref().expect("expected data"),
                &|_| Ok(Some(contract.clone().into())),
                platform_version,
            )
            .unwrap_or_else(|e| {
                panic!(
                    "expect to verify state transition proof {}, error is {}",
                    hex::encode(proof.data.expect("expected data")),
                    e
                )
            });

            if keeps_freezing_history {
                assert_matches!(
                    result,
                    StateTransitionProofResult::VerifiedTokenGroupActionWithDocument(power, doc) => {
                        assert_eq!(power, 1);
                        assert_eq!(doc, None);
                    }
                );
            } else {
                assert_matches!(
                    result,
                    StateTransitionProofResult::VerifiedTokenGroupActionWithTokenIdentityInfo(power, status, info) => {
                        assert_eq!(power, 1);
                        assert_eq!(status, GroupActionStatus::ActionActive);
                        // mid-action, state has not yet been flipped.
                        assert_eq!(info, Some(IdentityTokenInfo::V0(IdentityTokenInfoV0 { frozen: true })));
                    }
                );
            }

            // Second signer completes the group action.
            let action_id = TokenUnfreezeTransition::calculate_action_id_with_fields(
                token_id.as_bytes(),
                id1.id().as_bytes(),
                3,
                target.id().as_bytes(),
            );

            let unfreeze_confirm = BatchTransition::new_token_unfreeze_transition(
                token_id,
                id2.id(),
                contract.id(),
                0,
                target.id(),
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
                &sign2,
                platform_version,
                None,
            )
            .await
            .unwrap();

            let unfreeze_confirm_ser = unfreeze_confirm
                .serialize_to_bytes()
                .expect("expected serialized state transition");

            let transaction = platform.drive.grove.start_transaction();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[unfreeze_confirm_ser],
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

            // Prove & verify the confirmation.
            let proof = platform
                .drive
                .prove_state_transition(&unfreeze_confirm, None, platform_version)
                .expect("expect to prove state transition");
            let (_root_hash, result) = Drive::verify_state_transition_was_executed_with_proof(
                &unfreeze_confirm,
                &BlockInfo::default(),
                proof.data.as_ref().expect("expected data"),
                &|_| Ok(Some(contract.clone().into())),
                platform_version,
            )
            .unwrap_or_else(|e| {
                panic!(
                    "expect to verify state transition proof {}, error is {}",
                    hex::encode(proof.data.expect("expected data")),
                    e
                )
            });

            if keeps_freezing_history {
                assert_matches!(
                    result,
                    StateTransitionProofResult::VerifiedTokenGroupActionWithDocument(power, doc) => {
                        assert_eq!(power, 2);
                        assert_eq!(doc.expect("expected document").properties().get_identifier("frozenIdentityId").expect("expected frozen id"), target.id());
                    }
                );
            } else {
                assert_matches!(
                    result,
                    StateTransitionProofResult::VerifiedTokenGroupActionWithTokenIdentityInfo(power, status, info) => {
                        assert_eq!(power, 2);
                        assert_eq!(status, GroupActionStatus::ActionClosed);
                        // After group action completes, the target is no longer frozen.
                        assert_eq!(info, Some(IdentityTokenInfo::V0(IdentityTokenInfoV0 { frozen: false })));
                    }
                );
            }

            // Verify target is unfrozen on-chain.
            let frozen = platform
                .drive
                .fetch_identity_token_info(
                    token_id.to_buffer(),
                    target.id().to_buffer(),
                    None,
                    platform_version,
                )
                .unwrap()
                .map(|i| i.frozen());
            assert_eq!(frozen, Some(false));
        }
    }
}
