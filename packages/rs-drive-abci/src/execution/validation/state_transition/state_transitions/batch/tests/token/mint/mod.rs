use super::*;
mod token_mint_tests {
    use super::*;

    mod token_mint_tests_normal_scenarios {
        use super::*;

        #[tokio::test]
        async fn test_token_mint_by_owner_allowed_sending_to_self() {
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
            assert_eq!(token_balance, Some(101337));
        }

        #[tokio::test]
        async fn test_token_mint_with_public_note() {
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
            assert_eq!(token_balance, Some(101337));
        }

        #[tokio::test]
        async fn test_token_mint_by_owner_can_not_mint_past_max_supply() {
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
                    error: ConsensusError::StateError(StateError::TokenMintPastMaxSupplyError(_)),
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
            assert_eq!(token_balance, Some(100000));
        }

        #[tokio::test]
        async fn test_token_mint_to_exact_max_supply_succeeds() {
            // The max-supply check uses a strict `>` comparison
            // (token_mint_transition_action/state_v0/mod.rs), so minting to *exactly*
            // max_supply must be allowed. base_supply is 100_000, max_supply 1_000_000,
            // so minting 900_000 brings total to exactly 1_000_000.
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

            let mint_transition = BatchTransition::new_token_mint_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                900000,
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
            .await
            .expect("expect to create mint transition");

            let serialized = mint_transition
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

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    identity.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, Some(1000000));

            let total_supply = platform
                .drive
                .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
                .expect("expected to fetch total supply");
            assert_eq!(total_supply, Some(1000000));
        }

