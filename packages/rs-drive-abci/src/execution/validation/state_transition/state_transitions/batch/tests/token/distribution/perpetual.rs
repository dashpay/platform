use super::*;
mod perpetual_distribution {
    use super::*;
    #[test]
    fn test_token_perpetual_distribution_block_claim() {
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
            None,
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
                &vec![config_update_transition_serialized_transition.clone()],
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
}
