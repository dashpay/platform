use super::*;

mod token_selling_tests {
    use std::collections::BTreeMap;

    use crate::platform_types::state_transitions_processing_result::StateTransitionsProcessingResult;
    use crate::{rpc::core::MockCoreRPCLike, test::helpers::setup::TempPlatform};

    use super::*;

    use dpp::{
        dashcore::secp256k1::hashes::hex::{Case, DisplayHex},
        prelude::{DataContract, Identity, IdentityPublicKey},
        tokens::token_pricing_schedule::TokenPricingSchedule,
    };
    use drive::verify::RootHash;
    use simple_signer::signer::SimpleSigner;

    #[tokio::test]
    async fn test_successful_direct_purchase_single_price() {
        run_successful_direct_purchase_single_price_at_protocol_version(
            PlatformVersion::latest().protocol_version,
            699_868_122_220,
        )
        .await;
    }

    /// PROTOCOL_VERSION_11: pre-B4/B7 buyer balance — query_documents +
    /// transformer-phase reads were dropped, so the buyer paid 7,900
    /// credits less in fees. Pinned so v11 chain history stays
    /// bit-for-bit reproducible.
    #[tokio::test]
    async fn test_successful_direct_purchase_single_price_protocol_version_11() {
        run_successful_direct_purchase_single_price_at_protocol_version(11, 699_868_130_120).await;
    }

    async fn run_successful_direct_purchase_single_price_at_protocol_version(
        protocol_version: dpp::version::ProtocolVersion,
        expected_buyer_credit_balance: dpp::fee::Credits,
    ) {
        let platform_version = PlatformVersion::get(protocol_version)
            .expect("expected platform version for the requested protocol_version");
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(protocol_version)
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
        )
        .await;

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
            .await
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
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
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
        .await
        .unwrap();

