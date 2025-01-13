#[cfg(test)]
mod tests {
    use crate::execution::run_chain_for_strategy;
    use crate::strategy::NetworkStrategy;
    use dpp::dash_to_duffs;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::data_contract::accessors::v1::DataContractV1Getters;
    use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Setters;
    use dpp::data_contract::DataContract;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::state_transition::StateTransition;
    use dpp::tests::json_document::json_document_to_created_contract;
    use dpp::tokens::token_event::TokenEvent;
    use drive::query::PathQuery;
    use drive_abci::config::{
        ChainLockConfig, ExecutionConfig, InstantLockConfig, PlatformConfig, PlatformTestConfig,
        ValidatorSetConfig,
    };
    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use platform_version::version::PlatformVersion;
    use rand::prelude::StdRng;
    use rand::SeedableRng;
    use simple_signer::signer::SimpleSigner;
    use strategy_tests::frequency::Frequency;
    use strategy_tests::operations::{Operation, OperationType, TokenOp};
    use strategy_tests::transitions::create_state_transitions_for_identities;
    use strategy_tests::{IdentityInsertInfo, StartIdentities, Strategy};

    #[test]
    fn run_chain_insert_one_new_identity_per_block_and_a_token_mint() {
        let platform_version = PlatformVersion::latest();
        let mut created_contract = json_document_to_created_contract(
            "tests/supporting_files/contract/basic-token/basic-token.json",
            1,
            true,
            platform_version,
        )
        .expect("expected to get contract from a json document");

        let mut rng = StdRng::seed_from_u64(567);

        let mut simple_signer = SimpleSigner::default();

        let (mut identity1, keys1) = Identity::random_identity_with_main_keys_with_private_key::<
            Vec<_>,
        >(2, &mut rng, platform_version)
        .unwrap();

        let (mut identity2, keys2) = Identity::random_identity_with_main_keys_with_private_key::<
            Vec<_>,
        >(2, &mut rng, platform_version)
        .unwrap();

        simple_signer.add_keys(keys1);
        simple_signer.add_keys(keys2);

        let start_identities: Vec<(Identity, Option<StateTransition>)> =
            create_state_transitions_for_identities(
                vec![&mut identity1, &mut identity2],
                &(dash_to_duffs!(1)..=dash_to_duffs!(1)),
                &simple_signer,
                &mut rng,
                platform_version,
            )
            .into_iter()
            .map(|(identity, transition)| (identity, Some(transition)))
            .collect();

        let contract = created_contract.data_contract_mut();
        let mut token_configuration = contract
            .token_configuration_mut(0)
            .expect("expected to get token configuration");
        token_configuration.set_minting_allow_choosing_destination(true);
        contract.set_owner_id(identity1.id());
        let new_id = DataContract::generate_data_contract_id_v0(identity1.id(), 1);
        contract.set_id(new_id);
        let token_id = contract.token_id(0).expect("expected to get token_id");

        let token_op = TokenOp {
            contract: contract.clone(),
            token_id,
            token_pos: 0,
            use_identity_with_id: Some(identity1.id()),
            action: TokenEvent::Mint(1000, identity2.id(), None),
        };

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![(created_contract, None)],
                operations: vec![Operation {
                    op_type: OperationType::Token(token_op),
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                }],
                start_identities: StartIdentities {
                    hard_coded: start_identities.clone(),
                    ..Default::default()
                },
                identity_inserts: IdentityInsertInfo::default(),

                identity_contract_nonce_gaps: None,
                signer: Some(simple_signer),
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
            verify_state_transition_results: true,
            ..Default::default()
        };
        let day_in_ms = 1000 * 60 * 60 * 24;
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig::default_100_67(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,

                ..Default::default()
            },
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };
        let block_count = 12;
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome =
            run_chain_for_strategy(&mut platform, block_count, strategy, config, 15, &mut None);

        let drive = &outcome.abci_app.platform.drive;
        let identity_ids = vec![identity1.id().to_buffer(), identity2.id().to_buffer()];
        let balances = drive
            .fetch_identities_token_balances(
                token_id.to_buffer(),
                identity_ids.as_slice(),
                None,
                platform_version,
            )
            .expect("expected to get balances");

        for (identity_id, token_balance) in balances {
            assert!(token_balance.is_some())
        }

        let identity_1_token_balance = drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity1.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch");
        let identity_2_token_balance = drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity2.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch");

        assert_eq!(identity_1_token_balance, Some(100000)); // The initial amount from creating the contract
        assert_eq!(identity_2_token_balance, Some(11000)); // 11 blocks of 1000
    }

    #[test]
    fn run_chain_insert_one_new_identity_per_block_and_a_token_transfer() {
        let platform_version = PlatformVersion::latest();
        let mut created_contract = json_document_to_created_contract(
            "tests/supporting_files/contract/basic-token/basic-token.json",
            1,
            true,
            platform_version,
        )
        .expect("expected to get contract from a json document");

        let mut rng = StdRng::seed_from_u64(567);

        let mut simple_signer = SimpleSigner::default();

        let (mut identity1, keys1) = Identity::random_identity_with_main_keys_with_private_key::<
            Vec<_>,
        >(2, &mut rng, platform_version)
        .unwrap();

        let (mut identity2, keys2) = Identity::random_identity_with_main_keys_with_private_key::<
            Vec<_>,
        >(2, &mut rng, platform_version)
        .unwrap();

        simple_signer.add_keys(keys1);
        simple_signer.add_keys(keys2);

        let start_identities: Vec<(Identity, Option<StateTransition>)> =
            create_state_transitions_for_identities(
                vec![&mut identity1, &mut identity2],
                &(dash_to_duffs!(1)..=dash_to_duffs!(1)),
                &simple_signer,
                &mut rng,
                platform_version,
            )
            .into_iter()
            .map(|(identity, transition)| (identity, Some(transition)))
            .collect();

        let contract = created_contract.data_contract_mut();
        let mut token_configuration = contract
            .token_configuration_mut(0)
            .expect("expected to get token configuration");
        token_configuration.set_minting_allow_choosing_destination(true);
        contract.set_owner_id(identity1.id());
        let new_id = DataContract::generate_data_contract_id_v0(identity1.id(), 1);
        contract.set_id(new_id);
        let token_id = contract.token_id(0).expect("expected to get token_id");

        let token_op = TokenOp {
            contract: contract.clone(),
            token_id,
            token_pos: 0,
            use_identity_with_id: Some(identity1.id()),
            action: TokenEvent::Transfer(identity2.id(), None, None, None, 1000),
        };

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![(created_contract, None)],
                operations: vec![Operation {
                    op_type: OperationType::Token(token_op),
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                }],
                start_identities: StartIdentities {
                    hard_coded: start_identities.clone(),
                    ..Default::default()
                },
                identity_inserts: IdentityInsertInfo::default(),

                identity_contract_nonce_gaps: None,
                signer: Some(simple_signer),
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
            verify_state_transition_results: true,
            ..Default::default()
        };
        let day_in_ms = 1000 * 60 * 60 * 24;
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig::default_100_67(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,

                ..Default::default()
            },
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };
        let block_count = 12;
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome =
            run_chain_for_strategy(&mut platform, block_count, strategy, config, 15, &mut None);

        let drive = &outcome.abci_app.platform.drive;
        let identity_ids = vec![identity1.id().to_buffer(), identity2.id().to_buffer()];
        let balances = drive
            .fetch_identities_token_balances(
                token_id.to_buffer(),
                identity_ids.as_slice(),
                None,
                platform_version,
            )
            .expect("expected to get balances");

        assert_eq!(
            balances.get(&identity1.id()).copied(),
            Some(Some(100000 - 11000))
        );
        assert_eq!(balances.get(&identity2.id()).copied(), Some(Some(11000)));

        // Let's also try this fetching

        let identity_1_token_balance = drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity1.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch");
        let identity_2_token_balance = drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity2.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch");

        assert_eq!(identity_1_token_balance, Some(100000 - 11000)); // The initial amount from creating the contract less 11 times 1000 that we transferred
        assert_eq!(identity_2_token_balance, Some(11000)); // 11 blocks of 1000
    }
}
