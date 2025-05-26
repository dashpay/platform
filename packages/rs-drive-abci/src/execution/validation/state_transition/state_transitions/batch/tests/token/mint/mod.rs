use super::*;
mod token_mint_tests {
    use super::*;

    mod token_mint_tests_normal_scenarios {
        use super::*;

        #[test]
        fn test_token_mint_by_owner_allowed_sending_to_self() {
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

            let documents_batch_create_transition = BatchTransition::new_token_mint_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                1337,
                Some(identity.id()),
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
            assert_eq!(token_balance, Some(101337));
        }

        #[test]
        fn test_token_mint_with_public_note() {
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

            let documents_batch_create_transition = BatchTransition::new_token_mint_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                1337,
                Some(identity.id()),
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
            assert_eq!(token_balance, Some(101337));
        }

        #[test]
        fn test_token_mint_by_owner_can_not_mint_past_max_supply() {
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
                    token_configuration.set_max_supply(Some(1000000));
                }),
                None,
                None,
                None,
                platform_version,
            );

            let documents_batch_create_transition = BatchTransition::new_token_mint_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                2000000,
                Some(identity.id()),
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
                    ConsensusError::StateError(StateError::TokenMintPastMaxSupplyError(_)),
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
        }

        #[test]
        fn test_token_mint_by_owner_allowed_sending_to_other() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49853);

            let platform_state = platform.state.load();

            let (identity, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (receiver, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                identity.id(),
                None::<fn(&mut TokenConfiguration)>,
                None,
                None,
                None,
                platform_version,
            );

            let documents_batch_create_transition = BatchTransition::new_token_mint_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                1337,
                Some(receiver.id()),
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
                    receiver.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, Some(1337));
        }

        #[test]
        fn test_token_mint_sending_to_non_existing_identity_causes_error() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49853);

            let platform_state = platform.state.load();

            let (identity, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let receiver = Identifier::random_with_rng(&mut rng);

            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                identity.id(),
                None::<fn(&mut TokenConfiguration)>,
                None,
                None,
                None,
                platform_version,
            );

            let documents_batch_create_transition = BatchTransition::new_token_mint_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                1337,
                Some(receiver),
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
                    ConsensusError::StateError(StateError::RecipientIdentityDoesNotExistError(_)),
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
                    receiver.to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, None);
        }

        #[test]
        fn test_token_mint_by_owner_no_destination_causes_error() {
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

            let documents_batch_create_transition = BatchTransition::new_token_mint_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                1337,
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
                    ConsensusError::BasicError(
                        BasicError::DestinationIdentityForTokenMintingNotSetError(_)
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
    }

    mod token_mint_tests_no_recipient_minting {
        use super::*;

        #[test]
        fn test_token_mint_by_owned_id_allowed_sending_to_self() {
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
                    token_configuration
                        .distribution_rules_mut()
                        .set_minting_allow_choosing_destination(false);
                }),
                None,
                None,
                None,
                platform_version,
            );

            let documents_batch_create_transition = BatchTransition::new_token_mint_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                1337,
                Some(identity.id()),
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
                    ConsensusError::BasicError(
                        BasicError::ChoosingTokenMintRecipientNotAllowedError(_)
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
        }

        #[test]
        fn test_token_mint_by_owned_id_allowed_sending_to_other() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49853);

            let platform_state = platform.state.load();

            let (identity, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (receiver, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                identity.id(),
                Some(|token_configuration: &mut TokenConfiguration| {
                    token_configuration
                        .distribution_rules_mut()
                        .set_minting_allow_choosing_destination(false);
                }),
                None,
                None,
                None,
                platform_version,
            );

            let documents_batch_create_transition = BatchTransition::new_token_mint_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                1337,
                Some(receiver.id()),
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
                    ConsensusError::BasicError(
                        BasicError::ChoosingTokenMintRecipientNotAllowedError(_)
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
                    receiver.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, None);
        }

        #[test]
        fn test_token_mint_by_owned_id_no_destination_causes_error() {
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
                    token_configuration
                        .distribution_rules_mut()
                        .set_minting_allow_choosing_destination(false);
                }),
                None,
                None,
                None,
                platform_version,
            );

            let documents_batch_create_transition = BatchTransition::new_token_mint_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                1337,
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
                    ConsensusError::BasicError(
                        BasicError::DestinationIdentityForTokenMintingNotSetError(_)
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
    }

    mod token_mint_tests_contract_has_recipient {
        use super::*;

        #[test]
        fn test_token_mint_by_owned_id_allowed_sending_to_self() {
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
                    token_configuration
                        .distribution_rules_mut()
                        .set_minting_allow_choosing_destination(false);
                    token_configuration
                        .distribution_rules_mut()
                        .set_new_tokens_destination_identity(Some(identity.id()));
                }),
                None,
                None,
                None,
                platform_version,
            );

            let documents_batch_create_transition = BatchTransition::new_token_mint_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                1337,
                Some(identity.id()),
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
                    ConsensusError::BasicError(
                        BasicError::ChoosingTokenMintRecipientNotAllowedError(_)
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
        }

        #[test]
        fn test_token_mint_by_owned_id_allowed_sending_to_other() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49853);

            let platform_state = platform.state.load();

            let (identity, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (receiver, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                identity.id(),
                Some(|token_configuration: &mut TokenConfiguration| {
                    token_configuration
                        .distribution_rules_mut()
                        .set_minting_allow_choosing_destination(false);
                    token_configuration
                        .distribution_rules_mut()
                        .set_new_tokens_destination_identity(Some(identity.id()));
                }),
                None,
                None,
                None,
                platform_version,
            );

            let documents_batch_create_transition = BatchTransition::new_token_mint_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                1337,
                Some(receiver.id()),
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
                    ConsensusError::BasicError(
                        BasicError::ChoosingTokenMintRecipientNotAllowedError(_)
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
                    receiver.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, None);
        }

        #[test]
        fn test_token_mint_by_owned_id_no_set_destination_should_use_contracts() {
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
                    token_configuration
                        .distribution_rules_mut()
                        .set_minting_allow_choosing_destination(false);
                    token_configuration
                        .distribution_rules_mut()
                        .set_new_tokens_destination_identity(Some(identity.id()));
                }),
                None,
                None,
                None,
                platform_version,
            );

            let documents_batch_create_transition = BatchTransition::new_token_mint_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                1337,
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
            assert_eq!(token_balance, Some(101337));
        }
    }

    mod token_mint_tests_authorization_scenarios {
        use super::*;
        use crate::execution::check_tx::CheckTxLevel;
        use crate::platform_types::platform::PlatformRef;
        use dpp::data_contract::associated_token::token_keeps_history_rules::accessors::v0::TokenKeepsHistoryRulesV0Setters;
        use dpp::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
        use dpp::data_contract::change_control_rules::v0::ChangeControlRulesV0;
        use dpp::data_contract::change_control_rules::ChangeControlRules;
        use dpp::data_contract::group::v0::GroupV0;
        use dpp::data_contract::group::Group;
        use dpp::group::group_action_status::GroupActionStatus;
        use dpp::group::{GroupStateTransitionInfo, GroupStateTransitionInfoStatus};
        use dpp::state_transition::batch_transition::TokenMintTransition;
        use dpp::state_transition::proof_result::StateTransitionProofResult;
        use drive::drive::Drive;

        #[test]
        fn test_token_mint_by_owner_sending_to_self_minting_not_allowed() {
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
                    token_configuration.set_manual_minting_rules(ChangeControlRules::V0(
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

            let documents_batch_create_transition = BatchTransition::new_token_mint_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                1337,
                Some(identity.id()),
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
                    identity.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, Some(100000));
        }

        #[test]
        fn test_token_mint_by_owner_sending_to_self_minting_only_allowed_by_group() {
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
                            members: [(identity.id(), 5), (identity_2.id(), 5)].into(),
                            required_power: 10,
                        }),
                    )]
                    .into(),
                ),
                None,
                platform_version,
            );

            let documents_batch_create_transition = BatchTransition::new_token_mint_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                1337,
                Some(identity.id()),
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
                    identity.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, Some(100000));
        }

        #[test]
        fn test_token_mint_by_owner_sending_to_self_minting_only_allowed_by_group_enough_member_power(
        ) {
            // We are using a group, but our member alone has enough power in the group to do the action
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
                            members: [(identity.id(), 5), (identity_2.id(), 1)].into(),
                            required_power: 5,
                        }),
                    )]
                    .into(),
                ),
                None,
                platform_version,
            );

            let documents_batch_create_transition = BatchTransition::new_token_mint_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                1337,
                Some(identity.id()),
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
            assert_eq!(token_balance, Some(101337));
        }

        #[test]
        fn test_token_mint_by_owner_requires_group_other_member_with_history() {
            test_token_mint_by_owner_requires_group_other_member(true);
        }

        #[test]
        fn test_token_mint_by_owner_requires_group_other_member_no_history() {
            test_token_mint_by_owner_requires_group_other_member(false);
        }

        fn test_token_mint_by_owner_requires_group_other_member(keeps_minting_history: bool) {
            // We are using a group, and two members need to sign for the event to happen
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
                    token_configuration
                        .keeps_history_mut()
                        .set_keeps_minting_history(keeps_minting_history);
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
                            members: [(identity.id(), 1), (identity_2.id(), 1)].into(),
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

            // Let's verify the proof of the state transition

            let proof = platform
                .drive
                .prove_state_transition(&token_mint_transition, None, platform_version)
                .expect("expect to prove state transition");

            let (_root_hash, result) = Drive::verify_state_transition_was_executed_with_proof(
                &token_mint_transition,
                &BlockInfo::default(),
                proof.data.as_ref().expect("expected data"),
                &|_| Ok(Some(contract.clone().into())),
                platform_version,
            )
            .unwrap_or_else(|_| {
                panic!(
                    "expect to verify state transition proof {}",
                    hex::encode(proof.data.expect("expected data"))
                )
            });

            if keeps_minting_history {
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
                    StateTransitionProofResult::VerifiedTokenGroupActionWithTokenBalance(power, status, balance) => {
                        assert_eq!(power, 1);
                        assert_eq!(status, GroupActionStatus::ActionActive);
                        assert_eq!(balance, Some(100000));
                    }
                );
            }

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

            // Now we need to get the second identity to also sign it
            let action_id = TokenMintTransition::calculate_action_id_with_fields(
                token_id.as_bytes(),
                identity.id().as_bytes(),
                2,
                1337,
            );
            let confirm_token_mint_transition = BatchTransition::new_token_mint_transition(
                token_id,
                identity_2.id(),
                contract.id(),
                0,
                1337,
                Some(identity.id()),
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
            .expect("expect to create documents batch transition");

            let confirm_token_mint_serialized_transition = confirm_token_mint_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[confirm_token_mint_serialized_transition.clone()],
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

            // Let's verify the proof of the state transition

            let proof = platform
                .drive
                .prove_state_transition(&confirm_token_mint_transition, None, platform_version)
                .expect("expect to prove state transition");

            let (_root_hash, result) = Drive::verify_state_transition_was_executed_with_proof(
                &confirm_token_mint_transition,
                &BlockInfo::default(),
                proof.data.as_ref().expect("expected data"),
                &|_| Ok(Some(contract.clone().into())),
                platform_version,
            )
            .unwrap_or_else(|_| {
                panic!(
                    "expect to verify state transition proof {}",
                    hex::encode(proof.data.expect("expected data"))
                )
            });

            if keeps_minting_history {
                assert_matches!(
                    result,
                    StateTransitionProofResult::VerifiedTokenGroupActionWithDocument(power, doc) => {
                        assert_eq!(power, 2);
                        assert_eq!(doc.expect("expected to get doc").properties().get_u64("amount"), Ok(1337));
                    }
                );
            } else {
                assert_matches!(
                    result,
                    StateTransitionProofResult::VerifiedTokenGroupActionWithTokenBalance(power, status, balance) => {
                        assert_eq!(power, 2);
                        assert_eq!(status, GroupActionStatus::ActionClosed);
                        assert_eq!(balance, Some(101337));
                    }
                );
            }

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    identity.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, Some(101337));

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    identity_2.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, None);
        }

        #[test]
        fn test_token_mint_by_owner_requires_group_other_member_keeps_history_with_note() {
            // We are using a group, and two members need to sign for the event to happen
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
                    token_configuration
                        .keeps_history_mut()
                        .set_keeps_minting_history(true);
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
                            members: [(identity.id(), 1), (identity_2.id(), 1)].into(),
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
                Some("initial note".to_string()),
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

            // Let's verify the proof of the state transition

            let proof = platform
                .drive
                .prove_state_transition(&token_mint_transition, None, platform_version)
                .expect("expect to prove state transition");

            let (_root_hash, result) = Drive::verify_state_transition_was_executed_with_proof(
                &token_mint_transition,
                &BlockInfo::default(),
                proof.data.as_ref().expect("expected data"),
                &|_| Ok(Some(contract.clone().into())),
                platform_version,
            )
            .unwrap_or_else(|_| {
                panic!(
                    "expect to verify state transition proof {}",
                    hex::encode(proof.data.expect("expected data"))
                )
            });
            assert_matches!(
                result,
                StateTransitionProofResult::VerifiedTokenGroupActionWithDocument(power, doc) => {
                    assert_eq!(power, 1);
                    assert_eq!(doc, None);
                }
            );

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

            // Now we need to get the second identity to also sign it
            let action_id = TokenMintTransition::calculate_action_id_with_fields(
                token_id.as_bytes(),
                identity.id().as_bytes(),
                2,
                1337,
            );

            // with a note should fail

            let confirm_token_mint_transition = BatchTransition::new_token_mint_transition(
                token_id,
                identity_2.id(),
                contract.id(),
                0,
                1337,
                Some(identity.id()),
                Some("another note should fail".to_string()),
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
            .expect("expect to create documents batch transition");

            let confirm_token_mint_serialized_transition = confirm_token_mint_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[confirm_token_mint_serialized_transition.clone()],
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
                    ConsensusError::BasicError(BasicError::TokenNoteOnlyAllowedWhenProposerError(
                        _
                    ))
                )]
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            // now let's try with no note

            let confirm_token_mint_transition = BatchTransition::new_token_mint_transition(
                token_id,
                identity_2.id(),
                contract.id(),
                0,
                1337,
                Some(identity.id()),
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
                3,
                0,
                &signer2,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

            let confirm_token_mint_serialized_transition = confirm_token_mint_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[confirm_token_mint_serialized_transition.clone()],
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

            // Let's verify the proof of the state transition

            let proof = platform
                .drive
                .prove_state_transition(&confirm_token_mint_transition, None, platform_version)
                .expect("expect to prove state transition");

            let (_root_hash, result) = Drive::verify_state_transition_was_executed_with_proof(
                &confirm_token_mint_transition,
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
                    assert_eq!(doc.as_ref().expect("expected to get doc").properties().get_u64("amount"), Ok(1337));
                    assert_eq!(doc.expect("expected to get doc").properties().get_string("note"), Ok("initial note".to_string()));
                }
            );

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    identity.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, Some(101337));

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    identity_2.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, None);
        }

        #[test]
        fn test_token_mint_by_owner_requires_group_other_member_changes_minting_amount() {
            // We are using a group, and two members need to sign for the event to happen
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
                    token_configuration
                        .keeps_history_mut()
                        .set_keeps_minting_history(true);
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
                            members: [(identity.id(), 1), (identity_2.id(), 1)].into(),
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
                Some("initial note".to_string()),
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

            // Let's verify the proof of the state transition

            let proof = platform
                .drive
                .prove_state_transition(&token_mint_transition, None, platform_version)
                .expect("expect to prove state transition");

            let (_root_hash, result) = Drive::verify_state_transition_was_executed_with_proof(
                &token_mint_transition,
                &BlockInfo::default(),
                proof.data.as_ref().expect("expected data"),
                &|_| Ok(Some(contract.clone().into())),
                platform_version,
            )
            .unwrap_or_else(|_| {
                panic!(
                    "expect to verify state transition proof {}",
                    hex::encode(proof.data.expect("expected data"))
                )
            });
            assert_matches!(
                result,
                StateTransitionProofResult::VerifiedTokenGroupActionWithDocument(power, doc) => {
                    assert_eq!(power, 1);
                    assert_eq!(doc, None);
                }
            );

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

            // Now we need to get the second identity to also sign it
            let action_id = TokenMintTransition::calculate_action_id_with_fields(
                token_id.as_bytes(),
                identity.id().as_bytes(),
                2,
                1337,
            );

            let confirm_token_mint_transition = BatchTransition::new_token_mint_transition(
                token_id,
                identity_2.id(),
                contract.id(),
                0,
                99999,
                Some(identity.id()),
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
            .expect("expect to create documents batch transition");

            let confirm_token_mint_serialized_transition = confirm_token_mint_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[confirm_token_mint_serialized_transition.clone()],
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
                    identity_2.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, None);
        }

        #[test]
        fn test_token_mint_by_owner_requires_group_resubmitting_causes_error() {
            // We are using a group, and two members need to sign for the event to happen
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
                            members: [(identity.id(), 1), (identity_2.id(), 1)].into(),
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

            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &platform_state,
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };

            let validation_result = platform
                .check_tx(
                    &token_mint_serialized_transition,
                    CheckTxLevel::FirstTimeCheck,
                    &platform_ref,
                    platform_version,
                )
                .expect("expected to be able to check tx");

            assert_eq!(validation_result.errors.as_slice(), &[]);

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
                    identity.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, Some(100000));

            // Now we need to get the second identity to also sign it, but we are going to resubmit with first
            // This will create an error
            let action_id = TokenMintTransition::calculate_action_id_with_fields(
                token_id.as_bytes(),
                identity.id().as_bytes(),
                2,
                1337,
            );
            let confirm_token_mint_transition = BatchTransition::new_token_mint_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                1337,
                Some(identity.id()),
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
                &key,
                3,
                0,
                &signer,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

            let confirm_token_mint_serialized_transition = confirm_token_mint_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let validation_result = platform
                .check_tx(
                    &confirm_token_mint_serialized_transition,
                    CheckTxLevel::FirstTimeCheck,
                    &platform_ref,
                    platform_version,
                )
                .expect("expected to be able to check tx");

            assert_eq!(validation_result.errors.as_slice(), &[]);

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[confirm_token_mint_serialized_transition.clone()],
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
                        StateError::GroupActionAlreadySignedByIdentityError(_)
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
                    identity_2.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, None);
        }

        #[test]
        fn test_token_mint_by_owner_requires_group_other_member_resubmitting_causes_error() {
            // We are using a group, and two members need to sign for the event to happen
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

            let (identity_3, _, _) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

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
                            members: [
                                (identity.id(), 1),
                                (identity_2.id(), 1),
                                (identity_3.id(), 1),
                            ]
                            .into(),
                            required_power: 3,
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

            // Now we need to get the second identity to also sign it
            let action_id = TokenMintTransition::calculate_action_id_with_fields(
                token_id.as_bytes(),
                identity.id().as_bytes(),
                2,
                1337,
            );
            let confirm_token_mint_transition = BatchTransition::new_token_mint_transition(
                token_id,
                identity_2.id(),
                contract.id(),
                0,
                1337,
                Some(identity.id()),
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
            .expect("expect to create documents batch transition");

            let confirm_token_mint_serialized_transition = confirm_token_mint_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[confirm_token_mint_serialized_transition.clone()],
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
            assert_eq!(token_balance, Some(100000));

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    identity_2.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, None);

            // Now we need to get the second identity to sign it again to cause the error
            let confirm_token_mint_transition = BatchTransition::new_token_mint_transition(
                token_id,
                identity_2.id(),
                contract.id(),
                0,
                1337,
                Some(identity.id()),
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
                3,
                0,
                &signer2,
                platform_version,
                None,
            )
            .expect("expect to create documents batch transition");

            let confirm_token_mint_serialized_transition = confirm_token_mint_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[confirm_token_mint_serialized_transition.clone()],
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
                        StateError::GroupActionAlreadySignedByIdentityError(_)
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
                    identity_2.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, None);
        }

        #[test]
        fn test_token_mint_by_owner_requires_group_other_member_submitting_after_completion_causes_error(
        ) {
            // We are using a group, and two members need to sign for the event to happen
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

            let (identity_3, signer3, key3) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

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
                            members: [
                                (identity.id(), 1),
                                (identity_2.id(), 1),
                                (identity_3.id(), 1),
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

            // Now we need to get the second identity to also sign it
            let action_id = TokenMintTransition::calculate_action_id_with_fields(
                token_id.as_bytes(),
                identity.id().as_bytes(),
                2,
                1337,
            );
            let confirm_token_mint_transition = BatchTransition::new_token_mint_transition(
                token_id,
                identity_2.id(),
                contract.id(),
                0,
                1337,
                Some(identity.id()),
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
            .expect("expect to create documents batch transition");

            let confirm_token_mint_serialized_transition = confirm_token_mint_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[confirm_token_mint_serialized_transition.clone()],
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
            assert_eq!(token_balance, Some(101337));

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    identity_2.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, None);

            // Now we need to get the second identity to sign it again to cause the error
            let confirm_token_mint_transition = BatchTransition::new_token_mint_transition(
                token_id,
                identity_3.id(),
                contract.id(),
                0,
                1337,
                Some(identity.id()),
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
            .expect("expect to create documents batch transition");

            let confirm_token_mint_serialized_transition = confirm_token_mint_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let platform_ref = PlatformRef {
                drive: &platform.drive,
                state: &platform_state,
                config: &platform.config,
                core_rpc: &platform.core_rpc,
            };

            let validation_result = platform
                .check_tx(
                    &confirm_token_mint_serialized_transition,
                    CheckTxLevel::FirstTimeCheck,
                    &platform_ref,
                    platform_version,
                )
                .expect("expected to be able to check tx");

            assert_eq!(validation_result.errors.as_slice(), &[]);

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[confirm_token_mint_serialized_transition.clone()],
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

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    identity.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, Some(101337));

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    identity_2.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, None);
        }

        #[test]
        fn test_token_mint_by_owner_requires_group_proposer_not_in_group() {
            // We are using a group, and two members need to sign for the event to happen
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

            let (identity_3, _, _) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

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
                            members: [(identity_3.id(), 1), (identity_2.id(), 1)].into(),
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
                [PaidConsensusError(
                    ConsensusError::StateError(StateError::IdentityNotMemberOfGroupError(_)),
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
        }

        #[test]
        fn test_token_mint_by_owner_requires_group_other_signer_not_part_of_group() {
            // We are using a group, and two members need to sign for the event to happen
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

            let (identity_3, _, _) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

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
                            members: [(identity.id(), 1), (identity_3.id(), 1)].into(),
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

            // Now we need to get the second identity to also sign it
            let action_id = TokenMintTransition::calculate_action_id_with_fields(
                token_id.as_bytes(),
                identity.id().as_bytes(),
                2,
                1337,
            );
            let confirm_token_mint_transition = BatchTransition::new_token_mint_transition(
                token_id,
                identity_2.id(),
                contract.id(),
                0,
                1337,
                Some(identity.id()),
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
            .expect("expect to create documents batch transition");

            let confirm_token_mint_serialized_transition = confirm_token_mint_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[confirm_token_mint_serialized_transition.clone()],
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
                    ConsensusError::StateError(StateError::IdentityNotMemberOfGroupError(_)),
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
                    identity_2.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, None);
        }

        #[test]
        fn test_token_mint_other_signer_going_first_causes_error() {
            // We are using a group, and the second member gets a bit hasty and signs first
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49853);

            let platform_state = platform.state.load();

            let (identity, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let (identity_2, signer2, key2) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

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
                            members: [(identity.id(), 1), (identity_2.id(), 1)].into(),
                            required_power: 2,
                        }),
                    )]
                    .into(),
                ),
                None,
                platform_version,
            );

            // The second identity to also sign it
            let action_id = TokenMintTransition::calculate_action_id_with_fields(
                token_id.as_bytes(),
                identity.id().as_bytes(),
                2,
                1337,
            );
            let confirm_token_mint_transition = BatchTransition::new_token_mint_transition(
                token_id,
                identity_2.id(),
                contract.id(),
                0,
                1337,
                Some(identity.id()),
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
            .expect("expect to create documents batch transition");

            let confirm_token_mint_serialized_transition = confirm_token_mint_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[confirm_token_mint_serialized_transition.clone()],
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
                    ConsensusError::StateError(StateError::GroupActionDoesNotExistError(_)),
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
                    identity_2.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, None);
        }

        #[test]
        fn test_token_mint_by_owner_does_not_require_group_but_sends_group_info() {
            // We are using a group, and two members need to sign for the event to happen
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
                    token_configuration
                        .keeps_history_mut()
                        .set_keeps_minting_history(true);
                    token_configuration.set_manual_minting_rules(ChangeControlRules::V0(
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
        }
    }
}