        let processing_result = process_test_state_transition(
            &mut platform,
            purchase_transition,
            &platform_state,
            platform_version,
        );

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
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
        assert_eq!(
            buyer_credit_balance,
            Some(expected_buyer_credit_balance),
            "PROTOCOL_VERSION_{}: buyer credit balance after direct purchase must match the version-specific baseline (10.0 - 3.0 spent - fees =~ 7 dash left)",
            protocol_version,
        );
    }

    #[tokio::test]
    async fn test_direct_purchase_change_using_group_without_needing_group() {
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
            None,
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
            .await
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
            [PaidConsensusError {
                error: ConsensusError::StateError(StateError::UnauthorizedTokenActionError(_)),
                ..
            }]
        );
    }

    #[tokio::test]
    async fn test_direct_purchase_single_price_not_paying_full_price() {
        run_direct_purchase_single_price_not_paying_full_price_at_protocol_version(
            PlatformVersion::latest().protocol_version,
            999_987_864_860,
        )
        .await;
    }

    /// PROTOCOL_VERSION_11: pre-B4/B7 bump-only buyer balance — under v0
    /// the failed purchase still bumps the nonce but doesn't bill the
    /// extra read costs (7,900 credits). Pinned so v11 chain history
    /// stays bit-for-bit reproducible.
    #[tokio::test]
    async fn test_direct_purchase_single_price_not_paying_full_price_protocol_version_11() {
        run_direct_purchase_single_price_not_paying_full_price_at_protocol_version(
            11,
            999_987_872_760,
        )
        .await;
    }

    async fn run_direct_purchase_single_price_not_paying_full_price_at_protocol_version(
        protocol_version: dpp::version::ProtocolVersion,
        expected_buyer_credit_balance: dpp::fee::Credits,
    ) {
        let platform_version = PlatformVersion::get(protocol_version)
            .expect("expected platform version for the requested protocol_version");
        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(protocol_version)
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
            .await
            .unwrap();

        let processing_result = process_test_state_transition(
            &mut platform,
            set_price_transition,
            &platform_state,
            platform_version,
        );

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
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
        .await
        .unwrap();

        let processing_result = process_test_state_transition(
            &mut platform,
            purchase_transition,
            &platform_state,
            platform_version,
        );

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [PaidConsensusError {
                error: ConsensusError::StateError(StateError::TokenDirectPurchaseUserPriceTooLow(
                    _
                )),
                ..
            }]
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
        assert_eq!(
            buyer_credit_balance,
            Some(expected_buyer_credit_balance),
            "PROTOCOL_VERSION_{}: buyer credit balance after failed direct purchase must match the version-specific baseline (10.0 - bump action fees)",
            protocol_version,
        );
    }

    #[tokio::test]
    async fn test_direct_purchase_insufficient_credits() {
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
            .await
            .unwrap();

        let processing_result = process_test_state_transition(
            &mut platform,
            set_price_transition,
            &platform_state,
            platform_version,
        );

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
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
        .await
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
    #[tokio::test]
    async fn test_successful_direct_purchase_multiple_tokens() {
        //  Given 3 tokens
        let pricing_schedules = vec![
            TokenPricingSchedule::SinglePrice(dash_to_credits!(1)),
            TokenPricingSchedule::SetPrices(BTreeMap::from([
                (100, dash_to_credits!(10)),
                (500, dash_to_credits!(5)),
            ])),
            TokenPricingSchedule::SetPrices(BTreeMap::from([(1, dash_to_credits!(2))])),
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
        let mut tokens: BTreeMap<[u8; 32], _> = BTreeMap::new();
        for pricing in pricing_schedules.into_iter() {
            let (contract, token_id) = create_token_with_pricing(
                platform_version,
                &mut platform,
                &seller,
                &seller_signer,
                &seller_key,
                Some(pricing.clone()),
                &mut identity_contract_nonce,
            )
            .await;

            tokens.insert(token_id.to_buffer(), (pricing, contract));
        }

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

    #[tokio::test]
    async fn test_direct_purchase_from_yourself() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(54321);
        // Create an identity that will be both seller and buyer
        let (self_trader, self_trader_signer, self_trader_key) =
            setup_identity(&mut platform, rng.gen(), dash_to_credits!(10.0));

        let single_price = TokenPricingSchedule::SinglePrice(dash_to_credits!(1));

        let mut identity_contract_nonce: u64 = 2;
        let (contract, token_id) = create_token_with_pricing(
            platform_version,
            &mut platform,
            &self_trader,
            &self_trader_signer,
            &self_trader_key,
            Some(single_price.clone()),
            &mut identity_contract_nonce,
        )
        .await;

        // Set the price
        let set_price_transition =
            BatchTransition::new_token_change_direct_purchase_price_transition(
                token_id,
                self_trader.id(),
                contract.id(),
                0,
                Some(single_price.clone()),
                None,
                None,
                &self_trader_key,
                identity_contract_nonce,
                0,
                &self_trader_signer,
                platform_version,
                None,
            )
            .await
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
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );

        // Check initial token balance (should have some tokens as the owner)
        let initial_token_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                self_trader.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");

        // Now purchase tokens from yourself
        let purchase_transition = BatchTransition::new_token_direct_purchase_transition(
            token_id,
            self_trader.id(), // Buyer is the same as seller
            contract.id(),
            0,
            5,                   // Buying 5 tokens
            dash_to_credits!(5), // Paying for 5 tokens
            &self_trader_key,
            identity_contract_nonce + 1,
            0,
            &self_trader_signer,
            platform_version,
            None,
        )
        .await
        .unwrap();

        let initial_credit_balance = platform
            .drive
            .fetch_identity_balance(self_trader.id().to_buffer(), None, platform_version)
            .expect("expected to fetch credit balance");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[purchase_transition
                    .serialize_to_bytes()
                    .expect("expected documents batch serialized state transition")],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        platform
            .validate_token_aggregated_balance(&transaction, platform_version)
            .expect("expected to validate token aggregated balances");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );

        // Check token balance after purchase
        let final_token_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                self_trader.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");

        // Token balance should increase by 5
        assert_eq!(
            final_token_balance,
            initial_token_balance.map(|b| b + 5).or(Some(5))
        );

        // Check credit balance - should only be reduced by fees, not by the purchase price
        // (since the money goes back to yourself)
        let final_credit_balance = platform
            .drive
            .fetch_identity_balance(self_trader.id().to_buffer(), None, platform_version)
            .expect("expected to fetch credit balance");

        // The difference should be just the transaction fees
        let credit_diff = initial_credit_balance.unwrap() - final_credit_balance.unwrap();

        // Assert that the difference is much less than the purchase price (just fees)
        assert!(
            credit_diff < dash_to_credits!(0.01),
            "Credit difference should only be transaction fees, but was: {}",
            credit_diff
        );
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
    async fn create_token_with_pricing(
        platform_version: &PlatformVersion,
        platform: &mut TempPlatform<MockCoreRPCLike>,
        seller: &Identity,
        seller_signer: &SimpleSigner,
        seller_key: &IdentityPublicKey,
        pricing: Option<TokenPricingSchedule>,
        identity_contract_nonce: &mut u64,
    ) -> (DataContract, Identifier) {
        let (contract, token_id, processing_result) = create_token_with_pricing_result(
            platform_version,
            platform,
            seller,
            seller_signer,
            seller_key,
            pricing,
            identity_contract_nonce,
        )
        .await;

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );
        (contract, token_id)
    }

    /// Same as [`create_token_with_pricing`], but returns the set-price processing
    /// result instead of asserting it succeeded, for tests that expect rejection.
    async fn create_token_with_pricing_result(
        platform_version: &PlatformVersion,
        platform: &mut TempPlatform<MockCoreRPCLike>,
        seller: &Identity,
        seller_signer: &SimpleSigner,
        seller_key: &IdentityPublicKey,
        pricing: Option<TokenPricingSchedule>,
        identity_contract_nonce: &mut u64,
    ) -> (DataContract, Identifier, StateTransitionsProcessingResult) {
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
            .await
            .unwrap();
        *identity_contract_nonce += 1;

        let processing_result = process_test_state_transition(
            platform,
            set_price_transition,
            &platform_state,
            platform_version,
        );

        (contract, token_id, processing_result)
    }

    /// Regression test for the chain-halt where an empty `SetPrices` schedule
    /// caused a `.expect("Map is not empty")` panic in the direct-purchase
    /// transformer.
    ///
    /// An empty `SetPrices` schedule is now rejected at structure validation
    /// with `TokenPricingScheduleEmptyError`, so it can no longer reach state
    /// through a state transition — asserted first. The transformer must still
    /// handle an empty schedule defensively (defense in depth, e.g. state
    /// written before the structure check existed): the transformer runs on
    /// every validator during block execution, so a panic there would crash
    /// all validators and halt the chain. The schedule is therefore planted
    /// directly through the drive API, and a purchase against it must be
    /// rejected gracefully as `TokenNotForDirectSale` — no panic.
    #[tokio::test]
    async fn test_direct_purchase_empty_set_prices_does_not_panic() {
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

        let empty_set_prices = TokenPricingSchedule::SetPrices(BTreeMap::new());

        // Setting an EMPTY tiered pricing schedule via a state transition must
        // be rejected at structure validation.
        let mut identity_contract_nonce: u64 = 2;
        let (contract, token_id, processing_result) = create_token_with_pricing_result(
            platform_version,
            &mut platform,
            &seller,
            &seller_signer,
            &seller_key,
            Some(empty_set_prices.clone()),
            &mut identity_contract_nonce,
        )
        .await;

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::UnpaidConsensusError(
                ConsensusError::BasicError(BasicError::TokenPricingScheduleEmptyError(_))
            )]
        );

        // Plant: write the empty schedule straight into state through the
        // drive API, bypassing state transition validation — simulating state
        // written before the structure check existed.
        platform
            .drive
            .token_set_direct_purchase_price(
                token_id.to_buffer(),
                Some(empty_set_prices),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to set empty pricing schedule directly");

        // Detonate: any direct purchase used to panic at
        // `set_prices.keys().next().expect("Map is not empty")`.
        let platform_state = platform.state.load();
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
        .await
        .unwrap();

        let processing_result = process_test_state_transition(
            &mut platform,
            purchase_transition,
            &platform_state,
            platform_version,
        );

        // Must be rejected gracefully (the node did not panic to get here).
        assert_matches!(
            processing_result.execution_results().as_slice(),
            [PaidConsensusError {
                error: ConsensusError::StateError(StateError::TokenNotForDirectSale(_)),
                ..
            }]
        );

        // The buyer must not have received any tokens.
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

    /// Companion to the empty-`SetPrices` regression test: a NON-empty tiered
    /// schedule whose smallest tier is above the requested amount must be
    /// rejected as below the minimum sale amount (the `Some` arm of the same
    /// no-matching-tier branch), not panic.
    #[tokio::test]
    async fn test_direct_purchase_below_minimum_sale_amount() {
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

        // Tiered schedule whose smallest tier requires at least 100 tokens.
        let tiered = TokenPricingSchedule::SetPrices(BTreeMap::from([
            (100, dash_to_credits!(10)),
            (500, dash_to_credits!(5)),
        ]));

        let mut identity_contract_nonce: u64 = 2;
        let (contract, token_id) = create_token_with_pricing(
            platform_version,
            &mut platform,
            &seller,
            &seller_signer,
            &seller_key,
            Some(tiered),
            &mut identity_contract_nonce,
        )
        .await;

        // Buyer asks for 3 tokens — below the smallest (100-token) tier, so
        // `range(..=3).next_back()` is `None` and the smallest defined tier (100)
        // is reported as the minimum sale amount.
        let platform_state = platform.state.load();
        let purchase_transition = BatchTransition::new_token_direct_purchase_transition(
            token_id,
            buyer.id(),
            contract.id(),
            0,
            3,
            dash_to_credits!(3),
            &buyer_key,
            2,
            0,
            &buyer_signer,
            platform_version,
            None,
        )
        .await
        .unwrap();

        let processing_result = process_test_state_transition(
            &mut platform,
            purchase_transition,
            &platform_state,
            platform_version,
        );

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [PaidConsensusError {
                error: ConsensusError::StateError(StateError::TokenAmountUnderMinimumSaleAmount(_)),
                ..
            }]
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
