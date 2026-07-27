use super::*;

mod token_destroy_frozen_funds_tests {
    use super::*;
    use crate::execution::validation::state_transition::tests::process_test_state_transition;
    use crate::platform_types::platform_state::PlatformState;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::tokens::info::v0::IdentityTokenInfoV0Accessors;

    #[tokio::test]
    async fn test_token_destroy_frozen_funds_success() {
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
                token_configuration.set_destroy_frozen_funds_rules(ChangeControlRules::V0(
                    ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                        admin_action_takers: AuthorizedActionTakers::NoOne,
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_admin_action_takers_to_no_one_allowed: false,
                        self_changing_admin_action_takers_allowed: false,
                    },
                ));
                token_configuration.set_freeze_rules(ChangeControlRules::V0(
                    ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                        admin_action_takers: AuthorizedActionTakers::NoOne,
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_admin_action_takers_to_no_one_allowed: false,
                        self_changing_admin_action_takers_allowed: false,
                    },
                ));
                token_configuration.set_manual_minting_rules(ChangeControlRules::V0(
                    ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                        admin_action_takers: AuthorizedActionTakers::NoOne,
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_admin_action_takers_to_no_one_allowed: false,
                        self_changing_admin_action_takers_allowed: false,
                    },
                ));
                token_configuration
                    .distribution_rules_mut()
                    .set_minting_allow_choosing_destination(true);
            }),
            None,
            None,
            None,
            platform_version,
        );

        // Mint tokens to identity_2
        let mint_transition = BatchTransition::new_token_mint_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            5000,
            Some(identity_2.id()),
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
        .expect("expected to create mint transition");

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

        // Verify identity_2 has the minted tokens
        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity_2.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        assert_eq!(token_balance, Some(5000));

        // Freeze identity_2's token account
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
        .expect("expected to create freeze transition");

        let serialized = freeze_transition
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

        // Verify identity_2 is frozen
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

        // Destroy the frozen funds
        let destroy_transition = BatchTransition::new_token_destroy_frozen_funds_transition(
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
        .expect("expected to create destroy frozen funds transition");

        let serialized = destroy_transition
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

        // Verify the frozen funds were destroyed (balance should be 0)
        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity_2.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        assert_eq!(token_balance, Some(0));

        // Verify identity_2 is still frozen
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

    #[tokio::test]
    async fn test_token_destroy_frozen_funds_on_unfrozen_account_should_fail() {
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
                token_configuration.set_destroy_frozen_funds_rules(ChangeControlRules::V0(
                    ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                        admin_action_takers: AuthorizedActionTakers::NoOne,
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_admin_action_takers_to_no_one_allowed: false,
                        self_changing_admin_action_takers_allowed: false,
                    },
                ));
                // We also need mint + distribution rules to give identity_2 tokens
                token_configuration.set_manual_minting_rules(ChangeControlRules::V0(
                    ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                        admin_action_takers: AuthorizedActionTakers::NoOne,
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_admin_action_takers_to_no_one_allowed: false,
                        self_changing_admin_action_takers_allowed: false,
                    },
                ));
                token_configuration
                    .distribution_rules_mut()
                    .set_minting_allow_choosing_destination(true);
            }),
            None,
            None,
            None,
            platform_version,
        );

        // Mint tokens to identity_2 so they have a token info record (but not frozen)
        let mint_transition = BatchTransition::new_token_mint_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            5000,
            Some(identity_2.id()),
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
        .expect("expected to create mint transition");

        let serialized = mint_transition
            .serialize_to_bytes()
            .expect("expected to serialize");

        let transaction = platform.drive.grove.start_transaction();

        platform
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

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        // Try to destroy funds of identity that is not frozen
        let destroy_transition = BatchTransition::new_token_destroy_frozen_funds_transition(
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
        .expect("expected to create destroy frozen funds transition");

        let serialized = destroy_transition
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
                error: ConsensusError::StateError(StateError::IdentityTokenAccountNotFrozenError(
                    _
                )),
                ..
            }]
        );
    }

    #[tokio::test]
    async fn test_token_destroy_frozen_funds_to_zero_total_supply() {
        // Destroying the frozen funds of the sole holder must bring total_supply to
        // exactly 0. Existing destroy tests never assert total supply. base_supply is 0
        // and the entire supply (5000) is minted to identity_2, so identity_2's frozen
        // balance equals the whole supply.
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
                token_configuration.set_base_supply(0);
                token_configuration.set_destroy_frozen_funds_rules(ChangeControlRules::V0(
                    ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                        admin_action_takers: AuthorizedActionTakers::NoOne,
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_admin_action_takers_to_no_one_allowed: false,
                        self_changing_admin_action_takers_allowed: false,
                    },
                ));
                token_configuration.set_freeze_rules(ChangeControlRules::V0(
                    ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                        admin_action_takers: AuthorizedActionTakers::NoOne,
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_admin_action_takers_to_no_one_allowed: false,
                        self_changing_admin_action_takers_allowed: false,
                    },
                ));
                token_configuration.set_manual_minting_rules(ChangeControlRules::V0(
                    ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                        admin_action_takers: AuthorizedActionTakers::NoOne,
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_admin_action_takers_to_no_one_allowed: false,
                        self_changing_admin_action_takers_allowed: false,
                    },
                ));
                token_configuration
                    .distribution_rules_mut()
                    .set_minting_allow_choosing_destination(true);
            }),
            None,
            None,
            None,
            platform_version,
        );

        // With base_supply 0, total supply starts at 0.
        let total_supply_before = platform
            .drive
            .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");
        assert_eq!(total_supply_before, Some(0));

        // Mint the entire supply to identity_2.
        let mint_transition = BatchTransition::new_token_mint_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            5000,
            Some(identity_2.id()),
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
        .expect("expected to create mint transition");

        process_and_commit_success(
            &mut platform,
            &platform_state,
            mint_transition,
            platform_version,
        );

        let total_supply = platform
            .drive
            .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");
        assert_eq!(total_supply, Some(5000));

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
        .expect("expected to create freeze transition");

        process_and_commit_success(
            &mut platform,
            &platform_state,
            freeze_transition,
            platform_version,
        );

        // Destroy identity_2's frozen funds.
        let destroy_transition = BatchTransition::new_token_destroy_frozen_funds_transition(
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
        .expect("expected to create destroy frozen funds transition");

        process_and_commit_success(
            &mut platform,
            &platform_state,
            destroy_transition,
            platform_version,
        );

        // identity_2 balance is now 0 and total supply is 0.
        let balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity_2.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        assert_eq!(balance, Some(0));

        let total_supply = platform
            .drive
            .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");
        assert_eq!(total_supply, Some(0));

        // identity_2 is still frozen after destruction.
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

    #[tokio::test]
    async fn test_token_destroy_frozen_funds_decrements_total_supply() {
        // Destroying a frozen holder's funds must decrement total_supply by exactly the
        // destroyed amount, leaving other holders untouched. base_supply 0; mint 5000 to
        // identity_2 and 3000 to the owner (total 8000); destroying identity_2's 5000
        // must leave total supply at 3000.
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
                token_configuration.set_base_supply(0);
                token_configuration.set_destroy_frozen_funds_rules(ChangeControlRules::V0(
                    ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                        admin_action_takers: AuthorizedActionTakers::NoOne,
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_admin_action_takers_to_no_one_allowed: false,
                        self_changing_admin_action_takers_allowed: false,
                    },
                ));
                token_configuration.set_freeze_rules(ChangeControlRules::V0(
                    ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                        admin_action_takers: AuthorizedActionTakers::NoOne,
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_admin_action_takers_to_no_one_allowed: false,
                        self_changing_admin_action_takers_allowed: false,
                    },
                ));
                token_configuration.set_manual_minting_rules(ChangeControlRules::V0(
                    ChangeControlRulesV0 {
                        authorized_to_make_change: AuthorizedActionTakers::ContractOwner,
                        admin_action_takers: AuthorizedActionTakers::NoOne,
                        changing_authorized_action_takers_to_no_one_allowed: false,
                        changing_admin_action_takers_to_no_one_allowed: false,
                        self_changing_admin_action_takers_allowed: false,
                    },
                ));
                token_configuration
                    .distribution_rules_mut()
                    .set_minting_allow_choosing_destination(true);
            }),
            None,
            None,
            None,
            platform_version,
        );

        // Mint 5000 to identity_2.
        let mint_to_2 = BatchTransition::new_token_mint_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            5000,
            Some(identity_2.id()),
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
        .expect("expected to create mint transition");

        process_and_commit_success(&mut platform, &platform_state, mint_to_2, platform_version);

        // Mint 3000 to the owner.
        let mint_to_owner = BatchTransition::new_token_mint_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            3000,
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
        .expect("expected to create mint transition");

        process_and_commit_success(
            &mut platform,
            &platform_state,
            mint_to_owner,
            platform_version,
        );

        let total_supply_before = platform
            .drive
            .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");
        assert_eq!(total_supply_before, Some(8000));

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
            4,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("expected to create freeze transition");

        process_and_commit_success(
            &mut platform,
            &platform_state,
            freeze_transition,
            platform_version,
        );

        // Destroy identity_2's frozen funds (5000).
        let destroy_transition = BatchTransition::new_token_destroy_frozen_funds_transition(
            token_id,
            identity.id(),
            contract.id(),
            0,
            identity_2.id(),
            None,
            None,
            &key,
            5,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("expected to create destroy frozen funds transition");

        process_and_commit_success(
            &mut platform,
            &platform_state,
            destroy_transition,
            platform_version,
        );

        // identity_2 emptied; total supply decremented by exactly 5000 (8000 -> 3000);
        // owner balance untouched.
        let balance_2 = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity_2.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        assert_eq!(balance_2, Some(0));

        let balance_owner = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        assert_eq!(balance_owner, Some(3000));

        let total_supply_after = platform
            .drive
            .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");
        assert_eq!(total_supply_after, Some(3000));
    }

    // Helper: process a single state transition, assert it succeeded, and commit.
    // Delegates to the shared process_test_state_transition helper (commits internally).
    fn process_and_commit_success<S: dpp::serialization::PlatformSerializable>(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        platform_state: &PlatformState,
        transition: S,
        platform_version: &PlatformVersion,
    ) {
        let processing_result =
            process_test_state_transition(platform, transition, platform_state, platform_version);

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
    }
}
