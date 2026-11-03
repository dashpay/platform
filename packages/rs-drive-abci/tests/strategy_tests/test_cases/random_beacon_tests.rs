//! Same-block randomness across many blocks (protocol version 15): documents of a type with a
//! `sys.random.uniform` property are created block after block through the mimic, which hands
//! each block a stand-in random beacon through `ApplyBlockRandomness` and signs the commit
//! over the state after the draws. Every transition's proof is verified against that state, the
//! drawn values lie in their range, and the draws replaced the placeholder.

#[cfg(test)]
mod tests {
    use crate::execution::run_chain_for_strategy;
    use crate::strategy::NetworkStrategy;
    use dash_platform_macros::stack_size;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::random_document::{
        DocumentFieldFillSize, DocumentFieldFillType,
    };
    use dpp::data_contract::DataContractFactory;
    use dpp::document::DocumentV0Getters;
    use dpp::identifier::Identifier;
    use dpp::platform_value::platform_value;
    use dpp::version::PlatformVersion;
    use drive::drive::document::query::QueryDocumentsOutcomeV0Methods;
    use drive::query::DriveDocumentQuery;
    use drive_abci::config::{
        ChainLockConfig, ExecutionConfig, InstantLockConfig, PlatformConfig, PlatformTestConfig,
        ValidatorSetConfig,
    };
    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use strategy_tests::frequency::Frequency;
    use strategy_tests::operations::{DocumentAction, DocumentOp, Operation, OperationType};
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
}
