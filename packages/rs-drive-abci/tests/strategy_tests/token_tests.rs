#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use assert_matches::assert_matches;
    use crate::execution::run_chain_for_strategy;
    use crate::strategy::{ChainExecutionOutcome, NetworkStrategy};
    use dpp::dash_to_duffs;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::accessors::v1::DataContractV1Getters;
    use dpp::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
    use dpp::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Setters;
    use dpp::data_contract::DataContract;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::state_transition::StateTransition;
    use dpp::tests::json_document::json_document_to_created_contract;
    use dpp::tokens::token_event::TokenEvent;
    use drive_abci::config::{
        ChainLockConfig, ExecutionConfig, InstantLockConfig, PlatformConfig, PlatformTestConfig,
        ValidatorSetConfig,
    };
    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use platform_version::version::PlatformVersion;
    use rand::prelude::StdRng;
    use rand::SeedableRng;
    use rand::seq::IteratorRandom;
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::{Epoch, EpochIndex};
    use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;
    use dpp::block::finalized_epoch_info::FinalizedEpochInfo;
    use dpp::block::finalized_epoch_info::v0::getters::FinalizedEpochInfoGettersV0;
    use dpp::dashcore::hashes::Hash;
    use dpp::data_contract::associated_token::token_distribution_key::TokenDistributionType;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_function::DistributionFunction;
    use dpp::data_contract::associated_token::token_perpetual_distribution::distribution_recipient::TokenDistributionRecipient;
    use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
    use dpp::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
    use dpp::data_contract::associated_token::token_perpetual_distribution::v0::TokenPerpetualDistributionV0;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::batch_transition::BatchTransition;
    use dpp::state_transition::batch_transition::methods::v1::DocumentsBatchTransitionMethodsV1;
    use drive::drive::identity::key::fetch::{IdentityKeysRequest, KeyRequestType, OptionalSingleIdentityPublicKeyOutcome};
    use drive_abci::platform_types::platform_state::v0::PlatformStateV0Methods;
    use drive_abci::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
    use simple_signer::signer::SimpleSigner;
    use strategy_tests::frequency::Frequency;
    use strategy_tests::operations::{Operation, OperationType, TokenOp};
    use strategy_tests::transitions::create_state_transitions_for_identities;
    use strategy_tests::{IdentityInsertInfo, StartIdentities, Strategy};

    #[test]
    fn run_chain_insert_one_token_mint_per_block() {
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
        >(3, &mut rng, platform_version)
        .unwrap();

        let (mut identity2, keys2) = Identity::random_identity_with_main_keys_with_private_key::<
            Vec<_>,
        >(3, &mut rng, platform_version)
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
        let token_configuration = contract
            .token_configuration_mut(0)
            .expect("expected to get token configuration");
        token_configuration
            .distribution_rules_mut()
            .set_minting_allow_choosing_destination(true);
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

        let outcome = run_chain_for_strategy(
            &mut platform,
            block_count,
            strategy,
            config,
            15,
            &mut None,
            &mut None,
        );

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

        for (_identity_id, token_balance) in balances {
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
    fn run_chain_insert_one_token_transfer_per_block() {
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
        >(3, &mut rng, platform_version)
        .unwrap();

        let (mut identity2, keys2) = Identity::random_identity_with_main_keys_with_private_key::<
            Vec<_>,
        >(3, &mut rng, platform_version)
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
        let token_configuration = contract
            .token_configuration_mut(0)
            .expect("expected to get token configuration");
        token_configuration
            .distribution_rules_mut()
            .set_minting_allow_choosing_destination(true);
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

        let outcome = run_chain_for_strategy(
            &mut platform,
            block_count,
            strategy,
            config,
            15,
            &mut None,
            &mut None,
        );

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

    #[test]
    fn run_chain_token_perpetual_distribution_to_evonodes_fixed_distribution() {
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
        >(3, &mut rng, platform_version)
        .unwrap();

        simple_signer.add_keys(keys1);

        let start_identities: Vec<(Identity, Option<StateTransition>)> =
            create_state_transitions_for_identities(
                vec![&mut identity1],
                &(dash_to_duffs!(1)..=dash_to_duffs!(1)),
                &simple_signer,
                &mut rng,
                platform_version,
            )
            .into_iter()
            .map(|(identity, transition)| (identity, Some(transition)))
            .collect();

        let contract = created_contract.data_contract_mut();
        let token_configuration = contract
            .token_configuration_mut(0)
            .expect("expected to get token configuration");
        token_configuration
            .distribution_rules_mut()
            .set_perpetual_distribution(Some(TokenPerpetualDistribution::V0(
                TokenPerpetualDistributionV0 {
                    distribution_type: RewardDistributionType::EpochBasedDistribution {
                        interval: 1, // every epoch, we should split 1000 tokens between evonodes
                        function: DistributionFunction::FixedAmount { amount: 1000 },
                    },
                    distribution_recipient: TokenDistributionRecipient::EvonodesByParticipation,
                },
            )));
        contract.set_owner_id(identity1.id());
        let contract_id = DataContract::generate_data_contract_id_v0(identity1.id(), 1);
        contract.set_id(contract_id);
        let token_id = contract.token_id(0).expect("expected to get token_id");

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![(created_contract, None)],
                operations: vec![],
                start_identities: StartIdentities {
                    hard_coded: start_identities.clone(),
                    ..Default::default()
                },
                identity_inserts: IdentityInsertInfo::default(),

                identity_contract_nonce_gaps: None,
                signer: Some(simple_signer),
            },
            total_hpmns: 10,
            extra_normal_mns: 3,
            validator_quorum_count: 2,
            chain_lock_quorum_count: 1,
            upgrading_info: None,

            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: true,
            ..Default::default()
        };
        let half_day_in_ms = 1000 * 60 * 60 * 12;
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig::default_100_67(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,

                ..Default::default()
            },
            block_spacing_ms: half_day_in_ms,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .with_initial_protocol_version(9)
            .build_with_mock_rpc();

        let mut transfer_key_signer = Some(SimpleSigner::default());

        let block_count = 60;

        let ChainExecutionOutcome {
            abci_app,
            proposers,
            validator_quorums: quorums,
            current_validator_quorum_hash: current_quorum_hash,
            current_proposer_versions,
            end_time_ms,
            identity_nonce_counter,
            identity_contract_nonce_counter,
            instant_lock_quorums,
            ..
        } = run_chain_for_strategy(
            &mut platform,
            block_count, // block count is 30
            strategy.clone(),
            config.clone(),
            13,
            &mut None,
            &mut transfer_key_signer,
        );

        let platform = abci_app.platform;
        let state = platform.state.load();

        {
            let counter = platform.drive.cache.protocol_versions_counter.read();
            platform
                .drive
                .fetch_versions_with_counter(None, &platform_version.drive)
                .expect("expected to get versions");

            assert_eq!(
                state
                    .last_committed_block_info()
                    .as_ref()
                    .unwrap()
                    .basic_info()
                    .epoch
                    .index,
                3
            );
        }

        let hpmn = state
            .hpmn_masternode_list()
            .values()
            .choose(&mut rng)
            .expect("expected a hpmn");
        let identity_id_buffer = hpmn.pro_tx_hash.to_byte_array();

        let identity_key_request = IdentityKeysRequest {
            identity_id: identity_id_buffer,
            request_type: KeyRequestType::RecentWithdrawalKeys,
            limit: None,
            offset: None,
        };

        let maybe_key: OptionalSingleIdentityPublicKeyOutcome = platform
            .drive
            .fetch_identity_keys(identity_key_request, None, platform_version)
            .expect("expected to fetch partial identity");

        let key = maybe_key.expect("expected a key");

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity_id_buffer,
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");

        assert_eq!(token_balance, None); // the hpmn has no token balance

        // let's see how many blocks were proposed in each epoch
        let epochs: BTreeMap<EpochIndex, FinalizedEpochInfo> = platform
            .drive
            .get_finalized_epoch_infos(0, true, 3, false, None, platform_version)
            .expect("expected to get epoch infos");
        let epoch_0 = epochs.get(&0).expect("expected to find epoch 0");
        let epoch_1 = epochs.get(&1).expect("expected to find epoch 1");
        let epoch_2 = epochs.get(&2).expect("expected to find epoch 2");

        assert_eq!(epoch_0.total_blocks_in_epoch(), 19);
        assert_eq!(epoch_1.total_blocks_in_epoch(), 18);
        assert_eq!(epoch_2.total_blocks_in_epoch(), 18);

        assert_eq!(
            epoch_0.block_proposers().get(&identity_id_buffer.into()),
            Some(&2)
        ); // we proposed 2 blocks
        assert_eq!(
            epoch_1.block_proposers().get(&identity_id_buffer.into()),
            Some(&2)
        ); // we proposed 2 blocks
        assert_eq!(
            epoch_2.block_proposers().get(&identity_id_buffer.into()),
            Some(&2)
        ); // we proposed 2 blocks

        let claim_transition = BatchTransition::new_token_claim_transition(
            token_id,
            identity_id_buffer.into(),
            contract_id,
            0,
            TokenDistributionType::Perpetual,
            None,
            &key,
            1,
            0,
            &transfer_key_signer.unwrap(),
            platform_version,
            None,
        )
        .expect("expect to create documents batch transition");

        let claim_serialized_transition = claim_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .process_raw_state_transitions(
                &vec![claim_serialized_transition.clone()],
                &state,
                &BlockInfo {
                    time_ms: state
                        .last_committed_block_time_ms()
                        .expect("expected to get committed block time")
                        + 1000,
                    height: state.last_committed_block_height() + 1,
                    core_height: state.last_committed_core_height() + 1,
                    epoch: Epoch::new(3).unwrap(),
                },
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

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity_id_buffer,
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");

        // there were 2 epochs that should have distributed 1000 tokens each
        // We had 3 epochs with 19, 18 and 18 blocks in them, the first epoch will not count though.
        // He proposed 2 blocks in first epoch (won't count), two in second and two in the third.
        // So this identity proposed 4 out of 36 blocks.
        // They should get 2 * 1000 * 4 / 36
        // This is equal to 250

        assert_eq!(token_balance, Some(222));
    }

    #[test]
    fn run_chain_token_perpetual_distribution_to_evonodes_linear_distribution() {
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
        >(3, &mut rng, platform_version)
        .unwrap();

        simple_signer.add_keys(keys1);

        let start_identities: Vec<(Identity, Option<StateTransition>)> =
            create_state_transitions_for_identities(
                vec![&mut identity1],
                &(dash_to_duffs!(1)..=dash_to_duffs!(1)),
                &simple_signer,
                &mut rng,
                platform_version,
            )
            .into_iter()
            .map(|(identity, transition)| (identity, Some(transition)))
            .collect();

        let contract = created_contract.data_contract_mut();
        let token_configuration = contract
            .token_configuration_mut(0)
            .expect("expected to get token configuration");
        token_configuration
            .distribution_rules_mut()
            .set_perpetual_distribution(Some(TokenPerpetualDistribution::V0(
                TokenPerpetualDistributionV0 {
                    distribution_type: RewardDistributionType::EpochBasedDistribution {
                        interval: 1, // every epoch, we should split 1000 tokens between evonodes
                        function: DistributionFunction::Linear {
                            a: 100,
                            d: 1,
                            start_step: None,
                            starting_amount: 500,
                            min_value: None,
                            max_value: None,
                        },
                    },
                    distribution_recipient: TokenDistributionRecipient::EvonodesByParticipation,
                },
            )));
        contract.set_owner_id(identity1.id());
        let contract_id = DataContract::generate_data_contract_id_v0(identity1.id(), 1);
        contract.set_id(contract_id);
        let token_id = contract.token_id(0).expect("expected to get token_id");

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![(created_contract, None)],
                operations: vec![],
                start_identities: StartIdentities {
                    hard_coded: start_identities.clone(),
                    ..Default::default()
                },
                identity_inserts: IdentityInsertInfo::default(),

                identity_contract_nonce_gaps: None,
                signer: Some(simple_signer),
            },
            total_hpmns: 12,
            extra_normal_mns: 3,
            validator_quorum_count: 2,
            chain_lock_quorum_count: 1,
            upgrading_info: None,

            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: true,
            ..Default::default()
        };
        let half_day_in_ms = 1000 * 60 * 60 * 12;
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig::default_100_67(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,

                ..Default::default()
            },
            block_spacing_ms: half_day_in_ms,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .with_initial_protocol_version(9)
            .build_with_mock_rpc();

        let mut transfer_key_signer = Some(SimpleSigner::default());

        let block_count = 60;

        let ChainExecutionOutcome {
            abci_app,
            proposers,
            validator_quorums: quorums,
            current_validator_quorum_hash: current_quorum_hash,
            current_proposer_versions,
            end_time_ms,
            identity_nonce_counter,
            identity_contract_nonce_counter,
            instant_lock_quorums,
            state_transition_results_per_block,
            ..
        } = run_chain_for_strategy(
            &mut platform,
            block_count, // block count is 30
            strategy.clone(),
            config.clone(),
            13,
            &mut None,
            &mut transfer_key_signer,
        );

        for (block, tx_results_per_block) in state_transition_results_per_block.iter() {
            for (state_transition, result) in tx_results_per_block {
                assert_eq!(
                    result.code, 0,
                    "state transition got code {} : {:?} in block {}",
                    result.code, state_transition, block
                );
            }
        }

        let platform = abci_app.platform;
        let state = platform.state.load();

        {
            let counter = platform.drive.cache.protocol_versions_counter.read();
            platform
                .drive
                .fetch_versions_with_counter(None, &platform_version.drive)
                .expect("expected to get versions");

            assert_eq!(
                state
                    .last_committed_block_info()
                    .as_ref()
                    .unwrap()
                    .basic_info()
                    .epoch
                    .index,
                3
            );
        }

        let hpmn = state
            .hpmn_masternode_list()
            .values()
            .choose(&mut rng)
            .expect("expected a hpmn");
        let identity_id_buffer = hpmn.pro_tx_hash.to_byte_array();

        let identity_key_request = IdentityKeysRequest {
            identity_id: identity_id_buffer,
            request_type: KeyRequestType::RecentWithdrawalKeys,
            limit: None,
            offset: None,
        };

        let maybe_key: OptionalSingleIdentityPublicKeyOutcome = platform
            .drive
            .fetch_identity_keys(identity_key_request, None, platform_version)
            .expect("expected to fetch partial identity");

        let key = maybe_key.expect("expected a key");

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity_id_buffer,
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");

        assert_eq!(token_balance, None); // the hpmn has no token balance

        // let's see how many blocks were proposed in each epoch
        let epochs: BTreeMap<EpochIndex, FinalizedEpochInfo> = platform
            .drive
            .get_finalized_epoch_infos(0, true, 3, false, None, platform_version)
            .expect("expected to get epoch infos");
        let epoch_0 = epochs.get(&0).expect("expected to find epoch 0");
        let epoch_1 = epochs.get(&1).expect("expected to find epoch 1");
        let epoch_2 = epochs.get(&2).expect("expected to find epoch 2");

        assert_eq!(epoch_0.total_blocks_in_epoch(), 19);
        assert_eq!(epoch_1.total_blocks_in_epoch(), 18);
        assert_eq!(epoch_2.total_blocks_in_epoch(), 18);

        assert_eq!(
            epoch_0.block_proposers().get(&identity_id_buffer.into()),
            Some(&2)
        ); // we proposed 2 blocks
        assert_eq!(
            epoch_1.block_proposers().get(&identity_id_buffer.into()),
            Some(&1)
        ); // we proposed 1 block
        assert_eq!(
            epoch_2.block_proposers().get(&identity_id_buffer.into()),
            Some(&2)
        ); // we proposed 2 blocks

        let claim_transition = BatchTransition::new_token_claim_transition(
            token_id,
            identity_id_buffer.into(),
            contract_id,
            0,
            TokenDistributionType::Perpetual,
            None,
            &key,
            1,
            0,
            &transfer_key_signer.unwrap(),
            platform_version,
            None,
        )
        .expect("expect to create documents batch transition");

        let claim_serialized_transition = claim_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .process_raw_state_transitions(
                &vec![claim_serialized_transition.clone()],
                &state,
                &BlockInfo {
                    time_ms: state
                        .last_committed_block_time_ms()
                        .expect("expected to get committed block time")
                        + 1000,
                    height: state.last_committed_block_height() + 1,
                    core_height: state.last_committed_core_height() + 1,
                    epoch: Epoch::new(3).unwrap(),
                },
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

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                token_id.to_buffer(),
                identity_id_buffer,
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");

        // there were 2 epochs that should have distributed tokens each
        // Epoch 1: f(1) = 100 * 1 + 500 = 600
        // Epoch 2: f(2) = 100 * 2 + 500 = 700
        // We had 3 epochs with 19, 18 and 18 blocks in them, the first epoch will not count though.
        // He proposed 2 blocks in first epoch (won't count), one in second and two in the third.
        // They should get 600 * 1 / 18 = 33
        // + 700 * 2 / 18 = 77
        // This is equal to 110

        assert_eq!(token_balance, Some(110));
    }
}
