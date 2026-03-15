use super::*;

mod token_destroy_frozen_funds_tests {
    use super::*;

    #[test]
    fn test_token_destroy_frozen_funds_on_unfrozen_account_should_fail() {
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
}