        #[tokio::test]
        async fn test_token_mint_one_over_max_supply_fails() {
            // Off-by-one on the other side of the boundary: minting one token past
            // max_supply must be rejected. base_supply 100_000, max_supply 1_000_000,
            // minting 900_001 would bring total to 1_000_001.
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

            let mint_transition = BatchTransition::new_token_mint_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                900001,
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
            .await
            .expect("expect to create mint transition");

            let serialized = mint_transition
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

            // Assert the full error payload — the point of a boundary test is the exact
            // supply math, so we check amount / current_supply / max_supply, not just the
            // variant.
            let results = processing_result.execution_results();
            assert_matches!(
                results.as_slice(),
                [StateTransitionExecutionResult::PaidConsensusError {
                    error: ConsensusError::StateError(StateError::TokenMintPastMaxSupplyError(_)),
                    ..
                }]
            );
            let StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::TokenMintPastMaxSupplyError(err)),
                ..
            } = &results[0]
            else {
                unreachable!("asserted TokenMintPastMaxSupplyError above");
            };
            assert_eq!(err.amount(), 900001);
            assert_eq!(err.current_supply(), 100000);
            assert_eq!(err.max_supply(), 1000000);

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            // Supply and balance must be unchanged (still the base supply).
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

            let total_supply = platform
                .drive
                .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
                .expect("expected to fetch total supply");
            assert_eq!(total_supply, Some(100000));
        }

        #[tokio::test]
        async fn test_token_mint_unbounded_when_max_supply_none() {
            // When max_supply is None there is NO upper-bound check at the ABCI layer
            // (only Drive's i64::MAX guard applies). A very large mint that stays well
            // under i64::MAX must therefore succeed. This documents the intended
            // unbounded behavior. base_supply is 100_000.
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49853);

            let platform_state = platform.state.load();

            let (identity, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            // Default config leaves max_supply == None.
            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                identity.id(),
                None::<fn(&mut TokenConfiguration)>,
                None,
                None,
                None,
                platform_version,
            );

            let mint_amount = 1_000_000_000_000u64;
            let mint_transition = BatchTransition::new_token_mint_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                mint_amount,
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
            .await
            .expect("expect to create mint transition");

            let serialized = mint_transition
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

            let expected = 100000 + mint_amount;
            let total_supply = platform
                .drive
                .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
                .expect("expected to fetch total supply");
            assert_eq!(total_supply, Some(expected));
        }

        #[tokio::test]
        async fn test_token_mint_from_zero_base_supply() {
            // A token created with base_supply == 0 must initialize its total supply
            // entry to 0 (not leave it absent). If the entry were missing, mint
            // validation would fail with CorruptedDriveState. After a mint, supply and
            // balance must reflect the minted amount.
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
                    token_configuration.set_base_supply(0);
                }),
                None,
                None,
                None,
                platform_version,
            );

            // Initial supply must be Some(0), not None.
            let total_supply_before = platform
                .drive
                .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
                .expect("expected to fetch total supply");
            assert_eq!(total_supply_before, Some(0));

            let mint_transition = BatchTransition::new_token_mint_transition(
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
            .await
            .expect("expect to create mint transition");

            let serialized = mint_transition
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

            let token_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    identity.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, Some(1337));

            let total_supply_after = platform
                .drive
                .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
                .expect("expected to fetch total supply");
            assert_eq!(total_supply_after, Some(1337));
        }

        #[tokio::test]
        async fn test_token_mint_from_zero_base_supply_to_exact_max() {
            // Boundary logic must hold starting from a zero base supply: mint to exactly
            // max_supply succeeds, and a further mint of 1 is rejected.
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
                    token_configuration.set_base_supply(0);
                    token_configuration.set_max_supply(Some(1000));
                }),
                None,
                None,
                None,
                platform_version,
            );

            // First mint to exactly max_supply.
            let mint_transition = BatchTransition::new_token_mint_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                1000,
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
            .await
            .expect("expect to create mint transition");

            let serialized = mint_transition
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

            let total_supply = platform
                .drive
                .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
                .expect("expected to fetch total supply");
            assert_eq!(total_supply, Some(1000));

            // Second mint of 1 must be rejected as past max supply.
            let mint_transition_2 = BatchTransition::new_token_mint_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                1,
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
            .expect("expect to create second mint transition");

            let serialized_2 = mint_transition_2
                .serialize_to_bytes()
                .expect("expected to serialize");

            let transaction = platform.drive.grove.start_transaction();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[serialized_2],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            // Assert the full payload: minting 1 when current_supply == max_supply == 1000.
            let results = processing_result.execution_results();
            assert_matches!(
                results.as_slice(),
                [StateTransitionExecutionResult::PaidConsensusError {
                    error: ConsensusError::StateError(StateError::TokenMintPastMaxSupplyError(_)),
                    ..
                }]
            );
            let StateTransitionExecutionResult::PaidConsensusError {
                error: ConsensusError::StateError(StateError::TokenMintPastMaxSupplyError(err)),
                ..
            } = &results[0]
            else {
                unreachable!("asserted TokenMintPastMaxSupplyError above");
            };
            assert_eq!(err.amount(), 1);
            assert_eq!(err.current_supply(), 1000);
            assert_eq!(err.max_supply(), 1000);

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
            assert_eq!(total_supply, Some(1000));
        }

        #[tokio::test]
        async fn test_token_mint_with_max_i64_base_supply_then_overflow_returns_internal_error_without_mutating_supply(
        ) {
            // CHARACTERIZATION TEST (current behavior, not the desired long-term API).
            //
            // A contract may be created with base_supply == i64::MAX (the largest value
            // that passes the Drive sum-item guard). A subsequent mint of 1 would push the
            // supply past i64::MAX. With max_supply == None there is no validation-layer
            // guard, so this is only caught by the low-level Drive checked_add and surfaces
            // as an InternalError (the "corrupted execution" class) rather than a graceful
            // consensus rejection — while leaving supply unmutated.
            //
            // This test pins that current shape. When a validation-layer guard is added
            // (tracked separately), this test SHOULD break: that is the signal to update it
            // from the characterized-current behavior to the new graceful-rejection behavior.
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(49853);

            let platform_state = platform.state.load();

            let (identity, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let base = i64::MAX as u64;
            let (contract, token_id) = create_token_contract_with_owner_identity(
                &mut platform,
                identity.id(),
                Some(move |token_configuration: &mut TokenConfiguration| {
                    token_configuration.set_base_supply(base);
                }),
                None,
                None,
                None,
                platform_version,
            );

            // Creation initializes the supply to i64::MAX.
            let total_supply_before = platform
                .drive
                .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
                .expect("expected to fetch total supply");
            assert_eq!(total_supply_before, Some(base));

            let mint_transition = BatchTransition::new_token_mint_transition(
                token_id,
                identity.id(),
                contract.id(),
                0,
                1,
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
            .await
            .expect("expect to create mint transition");

            let serialized = mint_transition
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

            // Minting 1 past an i64::MAX supply is caught by the Drive sum-item guard
            // (checked_add in add_to_token_total_supply_operations_v0). With
            // max_supply == None there is no graceful consensus-level rejection, so it
            // surfaces as an InternalError carrying the Drive overflow message rather
            // than a clean PaidConsensusError. We assert that concrete shape (not merely
            // "not successful") so the test fails loudly if the surfaced result changes.
            let results = processing_result.execution_results();
            assert_matches!(
                results.as_slice(),
                [StateTransitionExecutionResult::InternalError(_)]
            );
            let StateTransitionExecutionResult::InternalError(message) = &results[0] else {
                unreachable!("asserted InternalError above");
            };
            assert!(
                message.contains("overflow total supply"),
                "expected the Drive overflow guard message, got: {message}"
            );

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            // Supply must be unchanged.
            let total_supply_after = platform
                .drive
                .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
                .expect("expected to fetch total supply");
            assert_eq!(total_supply_after, Some(base));
        }

        // NOTE: the base_supply > max_supply creation gap is documented by a real
        // validation-path test:
        // data_contract_create::tests::tokens::token_errors::
        //   test_data_contract_creation_with_base_supply_over_max_supply_should_cause_error
        // (it runs an actual DataContractCreateTransition, unlike the setup_contract
        // helper used here, which bypasses state-transition validation).

        #[tokio::test]
        async fn test_token_mint_by_owner_allowed_sending_to_other() {
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
                    receiver.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, Some(1337));
        }

        #[tokio::test]
        async fn test_token_mint_sending_to_non_existing_identity_causes_error() {
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
                        StateError::RecipientIdentityDoesNotExistError(_)
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
                    receiver.to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, None);
        }

        #[tokio::test]
        async fn test_token_mint_by_owner_no_destination_causes_error() {
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
                    error: ConsensusError::BasicError(
                        BasicError::DestinationIdentityForTokenMintingNotSetError(_)
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
    }

    mod token_mint_tests_no_recipient_minting {
        use super::*;

        #[tokio::test]
        async fn test_token_mint_by_owned_id_allowed_sending_to_self() {
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
                    error: ConsensusError::BasicError(
                        BasicError::ChoosingTokenMintRecipientNotAllowedError(_)
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
            assert_eq!(token_balance, Some(100000));
        }

        #[tokio::test]
        async fn test_token_mint_by_owned_id_allowed_sending_to_other() {
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
                    error: ConsensusError::BasicError(
                        BasicError::ChoosingTokenMintRecipientNotAllowedError(_)
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
                    receiver.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, None);
        }

        #[tokio::test]
        async fn test_token_mint_by_owned_id_no_destination_causes_error() {
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
                    error: ConsensusError::BasicError(
                        BasicError::DestinationIdentityForTokenMintingNotSetError(_)
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
    }

    mod token_mint_tests_contract_has_recipient {
        use super::*;

        #[tokio::test]
        async fn test_token_mint_by_owned_id_allowed_sending_to_self() {
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
                    error: ConsensusError::BasicError(
                        BasicError::ChoosingTokenMintRecipientNotAllowedError(_)
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
            assert_eq!(token_balance, Some(100000));
        }

        #[tokio::test]
        async fn test_token_mint_by_owned_id_allowed_sending_to_other() {
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
                    error: ConsensusError::BasicError(
                        BasicError::ChoosingTokenMintRecipientNotAllowedError(_)
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
                    receiver.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, None);
        }

        #[tokio::test]
        async fn test_token_mint_by_owned_id_no_set_destination_should_use_contracts() {
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
            assert_eq!(token_balance, Some(101337));
        }
    }

    mod token_mint_tests_authorization_scenarios {
        use super::*;
        use crate::execution::check_tx::CheckTxLevel;
        use crate::platform_types::platform::PlatformRef;
        use dpp::data_contract::accessors::v0::DataContractV0Setters;
        use dpp::data_contract::accessors::v1::{DataContractV1Getters, DataContractV1Setters};
        use dpp::data_contract::associated_token::token_keeps_history_rules::accessors::v0::TokenKeepsHistoryRulesV0Setters;
        use dpp::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
        use dpp::data_contract::change_control_rules::v0::ChangeControlRulesV0;
        use dpp::data_contract::change_control_rules::ChangeControlRules;
        use dpp::data_contract::group::v0::GroupV0;
        use dpp::data_contract::group::Group;
        use dpp::group::group_action_status::GroupActionStatus;
        use dpp::group::{GroupStateTransitionInfo, GroupStateTransitionInfoStatus};
        use dpp::prelude::DataContract;
        use dpp::state_transition::batch_transition::TokenMintTransition;
        use dpp::state_transition::proof_result::StateTransitionProofResult;
        use dpp::tokens::calculate_token_id;
        use drive::drive::Drive;
        use drive::util::test_helpers::setup_contract;

        #[tokio::test]
        async fn test_token_mint_by_owner_sending_to_self_minting_not_allowed() {
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
                    identity.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, Some(100000));
        }

        #[tokio::test]
        async fn test_token_mint_by_owner_sending_to_self_minting_only_allowed_by_group() {
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
                    identity.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch token balance");
            assert_eq!(token_balance, Some(100000));
        }

        #[tokio::test]
        async fn test_token_mint_by_owner_sending_to_self_minting_only_allowed_by_group_enough_member_power(
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
            assert_eq!(token_balance, Some(101337));
        }

        #[tokio::test]
        async fn test_token_mint_by_owner_requires_group_other_member_with_history() {
            test_token_mint_by_owner_requires_group_other_member(true).await;
        }

        #[tokio::test]
        async fn test_token_mint_by_owner_requires_group_other_member_no_history() {
            test_token_mint_by_owner_requires_group_other_member(false).await;
        }

        async fn test_token_mint_by_owner_requires_group_other_member(keeps_minting_history: bool) {
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
            .await
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
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
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
            .map(|(root_hash, outcome)| (root_hash, outcome.into_result()))
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
            .await
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
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
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
            .map(|(root_hash, outcome)| (root_hash, outcome.into_result()))
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

        #[tokio::test]
        async fn test_token_mint_by_owner_requires_group_other_member_keeps_history_with_note() {
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
            .await
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
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
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
            .map(|(root_hash, outcome)| (root_hash, outcome.into_result()))
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
            .await
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
            .await
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
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
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
            .map(|(root_hash, outcome)| (root_hash, outcome.into_result()))
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

        #[tokio::test]
        async fn test_token_mint_by_owner_requires_group_other_member_changes_minting_amount() {
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
            .await
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
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
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
            .map(|(root_hash, outcome)| (root_hash, outcome.into_result()))
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
            .await
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
                [StateTransitionExecutionResult::PaidConsensusError {
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

        #[tokio::test]
        async fn test_token_mint_by_owner_requires_group_resubmitting_causes_error() {
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
            .await
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
            .await
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

        #[tokio::test]
        async fn test_token_mint_by_owner_requires_group_other_member_resubmitting_causes_error() {
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
            .await
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
            .await
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
            .await
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

        #[tokio::test]
        async fn test_token_mint_by_owner_requires_group_other_member_submitting_after_completion_causes_error(
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
            .await
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
            .await
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
            .await
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
                [PaidConsensusError {
                    error: ConsensusError::StateError(
                        StateError::GroupActionAlreadyCompletedError(_)
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

        #[tokio::test]
        async fn test_token_mint_by_owner_requires_group_proposer_not_in_group() {
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
            .await
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

        #[tokio::test]
        async fn test_token_mint_by_owner_requires_group_other_signer_not_part_of_group() {
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
            .await
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
            .await
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

        #[tokio::test]
        async fn test_token_mint_other_signer_going_first_causes_error() {
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
            .await
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
                [PaidConsensusError {
                    error: ConsensusError::StateError(StateError::GroupActionDoesNotExistError(_)),
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

        #[tokio::test]
        async fn test_token_mint_by_owner_does_not_require_group_but_sends_group_info() {
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
            .await
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
        }

        #[tokio::test]
        async fn test_token_mint_confirmation_cannot_change_token_position() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .with_latest_protocol_version()
                .build_with_mock_rpc()
                .set_genesis_state();

            let mut rng = StdRng::seed_from_u64(44017);
            let platform_state = platform.state.load();
            let (identity, signer, key) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
            let (identity_2, signer_2, key_2) =
                setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));

            let data_contract_id = DataContract::generate_data_contract_id_v0(identity.id(), 1);
            let contract = setup_contract(
                &platform.drive,
                "tests/supporting_files/contract/basic-token/basic-token.json",
                Some(data_contract_id.to_buffer()),
                Some(identity.id().to_buffer()),
                Some(|data_contract: &mut DataContract| {
                    data_contract.set_created_at_epoch(Some(0));
                    data_contract.set_created_at(Some(0));
                    data_contract.set_created_at_block_height(Some(0));

                    let second_token = data_contract
                        .expected_token_configuration(0)
                        .expect("expected token configuration")
                        .clone();
                    data_contract
                        .tokens_mut()
                        .expect("expected token map")
                        .insert(1, second_token);

                    for token_position in [0, 1] {
                        data_contract
                            .token_configuration_mut(token_position)
                            .expect("expected token configuration")
                            .set_manual_minting_rules(ChangeControlRules::V0(
                                ChangeControlRulesV0 {
                                    authorized_to_make_change: AuthorizedActionTakers::Group(0),
                                    admin_action_takers: AuthorizedActionTakers::NoOne,
                                    changing_authorized_action_takers_to_no_one_allowed: false,
                                    changing_admin_action_takers_to_no_one_allowed: false,
                                    self_changing_admin_action_takers_allowed: false,
                                },
                            ));
                    }

                    data_contract.set_groups(
                        [(
                            0,
                            Group::V0(GroupV0 {
                                members: [(identity.id(), 1), (identity_2.id(), 1)].into(),
                                required_power: 2,
                            }),
                        )]
                        .into(),
                    );
                }),
                None,
                Some(platform_version),
            );
            let token_id_0: Identifier = calculate_token_id(data_contract_id.as_bytes(), 0).into();
            let token_id_1: Identifier = calculate_token_id(data_contract_id.as_bytes(), 1).into();

            let proposal = BatchTransition::new_token_mint_transition(
                token_id_0,
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
            .await
            .expect("expect to create mint proposal");

            let transaction = platform.drive.grove.start_transaction();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[proposal
                        .serialize_to_bytes()
                        .expect("expected serialized proposal")],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process proposal");
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution { .. }]
            );
            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit proposal");

            let action_id = TokenMintTransition::calculate_action_id_with_fields(
                token_id_0.as_bytes(),
                identity.id().as_bytes(),
                2,
                1337,
            );
            let confirmation = BatchTransition::new_token_mint_transition(
                token_id_1,
                identity_2.id(),
                contract.id(),
                1,
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
                &key_2,
                2,
                0,
                &signer_2,
                platform_version,
                None,
            )
            .await
            .expect("expect to create mint confirmation");

            let transaction = platform.drive.grove.start_transaction();
            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &[confirmation
                        .serialize_to_bytes()
                        .expect("expected serialized confirmation")],
                    &platform_state,
                    &BlockInfo::default(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process confirmation");
            assert_matches!(
                processing_result.execution_results().as_slice(),
                [PaidConsensusError {
                    error: ConsensusError::StateError(
                        StateError::ModificationOfGroupActionMainParametersNotPermittedError(_)
                    ),
                    ..
                }]
            );

            let target_balance = platform
                .drive
                .fetch_identity_token_balance(
                    token_id_1.to_buffer(),
                    identity.id().to_buffer(),
                    Some(&transaction),
                    platform_version,
                )
                .expect("expected target token balance");
            assert_eq!(target_balance, Some(100_000));
        }
    }
}
