//! Same-block randomness across many blocks (protocol version 15): documents of a type with a
//! `sys.random.uniform` property, and random token transfers and mints, are sent block after
//! block through the mimic, which hands each block a stand-in random beacon through
//! `ApplyBlockRandomness` and signs the commit over the state after the draws. Every
//! transition's proof is verified against that state, the drawn values lie in their range, and
//! the draws replaced the placeholder.

#[cfg(test)]
mod tests {
    use crate::execution::run_chain_for_strategy;
    use crate::strategy::NetworkStrategy;
    use dash_platform_macros::stack_size;
    use dpp::dash_to_duffs;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::accessors::v1::DataContractV1Getters;
    use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
    use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Setters;
    use dpp::data_contract::associated_token::token_keeps_history_rules::accessors::v0::TokenKeepsHistoryRulesV0Setters;
    use dpp::data_contract::document_type::random_document::{
        DocumentFieldFillSize, DocumentFieldFillType,
    };
    use dpp::data_contract::{DataContract, DataContractFactory};
    use dpp::document::{Document, DocumentV0Getters};
    use dpp::identifier::Identifier;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::platform_value::platform_value;
    use dpp::state_transition::StateTransition;
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dpp::tests::json_document::json_document_to_created_contract;
    use dpp::tokens::token_event::TokenEvent;
    use dpp::version::PlatformVersion;
    use drive::drive::document::query::QueryDocumentsOutcomeV0Methods;
    use drive::drive::tokens::settle_random_token_amounts::random_token_reservation_id;
    use drive::drive::Drive;
    use drive::query::DriveDocumentQuery;
    use drive_abci::config::{
        ChainLockConfig, ExecutionConfig, InstantLockConfig, PlatformConfig, PlatformTestConfig,
        ValidatorSetConfig,
    };
    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use simple_signer::signer::SimpleSigner;
    use strategy_tests::frequency::Frequency;
    use strategy_tests::operations::{
        DocumentAction, DocumentOp, Operation, OperationType, TokenOp,
    };
    use strategy_tests::transitions::create_state_transitions_for_identities;
    use strategy_tests::{IdentityInsertInfo, StartAddresses, StartIdentities, Strategy};

