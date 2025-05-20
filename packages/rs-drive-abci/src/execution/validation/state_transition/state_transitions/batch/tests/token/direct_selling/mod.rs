use super::*;

mod token_selling_tests {
    use std::collections::BTreeMap;

    use crate::{rpc::core::MockCoreRPCLike, test::helpers::setup::TempPlatform};

    use super::*;

    use dpp::{
        dashcore::secp256k1::hashes::hex::{Case, DisplayHex},
        prelude::{DataContract, Identity, IdentityPublicKey},
        tokens::token_pricing_schedule::TokenPricingSchedule,
    };
    use drive::verify::RootHash;
    use simple_signer::signer::SimpleSigner;
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

        let single_price = TokenPricingSchedule::SinglePrice(dash_to_credits!(1));

        let mut identity_contract_nonce: u64 = 2;
        let (contract, token_id) = create_token_with_pricing(
            platform_version,
            &mut platform,
            &seller,
            &seller_signer,
            &seller_key,
            Some(single_price.clone()),
            &mut identity_contract_nonce,
        );

        // Seller sets single price
        let set_price_transition =
            BatchTransition::new_token_change_direct_purchase_price_transition(
                token_id,
                seller.id(),
                contract.id(),
                0,
                Some(single_price.clone()), // Price per token
                None,
                None,
                &seller_key,
                identity_contract_nonce,
                0,
                &seller_signer,
                platform_version,
                None,
            )
            .unwrap();

        let platform_state = platform.state.load();
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

        let token_id_buffer = token_id.to_buffer();
        // Buyer checks the price; first, without proofs
        let prices = platform
            .drive
            .fetch_tokens_direct_purchase_price(&[token_id_buffer], None, platform_version)
            .expect("expected to fetch token price");
        assert_price(
            prices,
            &token_id_buffer,
            &single_price,
            "fetched price mismatch",
        );

        // Buyer checks the price with proofs
        let price_proof = platform
            .drive
            .prove_tokens_direct_purchase_price(&[token_id_buffer], None, platform_version)
            .expect("expected to prove token price");

        let (_, price_response): (RootHash, BTreeMap<[u8; 32], Option<TokenPricingSchedule>>) =
            drive::drive::Drive::verify_token_direct_selling_prices(
                &price_proof,
                &[token_id_buffer],
                true,
                platform_version,
            )
            .expect("expected to verify token price proof");
        assert_price(
            price_response,
            &token_id_buffer,
            &single_price,
            "price in proof mismatch",
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
        assert_eq!(buyer_credit_balance, Some(699_868_051_500)); // 10.0 - 3.0 spent - fees =~ 7 dash left
    }

    #[test]
    fn test_direct_purchase_change_using_group_without_needing_group() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(12345);
        let (seller, seller_signer, seller_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(1.0));

