use super::*;

mod token_selling_tests {
    use super::*;
    use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
    #[test]
    fn test_successful_direct_purchase_single_price() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(12345);
        let (seller, seller_signer, seller_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(1.0));
        let (buyer, buyer_signer, buyer_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(10.0));

        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            seller.id(),
            Some(|token_configuration: &mut TokenConfiguration| {
                token_configuration
                    .distribution_rules_mut()
                    .set_change_direct_purchase_pricing_rules(ChangeControlRules::V0(
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

        let platform_state = platform.state.load();

        // Seller sets single price
        let set_price_transition =
            BatchTransition::new_token_change_direct_purchase_price_transition(
                token_id,
                seller.id(),
                contract.id(),
                0,
                Some(TokenPricingSchedule::SinglePrice(dash_to_credits!(1))), // Price per token
                None,
                None,
                &seller_key,
                2,
                0,
                &seller_signer,
                platform_version,
                None,
                None,
                None,
            )
            .unwrap();

        let processing_result = process_test_state_transition(
            &mut platform,
            set_price_transition,
            &platform_state,
            platform_version,
        );

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution(..)]
        );

        // Buyer purchases tokens
        let purchase_transition = BatchTransition::new_token_direct_purchase_transition(
            token_id,
            buyer.id(),
            contract.id(),
            0,
            3, // Buying 3 tokens
            dash_to_credits!(3),
            &buyer_key,
            2,
            0,
            &buyer_signer,
            platform_version,
            None,
            None,
            None,
        )
        .unwrap();

        let processing_result = process_test_state_transition(
            &mut platform,
            purchase_transition,
            &platform_state,
            platform_version,
        );

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution(..)]
        );

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                buyer.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        assert_eq!(token_balance, Some(3));

        let buyer_credit_balance = platform
            .drive
            .fetch_identity_balance(buyer.id().to_buffer(), None, platform_version)
            .expect("expected to fetch credit balance");
        assert_eq!(buyer_credit_balance, Some(699_868_054_220)); // 10.0 - 3.0 spent - fees =~ 7 dash left
    }

    #[test]
    fn test_direct_purchase_single_price_not_paying_full_price() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(12345);
        let (seller, seller_signer, seller_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(1.0));
        let (buyer, buyer_signer, buyer_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(10.0));

        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            seller.id(),
            Some(|token_configuration: &mut TokenConfiguration| {
                token_configuration
                    .distribution_rules_mut()
                    .set_change_direct_purchase_pricing_rules(ChangeControlRules::V0(
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

        let platform_state = platform.state.load();

        // Seller sets single price
        let set_price_transition =
            BatchTransition::new_token_change_direct_purchase_price_transition(
                token_id,
                seller.id(),
                contract.id(),
                0,
                Some(TokenPricingSchedule::SinglePrice(dash_to_credits!(1))), // Price per token
                None,
                None,
                &seller_key,
                2,
                0,
                &seller_signer,
                platform_version,
                None,
                None,
                None,
            )
            .unwrap();

        let processing_result = process_test_state_transition(
            &mut platform,
            set_price_transition,
            &platform_state,
            platform_version,
        );

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution(..)]
        );

        // Buyer purchases tokens
        let purchase_transition = BatchTransition::new_token_direct_purchase_transition(
            token_id,
            buyer.id(),
            contract.id(),
            0,
            3,                   // Buying 3 tokens
            dash_to_credits!(2), // Not enough
            &buyer_key,
            2,
            0,
            &buyer_signer,
            platform_version,
            None,
            None,
            None,
        )
        .unwrap();

        let processing_result = process_test_state_transition(
            &mut platform,
            purchase_transition,
            &platform_state,
            platform_version,
        );

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::PaidConsensusError(
                ConsensusError::StateError(StateError::TokenDirectPurchaseUserPriceTooLow(_)),
                _
            )]
        );

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                buyer.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        assert_eq!(token_balance, None);

        let buyer_credit_balance = platform
            .drive
            .fetch_identity_balance(buyer.id().to_buffer(), None, platform_version)
            .expect("expected to fetch credit balance");
        assert_eq!(buyer_credit_balance, Some(999_987_872_760)); // 10.0 - bump action fees
    }

    #[test]
    fn test_direct_purchase_insufficient_credits() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(67890);
        let (seller, seller_signer, seller_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(1.0));
        let (buyer, buyer_signer, buyer_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.01)); // insufficient credits

        let (contract, token_id) = create_token_contract_with_owner_identity(
            &mut platform,
            seller.id(),
            Some(|token_configuration: &mut TokenConfiguration| {
                token_configuration
                    .distribution_rules_mut()
                    .set_change_direct_purchase_pricing_rules(ChangeControlRules::V0(
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

        let platform_state = platform.state.load();

        // Seller sets single price
        let set_price_transition =
            BatchTransition::new_token_change_direct_purchase_price_transition(
                token_id,
                seller.id(),
                contract.id(),
                0,
                Some(TokenPricingSchedule::SinglePrice(dash_to_credits!(1.0))), // Price per token
                None,
                None,
                &seller_key,
                2,
                0,
                &seller_signer,
                platform_version,
                None,
                None,
                None,
            )
            .unwrap();

        let processing_result = process_test_state_transition(
            &mut platform,
            set_price_transition,
            &platform_state,
            platform_version,
        );

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution(..)]
        );

        let purchase_transition = BatchTransition::new_token_direct_purchase_transition(
            token_id,
            buyer.id(),
            contract.id(),
            0,
            1,                     // Buying 1 token
            dash_to_credits!(1.0), // Agreed price per token
            &buyer_key,
            2,
            0,
            &buyer_signer,
            platform_version,
            None,
            None,
            None,
        )
        .unwrap();

        let processing_result = process_test_state_transition(
            &mut platform,
            purchase_transition,
            &platform_state,
            platform_version,
        );

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::StateError(StateError::IdentityInsufficientBalanceError(_))
            )]
        );

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                buyer.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");
        assert_eq!(token_balance, None);
    }
}