    #[stack_size(4 * 1024 * 1024)]
    #[test]
    async fn should_draw_every_random_property_from_its_block_beacon() {
        let platform_version = PlatformVersion::latest();

        // A `card` whose `rarity` is drawn from 1 to 100 and indexed. The strategy's random
        // documents leave it out, as a client does: the platform refuses a sent value.
        let created_contract = DataContractFactory::new(platform_version.protocol_version)
            .expect("expected a contract factory")
            .create_with_value_config(
                Identifier::default(),
                1,
                platform_value!({
                    "card": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string", "maxLength": 32, "position": 0 },
                            "rarity": {
                                "type": "integer",
                                "minimum": 1,
                                "maximum": 100,
                                "generatedFrom": { "function": "sys.random.uniform" },
                                "position": 1
                            }
                        },
                        "indices": [{ "name": "byRarity", "properties": [{ "rarity": "asc" }] }],
                        "required": ["name", "rarity"],
                        "additionalProperties": false
                    }
                }),
                None,
                None,
            )
            .expect("expected the card contract");

        let contract = created_contract.data_contract();

        let document_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentAction::DocumentActionInsertRandom(
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
            ),
            document_type: contract
                .document_type_for_name("card")
                .expect("expected the card document type")
                .to_owned_document_type(),
        };

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![(created_contract, None)],
                operations: vec![Operation {
                    op_type: OperationType::Document(document_op),
                    frequency: Frequency {
                        times_per_block_range: 1..3,
                        chance_per_block: None,
                    },
                }],
                start_identities: StartIdentities::default(),
                start_addresses: StartAddresses::default(),
                identity_inserts: IdentityInsertInfo {
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                    ..Default::default()
                },
                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: None,
            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            // Each transition's proof is verified against the state the commit signs, after
            // the block's draws
            verify_state_transition_results: true,
            ..Default::default()
        };
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig::default_100_67(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                ..Default::default()
            },
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .with_latest_protocol_version()
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(
            &mut platform,
            15,
            strategy,
            config,
            15,
            &mut None,
            &mut None,
        )
        .await;

        for results in outcome.state_transition_results_per_block.values() {
            for (state_transition, result) in results {
                assert_eq!(
                    result.code, 0,
                    "state transition got code {} : {:?}",
                    result.code, state_transition
                );
            }
        }

        // The runner derives the contract id at deployment, so read it back from the outcome's
        // strategy
        let contract_id = outcome
            .strategy
            .strategy
            .start_contracts
            .first()
            .expect("expected the start contract")
            .0
            .data_contract()
            .id();
        let fetched_contract = outcome
            .abci_app
            .platform
            .drive
            .fetch_contract(contract_id.to_buffer(), None, None, None, platform_version)
            .unwrap()
            .expect("expected to fetch the contract")
            .expect("expected the contract to exist");

        let query = DriveDocumentQuery::from_sql_expr(
            "select * from card",
            &fetched_contract.contract,
            None,
            platform_version,
        )
        .expect("expected a card query");
        let cards = outcome
            .abci_app
            .platform
            .drive
            .query_documents(query, None, false, None, None)
            .expect("expected to query the cards")
            .documents()
            .to_vec();
        assert!(
            cards.len() >= 5,
            "cards are created over many blocks, got {}",
            cards.len()
        );

        let rarities: Vec<i128> = cards
            .iter()
            .map(|card| {
                card.get("rarity")
                    .and_then(|rarity| rarity.to_integer::<i128>().ok())
                    .expect("expected every card to hold a rarity")
            })
            .collect();
        assert!(
            rarities.iter().all(|rarity| (1..=100).contains(rarity)),
            "every drawn rarity lies in its range: {rarities:?}"
        );
        assert!(
            rarities.iter().any(|rarity| *rarity != 1),
            "the draws replaced the placeholder: {rarities:?}"
        );
    }

    /// The amounts the token history documents of `document_type_name` (`transfer` or `mint`)
    /// hold.
    fn history_amounts(drive: &Drive, document_type_name: &str) -> Vec<u64> {
        let platform_version = PlatformVersion::latest();
        let token_history =
            load_system_data_contract(SystemDataContract::TokenHistory, platform_version)
                .expect("expected the token history contract");
        let query = DriveDocumentQuery::from_sql_expr(
            &format!("select * from {document_type_name}"),
            &token_history,
            None,
            platform_version,
        )
        .expect("expected a history query");
        drive
            .query_documents(query, None, false, None, None)
            .expect("expected to query the history")
            .documents()
            .iter()
            .map(|document: &Document| {
                document
                    .get("amount")
                    .and_then(|amount| amount.to_integer::<u64>().ok())
                    .expect("expected every history document to hold an amount")
            })
            .collect()
    }

    #[stack_size(4 * 1024 * 1024)]
    #[test]
    async fn should_settle_every_random_token_amount_from_its_block_beacon() {
        const BASE_SUPPLY: u64 = 100_000;
        const MINIMUM: u64 = 100;
        const MAXIMUM: u64 = 1_000;
        let platform_version = PlatformVersion::latest();

        // A token keeping its transfer and minting history, whose owner mints to a chosen
        // recipient
        let mut created_contract = json_document_to_created_contract(
            "tests/supporting_files/contract/basic-token/basic-token.json",
            1,
            true,
            platform_version,
        )
        .expect("expected the token contract");

        let mut rng = StdRng::seed_from_u64(567);
        let mut signer = SimpleSigner::default();
        let (mut owner, owner_keys) = Identity::random_identity_with_main_keys_with_private_key::<
            Vec<_>,
        >(3, &mut rng, platform_version)
        .expect("expected an identity");
        let (mut player, player_keys) =
            Identity::random_identity_with_main_keys_with_private_key::<Vec<_>>(
                3,
                &mut rng,
                platform_version,
            )
            .expect("expected an identity");
        signer.add_identity_public_keys(owner_keys);
        signer.add_identity_public_keys(player_keys);
        let start_identities: Vec<(Identity, Option<StateTransition>)> =
            create_state_transitions_for_identities(
                vec![&mut owner, &mut player],
                &(dash_to_duffs!(1)..=dash_to_duffs!(1)),
                &signer,
                &mut rng,
                platform_version,
            )
            .await
            .into_iter()
            .map(|(identity, transition)| (identity, Some(transition)))
            .collect();

        let contract = created_contract.data_contract_mut();
        let token_configuration = contract
            .token_configuration_mut(0)
            .expect("expected the token configuration");
        token_configuration
            .distribution_rules_mut()
            .set_minting_allow_choosing_destination(true);
        token_configuration
            .keeps_history_mut()
            .set_keeps_transfer_history(true);
        token_configuration
            .keeps_history_mut()
            .set_keeps_minting_history(true);
        contract.set_owner_id(owner.id());
        contract.set_id(DataContract::generate_data_contract_id_v0(owner.id(), 1));
        let token_id = contract.token_id(0).expect("expected the token id");

        let token_operation = |action: TokenEvent| Operation {
            op_type: OperationType::Token(TokenOp {
                contract: contract.clone(),
                token_id,
                token_pos: 0,
                use_identity_with_id: Some(owner.id()),
                action,
            }),
            frequency: Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            },
        };
        let operations = vec![
            token_operation(TokenEvent::Transfer(player.id(), None, None, None, MAXIMUM)),
            token_operation(TokenEvent::Mint(MAXIMUM, player.id(), None)),
        ];

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![(created_contract, None)],
                operations,
                start_identities: StartIdentities {
                    hard_coded: start_identities,
                    ..Default::default()
                },
                start_addresses: StartAddresses::default(),
                identity_inserts: IdentityInsertInfo::default(),
                identity_contract_nonce_gaps: None,
                signer: Some(signer),
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: None,
            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            // Each transition's proof is verified against the state the commit signs, after
            // the block's settlements
            verify_state_transition_results: true,
            // Every transfer and mint moves an amount drawn from the minimum to its amount
            random_token_minimum_amount: Some(MINIMUM),
            ..Default::default()
        };
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig::default_100_67(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                ..Default::default()
            },
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .with_latest_protocol_version()
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(
            &mut platform,
            12,
            strategy,
            config,
            15,
            &mut None,
            &mut None,
        )
        .await;

        let mut sent = 0u64;
        for results in outcome.state_transition_results_per_block.values() {
            for (state_transition, result) in results {
                assert_eq!(
                    result.code, 0,
                    "state transition got code {} : {:?}",
                    result.code, state_transition
                );
                if matches!(state_transition, StateTransition::Batch(_)) {
                    sent += 1;
                }
            }
        }

        let drive = &outcome.abci_app.platform.drive;
        let transferred = history_amounts(drive, "transfer");
        let minted = history_amounts(drive, "mint");
        assert!(
            transferred.len() >= 5 && minted.len() >= 5,
            "random transfers and mints are settled over many blocks: {transferred:?}, {minted:?}"
        );
        assert_eq!(
            (transferred.len() + minted.len()) as u64,
            sent,
            "every random transfer and mint wrote its history"
        );
        let drawn: Vec<u64> = transferred.iter().chain(&minted).copied().collect();
        assert!(
            drawn
                .iter()
                .all(|amount| (MINIMUM..=MAXIMUM).contains(amount)),
            "every drawn amount lies in its range: {drawn:?}"
        );
        assert!(
            drawn.iter().any(|amount| *amount != MINIMUM),
            "the settlements replaced the placeholder: {drawn:?}"
        );

        // Each settlement paid out once, the drawn amount: the balances and the supply are
        // those of plain transfers and mints of the amounts the history holds
        let transferred_total: u64 = transferred.iter().sum();
        let minted_total: u64 = minted.iter().sum();
        let balance = |identity: &Identity| {
            drive
                .fetch_identity_token_balance(
                    token_id.to_buffer(),
                    identity.id().to_buffer(),
                    None,
                    platform_version,
                )
                .expect("expected to fetch a token balance")
        };
        assert_eq!(balance(&owner), Some(BASE_SUPPLY - transferred_total));
        assert_eq!(balance(&player), Some(transferred_total + minted_total));
        assert_eq!(
            drive
                .fetch_token_total_supply(token_id.to_buffer(), None, platform_version)
                .expect("expected to fetch the total supply"),
            Some(BASE_SUPPLY + minted_total)
        );

        // No reservation is left in the escrow, and the token totals balance
        for nonce in 1..=sent {
            assert_eq!(
                drive
                    .fetch_token_order_escrow(
                        token_id.to_buffer(),
                        random_token_reservation_id(&token_id, &owner.id(), nonce),
                        None,
                        &mut vec![],
                        platform_version,
                    )
                    .expect("expected to fetch a reservation"),
                None,
                "the reservation of nonce {nonce} was settled"
            );
        }
        let total = drive
            .calculate_total_tokens_balance(None, platform_version)
            .expect("expected to sum the token balances");
        assert!(
            total.ok().expect("expected the sums to compare"),
            "tokens are not balanced: {total}"
        );
    }
}
