use super::*;

mod token_config_update_tests {
    use super::*;
    use dpp::data_contract::accessors::v1::DataContractV1Getters;
    use dpp::data_contract::associated_token::token_configuration_convention::TokenConfigurationConvention;
    use dpp::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;

    mod non_group {
        use dpp::state_transition::proof_result::StateTransitionProofResult;
        use drive::drive::Drive;

        use super::*;
        #[test]
        fn test_token_config_update_by_owner_changing_total_max_supply() {
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
                    token_configuration.set_max_supply_change_rules(ChangeControlRules::V0(
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

            let config_update_transition = BatchTransition::new_token_config_update_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                TokenConfigurationChangeItem::MaxSupply(Some(1000000)),
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

            let config_update_transition_serialized_transition = config_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[config_update_transition_serialized_transition.clone()],
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

            let contract = platform
                .drive
                .fetch_contract(
                    contract.id().to_buffer(),
                    None,
                    None,
                    None,
                    platform_version,
                )
                .unwrap()
                .expect("expected to fetch token balance")
                .expect("expected contract");
            let updated_token_config = contract
                .contract
                .expected_token_configuration(0)
                .expect("expected token configuration");
            assert_eq!(updated_token_config.max_supply(), Some(1000000));
        }

        /// Added this test to verify that adding "note" property to token history contract document types
        /// Makes the proof verification work
        #[test]
        fn test_token_config_update_by_owner_changing_total_max_supply_with_public_note() {
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
                    token_configuration.set_max_supply_change_rules(ChangeControlRules::V0(
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

            let config_update_transition = BatchTransition::new_token_config_update_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                TokenConfigurationChangeItem::MaxSupply(Some(1000000)),
                Some("this is a public note".to_string()),
                None,
                &key,
                2,
                0,
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

            let config_update_transition_serialized_transition = config_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[config_update_transition_serialized_transition.clone()],
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

            if processing_result.valid_count() == 1 {
                let proof_result = platform
                    .platform
                    .drive
                    .prove_state_transition(&config_update_transition, None, platform_version)
                    .map_err(|e| e.to_string())
                    .expect("expected to create proof");

                if let Some(proof_error) = proof_result.first_error() {
                    panic!("proof_result is not valid with error {}", proof_error);
                }

                let proof_data = proof_result
                    .into_data()
                    .map_err(|e| e.to_string())
                    .expect("expected to get proof data");

                let (_, verification_result) =
                    Drive::verify_state_transition_was_executed_with_proof(
                        &config_update_transition,
                        &BlockInfo::default(),
                        &proof_data,
                        &|id: &Identifier| {
                            if *id == contract.id() {
                                Ok(Some(contract.clone().into()))
                            } else {
                                Ok(None)
                            }
                        },
                        platform_version,
                    )
                    .map_err(|e| e.to_string())
                    .expect("expected to verify state transition");

                let StateTransitionProofResult::VerifiedTokenActionWithDocument(_document) =
                    verification_result
                else {
                    panic!(
                        "verification_result expected config update document, but got: {:?}",
                        verification_result
                    );
                };
            }

            let contract = platform
                .drive
                .fetch_contract(
                    contract.id().to_buffer(),
                    None,
                    None,
                    None,
                    platform_version,
                )
                .unwrap()
                .expect("expected to fetch token balance")
                .expect("expected contract");
            let updated_token_config = contract
                .contract
                .expected_token_configuration(0)
                .expect("expected token configuration");
            assert_eq!(updated_token_config.max_supply(), Some(1000000));
        }

        #[test]
        fn test_token_config_update_by_owner_changing_total_max_supply_to_less_than_current_supply()
        {
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
                    token_configuration.set_max_supply_change_rules(ChangeControlRules::V0(
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

            let config_update_transition = BatchTransition::new_token_config_update_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                TokenConfigurationChangeItem::MaxSupply(Some(1000)),
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

            let config_update_transition_serialized_transition = config_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[config_update_transition_serialized_transition.clone()],
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
                        StateError::TokenSettingMaxSupplyToLessThanCurrentSupplyError(_)
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

            let contract = platform
                .drive
                .fetch_contract(
                    contract.id().to_buffer(),
                    None,
                    None,
                    None,
                    platform_version,
                )
                .unwrap()
                .expect("expected to fetch token balance")
                .expect("expected contract");
            let updated_token_config = contract
                .contract
                .expected_token_configuration(0)
                .expect("expected token configuration");
            assert_eq!(updated_token_config.max_supply(), None);
        }

        #[test]
        fn test_token_config_update_by_owner_change_admin_to_another_identity() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49853);

            let platform_state = platform.state.load();

            let (identity, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (identity_2, signer_2, key_2) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                identity.id(),
                Some(|token_configuration: &mut TokenConfiguration| {
                    token_configuration.set_max_supply_change_rules(ChangeControlRules::V0(
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

            let config_update_transition = BatchTransition::new_token_config_update_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                TokenConfigurationChangeItem::MaxSupplyControlGroup(
                    AuthorizedActionTakers::Identity(identity_2.id()),
                ),
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

            let config_update_transition_serialized_transition = config_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[config_update_transition_serialized_transition.clone()],
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

            let config_update_transition = BatchTransition::new_token_config_update_transition(
                token_id,
                identity_2.id(),
                contract.id(),
                0,
                TokenConfigurationChangeItem::MaxSupply(Some(1000000)),
                None,
                None,
                &key_2,
                2,
                0,
                &signer_2,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

            let config_update_transition_serialized_transition = config_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[config_update_transition_serialized_transition.clone()],
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

            let contract = platform
                .drive
                .fetch_contract(
                    contract.id().to_buffer(),
                    None,
                    None,
                    None,
                    platform_version,
                )
                .unwrap()
                .expect("expected to fetch token balance")
                .expect("expected contract");
            let updated_token_config = contract
                .contract
                .expected_token_configuration(0)
                .expect("expected token configuration");
            assert_eq!(updated_token_config.max_supply(), Some(1000000));
        }

        #[test]
        fn test_token_config_update_by_owner_change_admin_to_a_non_existent_identity_error() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49853);

            let platform_state = platform.state.load();

            let (identity, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let identity_2_id = Identifier::random_with_rng(&mut rng);

            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                identity.id(),
                Some(|token_configuration: &mut TokenConfiguration| {
                    token_configuration.set_max_supply_change_rules(ChangeControlRules::V0(
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

            let config_update_transition = BatchTransition::new_token_config_update_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                TokenConfigurationChangeItem::MaxSupplyControlGroup(
                    AuthorizedActionTakers::Identity(identity_2_id),
                ),
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

            let config_update_transition_serialized_transition = config_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[config_update_transition_serialized_transition.clone()],
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
                        StateError::NewAuthorizedActionTakerIdentityDoesNotExistError(_)
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
        }

        #[test]
        fn test_token_config_update_by_owner_change_admin_to_a_non_existent_group_error() {
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
                    token_configuration.set_max_supply_change_rules(ChangeControlRules::V0(
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

            let config_update_transition = BatchTransition::new_token_config_update_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                TokenConfigurationChangeItem::MaxSupplyControlGroup(AuthorizedActionTakers::Group(
                    0,
                )),
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

            let config_update_transition_serialized_transition = config_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[config_update_transition_serialized_transition.clone()],
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
                        StateError::NewAuthorizedActionTakerGroupDoesNotExistError(_)
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
        }

        #[test]
        fn test_token_config_update_by_owner_change_admin_to_main_group_not_set_error() {
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
                    token_configuration.set_max_supply_change_rules(ChangeControlRules::V0(
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

            let config_update_transition = BatchTransition::new_token_config_update_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                TokenConfigurationChangeItem::MaxSupplyControlGroup(
                    AuthorizedActionTakers::MainGroup,
                ),
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

            let config_update_transition_serialized_transition = config_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[config_update_transition_serialized_transition.clone()],
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
                        StateError::NewAuthorizedActionTakerMainGroupNotSetError(_)
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
        }

        #[test]
        fn test_token_config_update_by_owner_changing_main_control_group() {
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
                    token_configuration.set_main_control_group_can_be_modified(
                        AuthorizedActionTakers::ContractOwner,
                    );
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

            let config_update_transition = BatchTransition::new_token_config_update_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                TokenConfigurationChangeItem::MainControlGroup(Some(0)),
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

            let config_update_transition_serialized_transition = config_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[config_update_transition_serialized_transition.clone()],
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

            let contract = platform
                .drive
                .fetch_contract(
                    contract.id().to_buffer(),
                    None,
                    None,
                    None,
                    platform_version,
                )
                .unwrap()
                .expect("expected to fetch token balance")
                .expect("expected contract");
            let updated_token_config = contract
                .contract
                .expected_token_configuration(0)
                .expect("expected token configuration");
            assert_eq!(updated_token_config.main_control_group(), Some(0));
        }
    }

    mod with_group {
        use super::*;
        use dpp::data_contract::associated_token::token_configuration_localization::v0::TokenConfigurationLocalizationV0;
        use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;

        #[test]
        fn test_token_config_update_by_group_member_changing_total_max_supply_not_using_group_gives_error(
        ) {
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
                    token_configuration.set_max_supply_change_rules(ChangeControlRules::V0(
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

            let config_update_transition = BatchTransition::new_token_config_update_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                TokenConfigurationChangeItem::MaxSupply(Some(1000000)),
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

            let config_update_transition_serialized_transition = config_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[config_update_transition_serialized_transition.clone()],
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

            let contract = platform
                .drive
                .fetch_contract(
                    contract.id().to_buffer(),
                    None,
                    None,
                    None,
                    platform_version,
                )
                .unwrap()
                .expect("expected to fetch token balance")
                .expect("expected contract");
            let updated_token_config = contract
                .contract
                .expected_token_configuration(0)
                .expect("expected token configuration");
            assert_eq!(updated_token_config.max_supply(), None);
        }

        #[test]
        fn test_token_config_update_by_group_member_changing_total_max_supply() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49853);

            let platform_state = platform.state.load();

            let (identity, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (identity_2, signer_2, key_2) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                identity.id(),
                Some(|token_configuration: &mut TokenConfiguration| {
                    token_configuration.set_max_supply_change_rules(ChangeControlRules::V0(
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

            let action_id = TokenConfigUpdateTransition::calculate_action_id_with_fields(
                token_id.as_bytes(),
                identity.id().as_bytes(),
                2,
                TokenConfigurationChangeItem::MaxSupply(Some(1000000)).u8_item_index(),
            );

            let config_update_transition = BatchTransition::new_token_config_update_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                TokenConfigurationChangeItem::MaxSupply(Some(1000000)),
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

            let config_update_transition_serialized_transition = config_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[config_update_transition_serialized_transition.clone()],
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

            let new_contract = platform
                .drive
                .fetch_contract(
                    contract.id().to_buffer(),
                    None,
                    None,
                    None,
                    platform_version,
                )
                .unwrap()
                .expect("expected to fetch token balance")
                .expect("expected contract");
            let updated_token_config = new_contract
                .contract
                .expected_token_configuration(0)
                .expect("expected token configuration");
            assert_eq!(updated_token_config.max_supply(), None);

            let config_update_transition = BatchTransition::new_token_config_update_transition(
                token_id,
                identity_2.id(),
                contract.id(),
                0,
                TokenConfigurationChangeItem::MaxSupply(Some(1000000)),
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
                &key_2,
                2,
                0,
                &signer_2,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

            let config_update_transition_serialized_transition = config_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[config_update_transition_serialized_transition.clone()],
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

            let new_contract = platform
                .drive
                .fetch_contract(
                    contract.id().to_buffer(),
                    None,
                    None,
                    None,
                    platform_version,
                )
                .unwrap()
                .expect("expected to fetch token balance")
                .expect("expected contract");
            let updated_token_config = new_contract
                .contract
                .expected_token_configuration(0)
                .expect("expected token configuration");
            assert_eq!(updated_token_config.max_supply(), Some(1000000));
        }

        #[test]
        fn test_token_config_update_by_group_member_changing_minting_destination_group() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49853);

            let platform_state = platform.state.load();

            let (identity, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (identity_2, signer_2, key_2) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (identity_3, _, _) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (identity_4, _, _) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                identity.id(),
                Some(|token_configuration: &mut TokenConfiguration| {
                    token_configuration
                        .distribution_rules_mut()
                        .set_minting_allow_choosing_destination(false);
                    token_configuration
                        .distribution_rules_mut()
                        .set_minting_allow_choosing_destination_rules(ChangeControlRules::V0(
                            ChangeControlRulesV0 {
                                authorized_to_make_change: AuthorizedActionTakers::Group(1),
                                admin_action_takers: AuthorizedActionTakers::Group(0),
                                changing_authorized_action_takers_to_no_one_allowed: false,
                                changing_admin_action_takers_to_no_one_allowed: false,
                                self_changing_admin_action_takers_allowed: false,
                            },
                        ));
                }),
                None,
                Some(
                    [
                        (
                            0,
                            Group::V0(GroupV0 {
                                members: [(identity.id(), 1), (identity_2.id(), 1)].into(),
                                required_power: 2,
                            }),
                        ),
                        (
                            1,
                            Group::V0(GroupV0 {
                                members: [(identity_3.id(), 1), (identity_4.id(), 1)].into(),
                                required_power: 2,
                            }),
                        ),
                    ]
                    .into(),
                ),
                None,
                platform_version,
            );

            let action_id = TokenConfigUpdateTransition::calculate_action_id_with_fields(
                token_id.as_bytes(),
                identity.id().as_bytes(),
                2,
                TokenConfigurationChangeItem::MintingAllowChoosingDestinationControlGroup(
                    AuthorizedActionTakers::Group(0),
                )
                .u8_item_index(),
            );

            let config_update_transition = BatchTransition::new_token_config_update_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                TokenConfigurationChangeItem::MintingAllowChoosingDestinationControlGroup(
                    AuthorizedActionTakers::Group(0),
                ),
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

            let config_update_transition_serialized_transition = config_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[config_update_transition_serialized_transition.clone()],
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

            let new_contract = platform
                .drive
                .fetch_contract(
                    contract.id().to_buffer(),
                    None,
                    None,
                    None,
                    platform_version,
                )
                .unwrap()
                .expect("expected to fetch token balance")
                .expect("expected contract");
            let updated_token_config = new_contract
                .contract
                .expected_token_configuration(0)
                .expect("expected token configuration");
            assert_eq!(
                updated_token_config
                    .distribution_rules()
                    .minting_allow_choosing_destination_rules()
                    .authorized_to_make_change_action_takers(),
                &AuthorizedActionTakers::Group(1)
            );

            let config_update_transition = BatchTransition::new_token_config_update_transition(
                token_id,
                identity_2.id(),
                contract.id(),
                0,
                TokenConfigurationChangeItem::MintingAllowChoosingDestinationControlGroup(
                    AuthorizedActionTakers::Group(0),
                ),
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
                &key_2,
                2,
                0,
                &signer_2,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

            let config_update_transition_serialized_transition = config_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[config_update_transition_serialized_transition.clone()],
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

            let new_contract = platform
                .drive
                .fetch_contract(
                    contract.id().to_buffer(),
                    None,
                    None,
                    None,
                    platform_version,
                )
                .unwrap()
                .expect("expected to fetch token balance")
                .expect("expected contract");
            let updated_token_config = new_contract
                .contract
                .expected_token_configuration(0)
                .expect("expected token configuration");
            assert_eq!(
                updated_token_config
                    .distribution_rules()
                    .minting_allow_choosing_destination_rules()
                    .authorized_to_make_change_action_takers(),
                &AuthorizedActionTakers::Group(0)
            );

            // now that we are group 0 as the group that can make the change, let's make the change to allow minting to choose the destination

            let action_id = TokenConfigUpdateTransition::calculate_action_id_with_fields(
                token_id.as_bytes(),
                identity.id().as_bytes(),
                3,
                TokenConfigurationChangeItem::MintingAllowChoosingDestination(true).u8_item_index(),
            );

            let config_update_transition = BatchTransition::new_token_config_update_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                TokenConfigurationChangeItem::MintingAllowChoosingDestination(true),
                None,
                Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
                &key,
                3,
                0,
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

            let config_update_transition_serialized_transition = config_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[config_update_transition_serialized_transition.clone()],
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

            let new_contract = platform
                .drive
                .fetch_contract(
                    contract.id().to_buffer(),
                    None,
                    None,
                    None,
                    platform_version,
                )
                .unwrap()
                .expect("expected to fetch token balance")
                .expect("expected contract");
            let updated_token_config = new_contract
                .contract
                .expected_token_configuration(0)
                .expect("expected token configuration");
            assert!(!updated_token_config
                .distribution_rules()
                .minting_allow_choosing_destination());

            let config_update_transition = BatchTransition::new_token_config_update_transition(
                token_id,
                identity_2.id(),
                contract.id(),
                0,
                TokenConfigurationChangeItem::MintingAllowChoosingDestination(true),
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
                &key_2,
                3,
                0,
                &signer_2,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

            let config_update_transition_serialized_transition = config_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[config_update_transition_serialized_transition.clone()],
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

            let new_contract = platform
                .drive
                .fetch_contract(
                    contract.id().to_buffer(),
                    None,
                    None,
                    None,
                    platform_version,
                )
                .unwrap()
                .expect("expected to fetch token balance")
                .expect("expected contract");
            let updated_token_config = new_contract
                .contract
                .expected_token_configuration(0)
                .expect("expected token configuration");
            assert!(updated_token_config
                .distribution_rules()
                .minting_allow_choosing_destination());
        }

        #[test]
        fn test_token_config_update_by_group_member_changing_minting_admin_group() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49853);

            let platform_state = platform.state.load();

            let (identity, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (identity_2, signer_2, key_2) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (identity_3, signer_3, key_3) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (identity_4, signer_4, key_4) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                identity.id(),
                Some(|token_configuration: &mut TokenConfiguration| {
                    token_configuration
                        .distribution_rules_mut()
                        .set_minting_allow_choosing_destination_rules(ChangeControlRules::V0(
                            ChangeControlRulesV0 {
                                authorized_to_make_change: AuthorizedActionTakers::Group(1),
                                admin_action_takers: AuthorizedActionTakers::Group(0),
                                changing_authorized_action_takers_to_no_one_allowed: false,
                                changing_admin_action_takers_to_no_one_allowed: false,
                                self_changing_admin_action_takers_allowed: true,
                            },
                        ));
                }),
                None,
                Some(
                    [
                        (
                            0,
                            Group::V0(GroupV0 {
                                members: [(identity.id(), 1), (identity_2.id(), 1)].into(),
                                required_power: 2,
                            }),
                        ),
                        (
                            1,
                            Group::V0(GroupV0 {
                                members: [(identity_3.id(), 1), (identity_4.id(), 1)].into(),
                                required_power: 2,
                            }),
                        ),
                        (
                            2,
                            Group::V0(GroupV0 {
                                members: [(identity.id(), 1), (identity_4.id(), 1)].into(),
                                required_power: 2,
                            }),
                        ),
                    ]
                    .into(),
                ),
                None,
                platform_version,
            );

            let action_id = TokenConfigUpdateTransition::calculate_action_id_with_fields(
                token_id.as_bytes(),
                identity.id().as_bytes(),
                2,
                TokenConfigurationChangeItem::MintingAllowChoosingDestinationAdminGroup(
                    AuthorizedActionTakers::Group(1),
                )
                .u8_item_index(),
            );

            let config_update_transition = BatchTransition::new_token_config_update_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                TokenConfigurationChangeItem::MintingAllowChoosingDestinationAdminGroup(
                    AuthorizedActionTakers::Group(1),
                ),
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

            let config_update_transition_serialized_transition = config_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[config_update_transition_serialized_transition.clone()],
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

            let new_contract = platform
                .drive
                .fetch_contract(
                    contract.id().to_buffer(),
                    None,
                    None,
                    None,
                    platform_version,
                )
                .unwrap()
                .expect("expected to fetch token balance")
                .expect("expected contract");
            let updated_token_config = new_contract
                .contract
                .expected_token_configuration(0)
                .expect("expected token configuration");
            assert_eq!(
                updated_token_config
                    .distribution_rules()
                    .minting_allow_choosing_destination_rules()
                    .admin_action_takers(),
                &AuthorizedActionTakers::Group(0)
            );

            let config_update_transition = BatchTransition::new_token_config_update_transition(
                token_id,
                identity_2.id(),
                contract.id(),
                0,
                TokenConfigurationChangeItem::MintingAllowChoosingDestinationAdminGroup(
                    AuthorizedActionTakers::Group(1),
                ),
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
                &key_2,
                2,
                0,
                &signer_2,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

            let config_update_transition_serialized_transition = config_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[config_update_transition_serialized_transition.clone()],
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

            let new_contract = platform
                .drive
                .fetch_contract(
                    contract.id().to_buffer(),
                    None,
                    None,
                    None,
                    platform_version,
                )
                .unwrap()
                .expect("expected to fetch token balance")
                .expect("expected contract");
            let updated_token_config = new_contract
                .contract
                .expected_token_configuration(0)
                .expect("expected token configuration");
            assert_eq!(
                updated_token_config
                    .distribution_rules()
                    .minting_allow_choosing_destination_rules()
                    .admin_action_takers(),
                &AuthorizedActionTakers::Group(1)
            );

            // The admin group has changed to 1, let's try to do an action with admin group as 0.

            let config_update_transition = BatchTransition::new_token_config_update_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                TokenConfigurationChangeItem::MintingAllowChoosingDestinationControlGroup(
                    AuthorizedActionTakers::Group(2),
                ),
                None,
                Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
                &key,
                3,
                0,
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

            let config_update_transition_serialized_transition = config_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[config_update_transition_serialized_transition.clone()],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // It should fail because the authorized group to do this action is now 1

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

            // Let's try doing this action with group 1

            let action_id = TokenConfigUpdateTransition::calculate_action_id_with_fields(
                token_id.as_bytes(),
                identity_3.id().as_bytes(),
                2,
                TokenConfigurationChangeItem::MintingAllowChoosingDestinationControlGroup(
                    AuthorizedActionTakers::Group(2),
                )
                .u8_item_index(),
            );

            let config_update_transition = BatchTransition::new_token_config_update_transition(
                token_id,
                identity_3.id(),
                contract.id(),
                0,
                TokenConfigurationChangeItem::MintingAllowChoosingDestinationControlGroup(
                    AuthorizedActionTakers::Group(2),
                ),
                None,
                Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(1)),
                &key_3,
                2,
                0,
                &signer_3,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

            let config_update_transition_serialized_transition = config_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[config_update_transition_serialized_transition.clone()],
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

            let new_contract = platform
                .drive
                .fetch_contract(
                    contract.id().to_buffer(),
                    None,
                    None,
                    None,
                    platform_version,
                )
                .unwrap()
                .expect("expected to fetch token balance")
                .expect("expected contract");
            let updated_token_config = new_contract
                .contract
                .expected_token_configuration(0)
                .expect("expected token configuration");
            assert_eq!(
                updated_token_config
                    .distribution_rules()
                    .minting_allow_choosing_destination_rules()
                    .authorized_to_make_change_action_takers(),
                &AuthorizedActionTakers::Group(1)
            );

            let config_update_transition = BatchTransition::new_token_config_update_transition(
                token_id,
                identity_4.id(),
                contract.id(),
                0,
                TokenConfigurationChangeItem::MintingAllowChoosingDestinationControlGroup(
                    AuthorizedActionTakers::Group(2),
                ),
                None,
                Some(
                    GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
                        GroupStateTransitionInfo {
                            group_contract_position: 1,
                            action_id,
                            action_is_proposer: false,
                        },
                    ),
                ),
                &key_4,
                2,
                0,
                &signer_4,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

            let config_update_transition_serialized_transition = config_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[config_update_transition_serialized_transition.clone()],
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

            let new_contract = platform
                .drive
                .fetch_contract(
                    contract.id().to_buffer(),
                    None,
                    None,
                    None,
                    platform_version,
                )
                .unwrap()
                .expect("expected to fetch token balance")
                .expect("expected contract");
            let updated_token_config = new_contract
                .contract
                .expected_token_configuration(0)
                .expect("expected token configuration");
            assert_eq!(
                updated_token_config
                    .distribution_rules()
                    .minting_allow_choosing_destination_rules()
                    .authorized_to_make_change_action_takers(),
                &AuthorizedActionTakers::Group(2)
            );
        }

        #[test]
        fn test_token_config_change_own_admin_group_give_control_power_and_change_admin_back() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49853);

            let platform_state = platform.state.load();

            let (identity, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (identity_2, signer_2, key_2) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (identity_3, signer_3, key_3) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (identity_4, signer_4, key_4) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (identity_5, signer_5, key_5) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                identity.id(),
                Some(|token_configuration: &mut TokenConfiguration| {
                    token_configuration.set_conventions_change_rules(ChangeControlRules::V0(
                        ChangeControlRulesV0 {
                            authorized_to_make_change: AuthorizedActionTakers::Group(0),
                            admin_action_takers: AuthorizedActionTakers::Group(1),
                            changing_authorized_action_takers_to_no_one_allowed: false,
                            changing_admin_action_takers_to_no_one_allowed: false,
                            self_changing_admin_action_takers_allowed: true,
                        },
                    ));
                }),
                None,
                Some(
                    [
                        (
                            0,
                            Group::V0(GroupV0 {
                                members: [(identity.id(), 1), (identity_2.id(), 1)].into(),
                                required_power: 2,
                            }),
                        ),
                        (
                            1,
                            Group::V0(GroupV0 {
                                members: [
                                    (identity_3.id(), 1),
                                    (identity_4.id(), 1),
                                    (identity_5.id(), 1),
                                ]
                                .into(),
                                required_power: 2,
                            }),
                        ),
                    ]
                    .into(),
                ),
                None,
                platform_version,
            );

            let action_id = TokenConfigUpdateTransition::calculate_action_id_with_fields(
                token_id.as_bytes(),
                identity_3.id().as_bytes(),
                2,
                TokenConfigurationChangeItem::ConventionsAdminGroup(AuthorizedActionTakers::Group(
                    0,
                ))
                .u8_item_index(),
            );

            let config_update_transition = BatchTransition::new_token_config_update_transition(
                token_id,
                identity_3.id(),
                contract.id(),
                0,
                TokenConfigurationChangeItem::ConventionsAdminGroup(AuthorizedActionTakers::Group(
                    0,
                )),
                None,
                Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(1)),
                &key_3,
                2,
                0,
                &signer_3,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

            let config_update_transition_serialized_transition = config_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[config_update_transition_serialized_transition.clone()],
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

            let new_contract = platform
                .drive
                .fetch_contract(
                    contract.id().to_buffer(),
                    None,
                    None,
                    None,
                    platform_version,
                )
                .unwrap()
                .expect("expected to fetch token balance")
                .expect("expected contract");
            let updated_token_config = new_contract
                .contract
                .expected_token_configuration(0)
                .expect("expected token configuration");
            assert_eq!(
                updated_token_config
                    .conventions_change_rules()
                    .admin_action_takers(),
                &AuthorizedActionTakers::Group(1)
            );

            let config_update_transition = BatchTransition::new_token_config_update_transition(
                token_id,
                identity_4.id(),
                contract.id(),
                0,
                TokenConfigurationChangeItem::ConventionsAdminGroup(AuthorizedActionTakers::Group(
                    0,
                )),
                None,
                Some(
                    GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
                        GroupStateTransitionInfo {
                            group_contract_position: 1,
                            action_id,
                            action_is_proposer: false,
                        },
                    ),
                ),
                &key_4,
                2,
                0,
                &signer_4,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

            let config_update_transition_serialized_transition = config_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[config_update_transition_serialized_transition.clone()],
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

            let new_contract = platform
                .drive
                .fetch_contract(
                    contract.id().to_buffer(),
                    None,
                    None,
                    None,
                    platform_version,
                )
                .unwrap()
                .expect("expected to fetch token balance")
                .expect("expected contract");
            let updated_token_config = new_contract
                .contract
                .expected_token_configuration(0)
                .expect("expected token configuration");
            assert_eq!(
                updated_token_config
                    .conventions_change_rules()
                    .admin_action_takers(),
                &AuthorizedActionTakers::Group(0)
            );
            assert_eq!(new_contract.contract.version(), 2);

            // 5 is late to the game, admin control has already been transferred, he should get an error
            let config_update_transition = BatchTransition::new_token_config_update_transition(
                token_id,
                identity_5.id(),
                contract.id(),
                0,
                TokenConfigurationChangeItem::ConventionsAdminGroup(AuthorizedActionTakers::Group(
                    0,
                )),
                None,
                Some(
                    GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
                        GroupStateTransitionInfo {
                            group_contract_position: 1,
                            action_id,
                            action_is_proposer: false,
                        },
                    ),
                ),
                &key_5,
                2,
                0,
                &signer_5,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

            let config_update_transition_serialized_transition = config_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[config_update_transition_serialized_transition.clone()],
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
                    ConsensusError::StateError(StateError::GroupActionAlreadyCompletedError(_)),
                    _
                )]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            // Let's try if he proposes it now

            let config_update_transition = BatchTransition::new_token_config_update_transition(
                token_id,
                identity_5.id(),
                contract.id(),
                0,
                TokenConfigurationChangeItem::ConventionsAdminGroup(AuthorizedActionTakers::Group(
                    0,
                )),
                None,
                Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(1)),
                &key_5,
                3,
                0,
                &signer_5,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

            let config_update_transition_serialized_transition = config_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[config_update_transition_serialized_transition.clone()],
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

            // Now let's have Group 0 change the control of the conventions to identity 2 only

            let action_id_change_control =
                TokenConfigUpdateTransition::calculate_action_id_with_fields(
                    token_id.as_bytes(),
                    identity.id().as_bytes(),
                    2,
                    TokenConfigurationChangeItem::ConventionsControlGroup(
                        AuthorizedActionTakers::Identity(identity_2.id()),
                    )
                    .u8_item_index(),
                );

            let config_update_transition = BatchTransition::new_token_config_update_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                TokenConfigurationChangeItem::ConventionsControlGroup(
                    AuthorizedActionTakers::Identity(identity_2.id()),
                ),
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

            let config_update_transition_serialized_transition = config_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[config_update_transition_serialized_transition.clone()],
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

            let config_update_transition = BatchTransition::new_token_config_update_transition(
                token_id,
                identity_2.id(),
                contract.id(),
                0,
                TokenConfigurationChangeItem::ConventionsControlGroup(
                    AuthorizedActionTakers::Identity(identity_2.id()),
                ),
                None,
                Some(
                    GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
                        GroupStateTransitionInfo {
                            group_contract_position: 0,
                            action_id: action_id_change_control,
                            action_is_proposer: false,
                        },
                    ),
                ),
                &key_2,
                2,
                0,
                &signer_2,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

            let config_update_transition_serialized_transition = config_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[config_update_transition_serialized_transition.clone()],
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

            let new_contract = platform
                .drive
                .fetch_contract(
                    contract.id().to_buffer(),
                    None,
                    None,
                    None,
                    platform_version,
                )
                .unwrap()
                .expect("expected to fetch token balance")
                .expect("expected contract");
            let updated_token_config = new_contract
                .contract
                .expected_token_configuration(0)
                .expect("expected token configuration");
            assert_eq!(
                updated_token_config
                    .conventions_change_rules()
                    .authorized_to_make_change_action_takers(),
                &AuthorizedActionTakers::Identity(identity_2.id())
            );
            assert_eq!(new_contract.contract.version(), 3);

            // Now let's have Group 0 hand it back to Group 1

            let action_id_return = TokenConfigUpdateTransition::calculate_action_id_with_fields(
                token_id.as_bytes(),
                identity.id().as_bytes(),
                3,
                TokenConfigurationChangeItem::ConventionsAdminGroup(AuthorizedActionTakers::Group(
                    1,
                ))
                .u8_item_index(),
            );

            let config_update_transition = BatchTransition::new_token_config_update_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                TokenConfigurationChangeItem::ConventionsAdminGroup(AuthorizedActionTakers::Group(
                    1,
                )),
                None,
                Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
                &key,
                3,
                0,
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

            let config_update_transition_serialized_transition = config_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[config_update_transition_serialized_transition.clone()],
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

            let config_update_transition = BatchTransition::new_token_config_update_transition(
                token_id,
                identity_2.id(),
                contract.id(),
                0,
                TokenConfigurationChangeItem::ConventionsAdminGroup(AuthorizedActionTakers::Group(
                    1,
                )),
                None,
                Some(
                    GroupStateTransitionInfoStatus::GroupStateTransitionInfoOtherSigner(
                        GroupStateTransitionInfo {
                            group_contract_position: 0,
                            action_id: action_id_return,
                            action_is_proposer: false,
                        },
                    ),
                ),
                &key_2,
                3,
                0,
                &signer_2,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

            let config_update_transition_serialized_transition = config_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[config_update_transition_serialized_transition.clone()],
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

            let new_contract = platform
                .drive
                .fetch_contract(
                    contract.id().to_buffer(),
                    None,
                    None,
                    None,
                    platform_version,
                )
                .unwrap()
                .expect("expected to fetch token balance")
                .expect("expected contract");
            let updated_token_config = new_contract
                .contract
                .expected_token_configuration(0)
                .expect("expected token configuration");
            assert_eq!(
                updated_token_config
                    .conventions_change_rules()
                    .admin_action_takers(),
                &AuthorizedActionTakers::Group(1)
            );
            assert_eq!(new_contract.contract.version(), 4);

            // Not let's try identity 3 to change the conventions (should fail)

            let config_update_transition = BatchTransition::new_token_config_update_transition(
                token_id,
                identity_3.id(),
                contract.id(),
                0,
                TokenConfigurationChangeItem::Conventions(TokenConfigurationConvention::V0(
                    TokenConfigurationConventionV0 {
                        localizations: [(
                            "en".to_string(),
                            TokenConfigurationLocalizationV0 {
                                should_capitalize: true,
                                singular_form: "garzon".to_string(),
                                plural_form: "garzons".to_string(),
                            }
                            .into(),
                        )]
                        .into(),
                        decimals: 8,
                    },
                )),
                None,
                None,
                &key_3,
                3,
                0,
                &signer_3,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

            let config_update_transition_serialized_transition = config_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[config_update_transition_serialized_transition.clone()],
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

            // Not let's try identity 2 to change the conventions (should succeed)

            let config_update_transition = BatchTransition::new_token_config_update_transition(
                token_id,
                identity_2.id(),
                contract.id(),
                0,
                TokenConfigurationChangeItem::Conventions(TokenConfigurationConvention::V0(
                    TokenConfigurationConventionV0 {
                        localizations: [(
                            "en".to_string(),
                            TokenConfigurationLocalizationV0 {
                                should_capitalize: true,
                                singular_form: "garzon".to_string(),
                                plural_form: "garzons".to_string(),
                            }
                            .into(),
                        )]
                        .into(),
                        decimals: 8,
                    },
                )),
                None,
                None,
                &key_2,
                4,
                0,
                &signer_2,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

            let config_update_transition_serialized_transition = config_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[config_update_transition_serialized_transition.clone()],
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
        }

        #[test]
        fn test_token_config_update_as_group_member_but_group_not_needed() {
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
                    token_configuration.set_max_supply_change_rules(ChangeControlRules::V0(
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

            let config_update_transition = BatchTransition::new_token_config_update_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                TokenConfigurationChangeItem::MaxSupply(Some(1000000)),
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

            let config_update_transition_serialized_transition = config_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[config_update_transition_serialized_transition.clone()],
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

            let contract = platform
                .drive
                .fetch_contract(
                    contract.id().to_buffer(),
                    None,
                    None,
                    None,
                    platform_version,
                )
                .unwrap()
                .expect("expected to fetch token balance")
                .expect("expected contract");
            let updated_token_config = contract
                .contract
                .expected_token_configuration(0)
                .expect("expected token configuration");
            assert_eq!(updated_token_config.max_supply(), None);
        }
    }
}