        let (identity_2, _, _) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(1.0));

        let single_price = TokenPricingSchedule::SinglePrice(dash_to_credits!(1));

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
            Some(
                [(
                    0,
                    Group::V0(GroupV0 {
                        members: [(seller.id(), 1), (identity_2.id(), 1)].into(),
                        required_power: 2,
                    }),
                )]
                .into(),
            ),
            platform_version,
        );

        // Seller sets single price
        let set_price_transition =
            BatchTransition::new_token_change_direct_purchase_price_transition(
                token_id,
                seller.id(),
                contract.id(),
                0,
                Some(single_price.clone()), // Price per token
                None,
                Some(GroupStateTransitionInfoStatus::GroupStateTransitionInfoProposer(0)),
                &seller_key,
                2,
                0,
                &seller_signer,
                platform_version,
                None,
            )
            .unwrap();

        let platform_state = platform.state.load();
        let processing_result = process_test_state_transition(
            &mut platform,
            set_price_transition,
            &platform_state,
            platform_version,
        );

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [PaidConsensusError(
                ConsensusError::StateError(StateError::UnauthorizedTokenActionError(_)),
                _
            )]
        );
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

    /// Given 3 tokens, each of them with different pricing structure,
    /// When I create them and set their prices,
    /// Then I should get the correct price for each of them
    /// And the price should be the same as the one set by the seller.
    #[test]
    fn test_successful_direct_purchase_multiple_tokens() {
        //  Given 3 tokens
        let pricing_schedules = vec![
            TokenPricingSchedule::SinglePrice(dash_to_credits!(1)),
            TokenPricingSchedule::SetPrices(BTreeMap::from([
                (100, dash_to_credits!(10)),
                (500, dash_to_credits!(5)),
            ])),
            TokenPricingSchedule::SetPrices(BTreeMap::new()),
        ];

        // Setup the test
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(12345);

        let (seller, seller_signer, seller_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(1.0));

        let mut identity_contract_nonce = 2;
        let tokens = pricing_schedules
            .into_iter()
            .map(|pricing| {
                let (contract, token_id) = create_token_with_pricing(
                    platform_version,
                    &mut platform,
                    &seller,
                    &seller_signer,
                    &seller_key,
                    Some(pricing.clone()),
                    &mut identity_contract_nonce,
                );

                (token_id.to_buffer(), (pricing, contract))
            })
            .collect::<BTreeMap<_, _>>();

        //
        // When I fetch tokens, with or without proofs
        //
        let token_ids: Vec<[u8; 32]> = tokens.keys().cloned().collect();

        // Fetch with proofs
        let proof = platform
            .drive
            .prove_tokens_direct_purchase_price(&token_ids, None, platform_version)
            .expect("expected to prove token price");

        let (_, prices_from_proof): (RootHash, BTreeMap<[u8; 32], Option<TokenPricingSchedule>>) =
            drive::drive::Drive::verify_token_direct_selling_prices(
                &proof,
                &token_ids,
                true,
                platform_version,
            )
            .expect("expected to verify token price proof");

        //Fetch without proofs
        let fetched_prices = platform
            .drive
            .fetch_tokens_direct_purchase_price(&token_ids, None, platform_version)
            .expect("expected to fetch token price");

        //
        // Then I get correct prices
        //
        assert_eq!(fetched_prices.len(), token_ids.len());

        for (token_id, (expected_price, _)) in &tokens {
            // from proof
            assert_price(
                prices_from_proof.clone(),
                token_id,
                expected_price,
                format!(
                    "price in proof mismatch for token {}",
                    token_id.to_hex_string(Case::Lower)
                )
                .as_str(),
            );

            // non-proof
            assert_price(
                fetched_prices.clone(),
                token_id,
                expected_price,
                format!(
                    "fetched price mismatch for token {}",
                    token_id.to_hex_string(Case::Lower)
                )
                .as_str(),
            );
        }
    }

    // Helper functions
    //
    // /\_/\
    //( o.o )
    // > ^ <
    //

    /// Asserts that the price for a given token ID matches the expected price.
    /// If the price does not match, it will panic.
    fn assert_price(
        prices: BTreeMap<[u8; 32], Option<TokenPricingSchedule>>,
        token_id: &[u8; 32],
        expected_price: &TokenPricingSchedule,
        msg: &str,
    ) {
        let price = prices
            .get(token_id)
            .unwrap_or_else(|| panic!("{}: token not found", msg))
            .as_ref()
            .unwrap_or_else(|| panic!("{}: empty token price", msg));
        assert_eq!(price, expected_price, "{}", msg);
    }

    /// Creates a token contract with the given owner identity and configuration, and sets the price.
    fn create_token_with_pricing(
        platform_version: &dpp::version::PlatformVersion,
        platform: &mut TempPlatform<MockCoreRPCLike>,
        seller: &Identity,
        seller_signer: &SimpleSigner,
        seller_key: &IdentityPublicKey,
        pricing: Option<TokenPricingSchedule>,
        identity_contract_nonce: &mut u64,
    ) -> (DataContract, Identifier) {
        let (contract, token_id) = create_token_contract_with_owner_identity(
            platform,
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
                pricing, // Price per token
                None,
                None,
                seller_key,
                *identity_contract_nonce,
                0,
                seller_signer,
                platform_version,
                None,
            )
            .unwrap();
        *identity_contract_nonce += 1;

        let processing_result = process_test_state_transition(
            platform,
            set_price_transition,
            &platform_state,
            platform_version,
        );

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution(..)]
        );
        (contract, token_id)
    }
}
