use super::*;

mod additional_validation_tests {
    use super::*;
    use dpp::tokens::info::v0::IdentityTokenInfoV0Accessors;

    #[test]
    fn test_token_freeze_already_frozen_should_fail() {
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

        // Freeze identity_2
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

        // Verify frozen
        let info = platform
            .drive
            .fetch_identity_token_info(
                token_id.to_buffer(),
                identity_2.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token info");
        assert_eq!(info.map(|i| i.frozen()), Some(true));

        // Try to freeze again - should fail
        let freeze_again_transition = BatchTransition::new_token_freeze_transition(
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
        .expect("expected to create freeze transition");

        let serialized = freeze_again_transition
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
                error: ConsensusError::StateError(
                    StateError::IdentityTokenAccountAlreadyFrozenError(_)
                ),
                ..
            }]
        );
    }

    #[test]
    fn test_token_unfreeze_not_frozen_should_fail() {
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

        // Try to unfreeze without being frozen
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
        .expect("expected to create unfreeze transition");

        let serialized = unfreeze_transition
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
}
