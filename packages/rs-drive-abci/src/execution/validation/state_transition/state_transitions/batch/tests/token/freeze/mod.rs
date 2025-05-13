use super::*;
mod token_freeze_tests {
    use super::*;
    use dpp::tokens::info::v0::IdentityTokenInfoV0Accessors;

    mod token_freeze_basic_tests {
        use super::*;
        #[test]
        fn test_token_freeze() {
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
                }),
                None,
                None,
                platform_version,
            );

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
            .expect("expect to create documents batch transition");

            let freeze_serialized_transition = freeze_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![freeze_serialized_transition.clone()],
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

        #[test]
        fn test_token_freeze_and_unfreeze() {
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
                platform_version,
            );

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
            .expect("expect to create documents batch transition");

            let freeze_serialized_transition = freeze_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![freeze_serialized_transition.clone()],
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
            .expect("expect to create documents batch transition");

            let unfreeze_serialized_transition = unfreeze_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![unfreeze_serialized_transition.clone()],
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

        #[test]
        fn test_token_frozen_receive_balance_allowed_sending_not_allowed_till_unfrozen() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49853);

            let platform_state = platform.state.load();

            let (identity, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (recipient, signer2, key2) =
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
                platform_version,
            );

            let freeze_transition = BatchTransition::new_token_freeze_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                recipient.id(),
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

            let freeze_serialized_transition = freeze_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![freeze_serialized_transition.clone()],
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

            let token_frozen = platform
                .drive
                .fetch_identity_token_info(
                    token_id.to_buffer(),
                    recipient.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token info")
                .map(|info| info.frozen());
            assert_eq!(token_frozen, Some(true));

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
                    &vec![token_transfer_serialized_transition.clone()],
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

            //now let's try sending our balance

            let token_transfer_back_transition = BatchTransition::new_token_transfer_transition(
                token_id,
                recipient.id(),
                contract.id(),
                0,
                300,
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
            .expect("expect to create documents batch transition");

            let token_transfer_back_serialized_transition = token_transfer_back_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![token_transfer_back_serialized_transition.clone()],
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
                [StateTransitionExecutionResult::PaidConsensusError(
                    ConsensusError::StateError(StateError::IdentityTokenAccountFrozenError(_)),
                    _
                )]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            // We expect no change

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

            let unfreeze_transition = BatchTransition::new_token_unfreeze_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                recipient.id(),
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

            let unfreeze_serialized_transition = unfreeze_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![unfreeze_serialized_transition.clone()],
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

            let token_frozen = platform
                .drive
                .fetch_identity_token_info(
                    token_id.to_buffer(),
                    recipient.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token info")
                .map(|info| info.frozen());
            assert_eq!(token_frozen, Some(false));

            let token_transfer_transition = BatchTransition::new_token_transfer_transition(
                token_id,
                recipient.id(),
                contract.id(),
                0,
                300,
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
            .expect("expect to create documents batch transition");

            let token_transfer_serialized_transition = token_transfer_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![token_transfer_serialized_transition.clone()],
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
            let expected_amount = 100000 - 1337 + 300;
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
            let expected_amount = 1337 - 300;
            assert_eq!(token_balance, Some(expected_amount));
        }

        #[test]
        fn test_token_frozen_receive_balance_may_not_be_allowed() {
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
                    token_configuration.allow_transfer_to_frozen_balance(false);

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
                platform_version,
            );

            let freeze_transition = BatchTransition::new_token_freeze_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                recipient.id(),
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

            let freeze_serialized_transition = freeze_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![freeze_serialized_transition.clone()],
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

            let token_frozen = platform
                .drive
                .fetch_identity_token_info(
                    token_id.to_buffer(),
                    recipient.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token info")
                .map(|info| info.frozen());
            assert_eq!(token_frozen, Some(true));

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
                    &vec![token_transfer_serialized_transition.clone()],
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
                    ConsensusError::StateError(StateError::IdentityTokenAccountFrozenError(_)),
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
    }

    mod token_freeze_group_tests {
        use super::*;
        use dpp::data_contract::associated_token::token_keeps_history_rules::accessors::v0::TokenKeepsHistoryRulesV0Setters;
        use dpp::group::group_action_status::GroupActionStatus;
        use dpp::state_transition::batch_transition::{
            TokenDestroyFrozenFundsTransition, TokenFreezeTransition,
        };
        use dpp::state_transition::proof_result::StateTransitionProofResult;
        use dpp::tokens::info::v0::IdentityTokenInfoV0;
        use dpp::tokens::info::IdentityTokenInfo;
        use drive::drive::Drive;
        // ──────────────────────────────────────────────────────────
        //  Owner tries to freeze, but authorization is *group‑only*
        //  and owner does NOT hold enough power alone  →  Error.
        // ──────────────────────────────────────────────────────────
        #[test]
        fn test_token_freeze_owner_not_authorized_group_required() {
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

            // Contract where freeze / unfreeze require Group(0)
            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                owner.id(),
                Some(|token_cfg: &mut TokenConfiguration| {
                    token_cfg.set_freeze_rules(ChangeControlRules::V0(ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::Group(0),
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
                platform_version,
            );

            // Owner proposes freeze outside of a group action
            let freeze = BatchTransition::new_token_freeze_transition(
                token_id,
                owner.id(),
                contract.id(),
                0,
                member.id(),
                None,
                None,
                &key,
                2,
                0,
                &signer,
                platform_version,
                None,
            )
            .expect("create freeze");

            let serialized = freeze.serialize_to_bytes().expect("serialize freeze");

            let tx = platform.drive.grove.start_transaction();
            let result = platform
                .platform
                .process_raw_state_transitions(
                    &[serialized],
                    &platform_state,
                    &BlockInfo::default(),
                    &tx,
                    platform_version,
                    false,
                    None,
                )
                .expect("process");

            assert_matches!(
                result.execution_results().as_slice(),
                [PaidConsensusError(
                    ConsensusError::StateError(StateError::UnauthorizedTokenActionError(_)),
                    _
                )]
            );
        }

        // ──────────────────────────────────────────────────────────
        //  Owner alone HAS enough power in the group (5 ≥ required 5)
        //  → freeze succeeds immediately.
        // ──────────────────────────────────────────────────────────
        #[test]
        fn test_token_freeze_owner_enough_group_power_without_group_action() {
            let version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(43);
            let state = platform.state.load();

            let (owner, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
            let (other_identity_1, _, _) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
            let (other_identity_2, _, _) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
            let (target, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                owner.id(),
                Some(|cfg: &mut TokenConfiguration| {
                    cfg.set_freeze_rules(ChangeControlRules::V0(ChangeControlRulesV0 {
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
                            members: [
                                (owner.id(), 2),
                                (other_identity_1.id(), 1),
                                (other_identity_2.id(), 1),
                            ]
                            .into(),
                            required_power: 2,
                        }),
                    )]
                    .into(),
                ),
                version,
            );

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
                version,
                None,
            )
            .unwrap();

            let freeze_ser = freeze.serialize_to_bytes().unwrap();
            let tx = platform.drive.grove.start_transaction();
            let res = platform
                .platform
                .process_raw_state_transitions(
                    &[freeze_ser],
                    &state,
                    &BlockInfo::default(),
                    &tx,
                    version,
                    false,
                    None,
                )
                .unwrap();

            assert_matches!(
                res.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );

            platform
                .drive
                .grove
                .commit_transaction(tx)
                .unwrap()
                .unwrap();

            let frozen = platform
                .drive
                .fetch_identity_token_info(
                    token_id.to_buffer(),
                    target.id().to_buffer(),
                    None,
                    version,
                )
                .unwrap()
                .map(|i| i.frozen());
            assert_eq!(frozen, Some(true));
        }

        // ──────────────────────────────────────────────────────────
        //  Owner alone HAS enough power in the group (5 ≥ required 5)
        //  → freeze succeeds immediately.
        // ──────────────────────────────────────────────────────────
        #[test]
        fn test_token_freeze_owner_enough_group_power_using_group_action() {
            let version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(43);
            let state = platform.state.load();

            let (owner, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
            let (other_identity_1, _, _) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
            let (other_identity_2, _, _) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
            let (target, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                owner.id(),
                Some(|cfg: &mut TokenConfiguration| {
                    cfg.set_freeze_rules(ChangeControlRules::V0(ChangeControlRulesV0 {
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
                            members: [
                                (owner.id(), 2),
                                (other_identity_1.id(), 1),
                                (other_identity_2.id(), 1),
                            ]
                            .into(),
                            required_power: 2,
                        }),
                    )]
                    .into(),
                ),
                version,
            );

            let freeze = BatchTransition::new_token_freeze_transition(
                token_id,
                owner.id(),
                contract.id(),
                0,
                target.id(),
                None,
                Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
                &key,
                2,
                0,
                &signer,
                version,
                None,
            )
            .unwrap();

            let freeze_ser = freeze.serialize_to_bytes().unwrap();
            let tx = platform.drive.grove.start_transaction();
            let res = platform
                .platform
                .process_raw_state_transitions(
                    &[freeze_ser],
                    &state,
                    &BlockInfo::default(),
                    &tx,
                    version,
                    false,
                    None,
                )
                .unwrap();

            assert_matches!(
                res.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );

            platform
                .drive
                .grove
                .commit_transaction(tx)
                .unwrap()
                .unwrap();

            let frozen = platform
                .drive
                .fetch_identity_token_info(
                    token_id.to_buffer(),
                    target.id().to_buffer(),
                    None,
                    version,
                )
                .unwrap()
                .map(|i| i.frozen());
            assert_eq!(frozen, Some(true));
        }

        #[test]
        fn test_token_freeze_two_member_group_no_keeping_history() {
            test_token_freeze_two_member_group_with_keeps_history(false);
        }

        #[test]
        fn test_token_freeze_two_member_group_keeping_history() {
            test_token_freeze_two_member_group_with_keeps_history(true);
        }

        // ──────────────────────────────────────────────────────────
        //  Two‑signer scenario: proposer + second member complete freeze
        // ──────────────────────────────────────────────────────────
        fn test_token_freeze_two_member_group_with_keeps_history(keeps_freezing_history: bool) {
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

            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                id1.id(),
                Some(|cfg: &mut TokenConfiguration| {
                    cfg.keeps_history_mut()
                        .set_keeps_freezing_history(keeps_freezing_history);
                    cfg.set_freeze_rules(ChangeControlRules::V0(ChangeControlRulesV0 {
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
                platform_version,
            );

            // proposer
            let token_freeze_transition = BatchTransition::new_token_freeze_transition(
                token_id,
                id1.id(),
                contract.id(),
                0,
                target.id(),
                None,
                Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
                &key1,
                2,
                0,
                &sign1,
                platform_version,
                None,
            )
            .expect("expect to create batch transition");

            let token_freeze_serialized_transition = token_freeze_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[token_freeze_serialized_transition.clone()],
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

            // Prove & verify
            let proof = platform
                .drive
                .prove_state_transition(&token_freeze_transition, None, platform_version)
                .expect("expect to prove state transition");
            let (_root_hash, result) = Drive::verify_state_transition_was_executed_with_proof(
                &token_freeze_transition,
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
                        assert_eq!(info, None);
                    }
                );
            }

            // second signer
            let action_id = TokenFreezeTransition::calculate_action_id_with_fields(
                token_id.as_bytes(),
                id1.id().as_bytes(),
                2,
                target.id().as_bytes(),
            );

            let token_freeze_confirm_transition = BatchTransition::new_token_freeze_transition(
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
            .unwrap();

            let token_freeze_serialized_transition = token_freeze_confirm_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[token_freeze_serialized_transition.clone()],
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

            // Prove & verify
            let proof = platform
                .drive
                .prove_state_transition(&token_freeze_confirm_transition, None, platform_version)
                .expect("expect to prove state transition");
            let (_root_hash, result) = Drive::verify_state_transition_was_executed_with_proof(
                &token_freeze_confirm_transition,
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
                        assert_eq!(info, Some(IdentityTokenInfo::V0(IdentityTokenInfoV0 { frozen: true })));
                    }
                );
            }

            // Verify frozen
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

        #[test]
        fn test_token_freeze_two_member_group_and_destroy_frozen_funds() {
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

            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                id1.id(),
                Some(|cfg: &mut TokenConfiguration| {
                    cfg.keeps_history_mut().set_keeps_freezing_history(true);
                    cfg.set_freeze_rules(ChangeControlRules::V0(ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                        admin_action_takers: AuthorizedActionTakers::NoOne,
                        ..Default::default()
                    }));
                    cfg.set_manual_minting_rules(ChangeControlRules::V0(ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                        admin_action_takers: AuthorizedActionTakers::NoOne,
                        ..Default::default()
                    }));
                    cfg.distribution_rules_mut()
                        .set_minting_allow_choosing_destination(true);
                    cfg.set_destroy_frozen_funds_rules(ChangeControlRules::V0(
                        ChangeControlRulesV0 {
                            authorized_to_make_change: AuthorizedActionTakers::Group(0),
                            admin_action_takers: AuthorizedActionTakers::NoOne,
                            ..Default::default()
                        },
                    ));
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
                platform_version,
            );

            let token_mint_transition = BatchTransition::new_token_mint_transition(
                token_id,
                id1.id(),
                contract.id(),
                0,
                1337,
                Some(target.id()),
                Some("minty".to_string()),
                None,
                &key1,
                2,
                0,
                &sign1,
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

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    target.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, Some(1337));

            // proposer
            let token_freeze_transition = BatchTransition::new_token_freeze_transition(
                token_id,
                id1.id(),
                contract.id(),
                0,
                target.id(),
                None,
                None,
                &key1,
                3,
                0,
                &sign1,
                platform_version,
                None,
            )
            .expect("expect to create batch transition");

            let token_freeze_serialized_transition = token_freeze_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[token_freeze_serialized_transition.clone()],
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

            // Verify still frozen
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

            let token_destroy_frozen_funds_transition =
                BatchTransition::new_token_destroy_frozen_funds_transition(
                    token_id,
                    id1.id(),
                    contract.id(),
                    0,
                    target.id(),
                    None,
                    Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
                    &key1,
                    4,
                    0,
                    &sign1,
                    platform_version,
                    None,
                )
                .unwrap();

            let token_destroy_frozen_funds_serialized_transition =
                token_destroy_frozen_funds_transition
                    .serialize_to_bytes()
                    .expect("expected serialized state transition");

            let transaction = platform.drive.grove.start_transaction();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[token_destroy_frozen_funds_serialized_transition.clone()],
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

            // second signer
            let action_id = TokenDestroyFrozenFundsTransition::calculate_action_id_with_fields(
                token_id.as_bytes(),
                id1.id().as_bytes(),
                4,
                target.id().as_bytes(),
            );

            let token_destroy_frozen_funds_confirm_transition =
                BatchTransition::new_token_destroy_frozen_funds_transition(
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
                .unwrap();

            let token_destroy_frozen_funds_serialized_confirm_transition =
                token_destroy_frozen_funds_confirm_transition
                    .serialize_to_bytes()
                    .expect("expected serialized state transition");

            let transaction = platform.drive.grove.start_transaction();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[token_destroy_frozen_funds_serialized_confirm_transition.clone()],
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

            // Prove & verify
            let proof = platform
                .drive
                .prove_state_transition(
                    &token_destroy_frozen_funds_confirm_transition,
                    None,
                    platform_version,
                )
                .expect("expect to prove state transition");
            let (_root_hash, result) = Drive::verify_state_transition_was_executed_with_proof(
                &token_destroy_frozen_funds_confirm_transition,
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
            assert_matches!(
                result,
                StateTransitionProofResult::VerifiedTokenGroupActionWithDocument(power, doc) => {
                    assert_eq!(power, 2);
                    assert_eq!(doc.expect("expected document").properties().get_identifier("frozenIdentityId").expect("expected frozen id"), target.id());
                }
            );

            // Verify still frozen
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

            // Verify balance is 0
            let frozen_identity_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    target.id().to_buffer(),
                    None,
                    platform_version,
                )
                .unwrap();
            assert_eq!(frozen_identity_balance, Some(0));
        }

        #[test]
        fn test_token_freeze_two_member_group_and_destroy_frozen_funds_change_target_id_mid_group_action(
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
            let (target2, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                id1.id(),
                Some(|cfg: &mut TokenConfiguration| {
                    cfg.keeps_history_mut().set_keeps_freezing_history(true);
                    cfg.set_freeze_rules(ChangeControlRules::V0(ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                        admin_action_takers: AuthorizedActionTakers::NoOne,
                        ..Default::default()
                    }));
                    cfg.set_manual_minting_rules(ChangeControlRules::V0(ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                        admin_action_takers: AuthorizedActionTakers::NoOne,
                        ..Default::default()
                    }));
                    cfg.distribution_rules_mut()
                        .set_minting_allow_choosing_destination(true);
                    cfg.set_destroy_frozen_funds_rules(ChangeControlRules::V0(
                        ChangeControlRulesV0 {
                            authorized_to_make_change: AuthorizedActionTakers::Group(0),
                            admin_action_takers: AuthorizedActionTakers::NoOne,
                            ..Default::default()
                        },
                    ));
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
                platform_version,
            );

            let token_mint_transition = BatchTransition::new_token_mint_transition(
                token_id,
                id1.id(),
                contract.id(),
                0,
                1337,
                Some(target.id()),
                Some("minty".to_string()),
                None,
                &key1,
                2,
                0,
                &sign1,
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

            let token_mint_transition = BatchTransition::new_token_mint_transition(
                token_id,
                id1.id(),
                contract.id(),
                0,
                4000,
                Some(target2.id()),
                Some("minty".to_string()),
                None,
                &key1,
                6,
                0,
                &sign1,
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

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    target.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, Some(1337));

            let token_freeze_transition = BatchTransition::new_token_freeze_transition(
                token_id,
                id1.id(),
                contract.id(),
                0,
                target.id(),
                None,
                None,
                &key1,
                3,
                0,
                &sign1,
                platform_version,
                None,
            )
            .expect("expect to create batch transition");

            let token_freeze_serialized_transition = token_freeze_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[token_freeze_serialized_transition.clone()],
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

            let token_freeze_transition = BatchTransition::new_token_freeze_transition(
                token_id,
                id1.id(),
                contract.id(),
                0,
                target2.id(),
                None,
                None,
                &key1,
                7,
                0,
                &sign1,
                platform_version,
                None,
            )
            .expect("expect to create batch transition");

            let token_freeze_serialized_transition = token_freeze_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[token_freeze_serialized_transition.clone()],
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

            // Verify still frozen
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

            let token_destroy_frozen_funds_transition =
                BatchTransition::new_token_destroy_frozen_funds_transition(
                    token_id,
                    id1.id(),
                    contract.id(),
                    0,
                    target.id(),
                    None,
                    Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
                    &key1,
                    4,
                    0,
                    &sign1,
                    platform_version,
                    None,
                )
                .unwrap();

            let token_destroy_frozen_funds_serialized_transition =
                token_destroy_frozen_funds_transition
                    .serialize_to_bytes()
                    .expect("expected serialized state transition");

            let transaction = platform.drive.grove.start_transaction();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[token_destroy_frozen_funds_serialized_transition.clone()],
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

            // second signer
            let action_id = TokenDestroyFrozenFundsTransition::calculate_action_id_with_fields(
                token_id.as_bytes(),
                id1.id().as_bytes(),
                4,
                target.id().as_bytes(),
            );

            // Here is the test, we switch to target2, which should not be allowed
            let token_destroy_frozen_funds_confirm_transition =
                BatchTransition::new_token_destroy_frozen_funds_transition(
                    token_id,
                    id2.id(),
                    contract.id(),
                    0,
                    target2.id(),
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
                .unwrap();

            let token_destroy_frozen_funds_serialized_confirm_transition =
                token_destroy_frozen_funds_confirm_transition
                    .serialize_to_bytes()
                    .expect("expected serialized state transition");

            let transaction = platform.drive.grove.start_transaction();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[token_destroy_frozen_funds_serialized_confirm_transition.clone()],
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
                        StateError::ModificationOfGroupActionMainParametersNotPermittedError(_)
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

            // Verify balance is not 0
            let frozen_identity_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    target.id().to_buffer(),
                    None,
                    platform_version,
                )
                .unwrap();
            assert_eq!(frozen_identity_balance, Some(1337));

            // Verify balance is not 0
            let frozen_identity_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    target2.id().to_buffer(),
                    None,
                    platform_version,
                )
                .unwrap();
            assert_eq!(frozen_identity_balance, Some(4000));
        }
    }
}
