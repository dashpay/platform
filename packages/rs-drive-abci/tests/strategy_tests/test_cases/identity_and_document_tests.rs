#[cfg(test)]
mod tests {

    use crate::execution::run_chain_for_strategy;
    use crate::query::QueryStrategy;
    use crate::strategy::NetworkStrategy;
    use dash_platform_macros::stack_size;
    use dpp::dash_to_duffs;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::document_type::random_document::{
        DocumentFieldFillSize, DocumentFieldFillType,
    };
    use dpp::data_contract::document_type::v0::random_document_type::{
        FieldMinMaxBounds, FieldTypeWeights, RandomDocumentTypeParameters,
    };
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::Identity;
    use dpp::state_transition::StateTransition;
    use dpp::tests::json_document::json_document_to_created_contract;
    use drive_abci::config::{
        ChainLockConfig, ExecutionConfig, InstantLockConfig, PlatformConfig, PlatformTestConfig,
        ValidatorSetConfig,
    };
    use drive_abci::logging::LogLevel;
    use drive_abci::platform_types::platform_state::PlatformStateV0Methods;
    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use platform_version::version::v1::PROTOCOL_VERSION_1;
    use platform_version::version::PlatformVersion;
    use rand::prelude::StdRng;
    use rand::SeedableRng;
    use simple_signer::signer::SimpleSigner;
    use std::collections::BTreeMap;
    use strategy_tests::frequency::Frequency;
    use strategy_tests::operations::DocumentAction::{
        DocumentActionReplaceRandom, DocumentActionTransferRandom,
    };
    use strategy_tests::operations::{DocumentAction, DocumentOp, Operation, OperationType};
    use strategy_tests::transitions::create_state_transitions_for_identities;
    use strategy_tests::{IdentityInsertInfo, StartAddresses, StartIdentities, Strategy};

    #[test]
    fn run_chain_one_identity_in_solitude_first_protocol_version() {
        let platform_version = PlatformVersion::first();
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
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
            .with_initial_protocol_version(1)
            .build_with_mock_rpc();

        let outcome =
            run_chain_for_strategy(&mut platform, 2, strategy, config, 15, &mut None, &mut None);

        let balance = outcome
            .abci_app
            .platform
            .drive
            .fetch_identity_balance(
                outcome.identities.first().unwrap().id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch balances")
            .expect("expected to have an identity to get balance from");

        assert_eq!(balance, 99864012200)
    }

    #[test]
    fn run_chain_one_identity_in_solitude_latest_protocol_version() {
        // This is different because in the root tree we added GroupActions
        //                                                                                DataContract_Documents 64
        //                                 /                                                                                                       \
        //                       Identities 32                                                                                                 Balances 96
        //             /                            \                                                                        /                                                       \
        //   Token_Balances 16                    Pools 48                                                    WithdrawalTransactions 80                                                Votes  112
        //       /      \                           /                     \                                         /                           \                            /                          \
        //     NUPKH->I 8 UPKH->I 24   PreFundedSpecializedBalances 40  Masternode Lists 56 (reserved)     SpentAssetLockTransactions 72    GroupActions 88             Misc 104                        Versions 120

        // This will cause the costs of insertion of a spent asset lock transition, since group actions now exist we will see a slight difference in processing costs
        // This is because WithdrawalTransactions will have a right element in the tree.

        let platform_version = PlatformVersion::latest();
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
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
            .build_with_mock_rpc();

        let outcome =
            run_chain_for_strategy(&mut platform, 2, strategy, config, 15, &mut None, &mut None);

        let balance = outcome
            .abci_app
            .platform
            .drive
            .fetch_identity_balance(
                outcome.identities.first().unwrap().id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch balances")
            .expect("expected to have an identity to get balance from");

        assert_eq!(balance, 99864009940)
    }

    #[test]
    fn run_chain_insert_one_new_identity_per_block_with_block_signing() {
        drive_abci::logging::init_for_tests(LogLevel::Silent);

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
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
            query_testing: Some(QueryStrategy {
                query_identities_by_public_key_hashes: Frequency {
                    times_per_block_range: 1..5,
                    chance_per_block: None,
                },
            }),
            verify_state_transition_results: true,
            sign_instant_locks: true,
            ..Default::default()
        };
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig::default_100_67(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default(),
            execution: ExecutionConfig {
                verify_sum_trees: true,

                ..Default::default()
            },
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(
            &mut platform,
            100,
            strategy,
            config,
            15,
            &mut None,
            &mut None,
        );

        assert_eq!(outcome.identities.len(), 100);
    }

    #[test]
    fn run_chain_insert_one_new_identity_per_block_with_epoch_change() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
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

        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .with_initial_protocol_version(PROTOCOL_VERSION_1)
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(
            &mut platform,
            150,
            strategy,
            config,
            15,
            &mut None,
            &mut None,
        );
        assert_eq!(outcome.identities.len(), 150);
        assert_eq!(outcome.masternode_identity_balances.len(), 100);
        let all_have_balances = outcome
            .masternode_identity_balances
            .iter()
            .all(|(_, balance)| *balance != 0);
        assert!(all_have_balances, "all masternodes should have a balance");

        let state = outcome.abci_app.platform.state.load();
        let protocol_version = state.current_protocol_version_in_consensus();
        let platform_version =
            PlatformVersion::get(protocol_version).expect("expected platform version");

        assert_eq!(
            hex::encode(
                outcome
                    .abci_app
                    .platform
                    .drive
                    .grove
                    .root_hash(None, &platform_version.drive.grove_version)
                    .unwrap()
                    .unwrap()
            ),
            "975735252c11cea7ef3fbba86928077e37ebe1926972e6ae38e237ce0864100c".to_string()
        )
    }

    #[test]
    fn run_chain_insert_one_new_identity_and_a_contract() {
        let platform_version = PlatformVersion::latest();
        let contract = json_document_to_created_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            1,
            true,
            platform_version,
        )
        .expect("expected to get contract from a json document");

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![(contract, None)],
                operations: vec![],
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
            .build_with_mock_rpc();

        let outcome =
            run_chain_for_strategy(&mut platform, 1, strategy, config, 15, &mut None, &mut None);

        for tx_results_per_block in outcome.state_transition_results_per_block.values() {
            for (state_transition, result) in tx_results_per_block {
                assert_eq!(
                    result.code, 0,
                    "state transition got code {} : {:?}",
                    result.code, state_transition
                );
            }
        }

        outcome
            .abci_app
            .platform
            .drive
            .fetch_contract(
                outcome
                    .strategy
                    .strategy
                    .start_contracts
                    .first()
                    .unwrap()
                    .0
                    .data_contract()
                    .id()
                    .to_buffer(),
                None,
                None,
                None,
                platform_version,
            )
            .unwrap()
            .expect("expected to execute the fetch of a contract")
            .expect("expected to get a contract");
    }

    #[test]
    fn run_chain_insert_one_new_identity_and_many_big_contracts() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![Operation {
                    op_type: OperationType::ContractCreate(
                        RandomDocumentTypeParameters {
                            new_fields_optional_count_range: 1..30,
                            new_fields_required_count_range: 1..40,
                            new_indexes_count_range: 10..11,
                            field_weights: FieldTypeWeights {
                                string_weight: 50,
                                float_weight: 50,
                                integer_weight: 50,
                                date_weight: 50,
                                boolean_weight: 20,
                                byte_array_weight: 70,
                            },
                            field_bounds: FieldMinMaxBounds {
                                string_min_len: 1..10,
                                string_has_min_len_chance: 0.5,
                                string_max_len: 10..63,
                                string_has_max_len_chance: 0.5,
                                integer_min: 1..10,
                                integer_has_min_chance: 0.5,
                                integer_max: 10..10000,
                                integer_has_max_chance: 0.5,
                                float_min: 0.1..10.0,
                                float_has_min_chance: 0.5,
                                float_max: 10.0..1000.0,
                                float_has_max_chance: 0.5,
                                date_min: 0,
                                date_max: 0,
                                byte_array_min_len: 1..10,
                                byte_array_has_min_len_chance: 0.0,
                                byte_array_max_len: 10..255,
                                byte_array_has_max_len_chance: 0.0,
                            },
                            keep_history_chance: 1.0,
                            documents_mutable_chance: 1.0,
                            documents_can_be_deleted_chance: 1.0,
                        },
                        30..31,
                    ),
                    frequency: Frequency {
                        times_per_block_range: 30..31,
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
            .build_with_mock_rpc();

        run_chain_for_strategy(
            &mut platform,
            30,
            strategy,
            config,
            15,
            &mut None,
            &mut None,
        );
    }

    #[test]
    #[stack_size(4 * 1024 * 1024)]
    fn run_chain_insert_one_new_identity_and_a_contract_with_updates() {
        let platform_version = PlatformVersion::latest();
        let contract = json_document_to_created_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            1,
            true,
            platform_version,
        )
        .expect("expected to get contract from a json document");

        let mut contract_update_1 = json_document_to_created_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable-update-1.json",
            1,
            true,
            platform_version,
        )
        .expect("expected to get contract from a json document");

        //todo: versions should start at 0 (so this should be 1)
        contract_update_1.data_contract_mut().set_version(2);

        let mut contract_update_2 = json_document_to_created_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable-update-2.json",
            1,
            true,
            platform_version,
        )
        .expect("expected to get contract from a json document");

        contract_update_2.data_contract_mut().set_version(3);

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![(
                    contract,
                    Some(BTreeMap::from([
                        (3, contract_update_1),
                        (8, contract_update_2),
                    ])),
                )],
                operations: vec![],
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
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(
            &mut platform,
            10,
            strategy,
            config,
            15,
            &mut None,
            &mut None,
        );

        outcome
            .abci_app
            .platform
            .drive
            .fetch_contract(
                outcome
                    .strategy
                    .strategy
                    .start_contracts
                    .first()
                    .unwrap()
                    .0
                    .data_contract()
                    .id()
                    .to_buffer(),
                None,
                None,
                None,
                platform_version,
            )
            .unwrap()
            .expect("expected to execute the fetch of a contract")
            .expect("expected to get a contract");
    }

    #[test]
    fn run_chain_insert_one_new_identity_per_block_and_one_new_document() {
        let platform_version = PlatformVersion::latest();
        let created_contract = json_document_to_created_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            1,
            true,
            platform_version,
        )
        .expect("expected to get contract from a json document");

        let contract = created_contract.data_contract();

        let document_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentAction::DocumentActionInsertRandom(
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
            ),
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .to_owned_document_type(),
        };

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![(created_contract, None)],
                operations: vec![Operation {
                    op_type: OperationType::Document(document_op),
                    frequency: Frequency {
                        times_per_block_range: 1..2,
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
            .build_with_mock_rpc();

        run_chain_for_strategy(
            &mut platform,
            100,
            strategy,
            config,
            15,
            &mut None,
            &mut None,
        );
    }

    #[test]
    fn run_chain_insert_one_new_identity_per_block_and_a_document_with_epoch_change() {
        let platform_version = PlatformVersion::latest();
        let created_contract = json_document_to_created_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            1,
            true,
            platform_version,
        )
        .expect("expected to get contract from a json document");

        let contract = created_contract.data_contract();

        let document_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentAction::DocumentActionInsertRandom(
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
            ),
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .to_owned_document_type(),
        };

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![(created_contract, None)],
                operations: vec![Operation {
                    op_type: OperationType::Document(document_op),
                    frequency: Frequency {
                        times_per_block_range: 1..2,
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
        let block_count = 120;
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
        assert_eq!(outcome.identities.len() as u64, block_count);
        assert_eq!(outcome.masternode_identity_balances.len(), 100);
        let all_have_balances = outcome
            .masternode_identity_balances
            .iter()
            .all(|(_, balance)| *balance != 0);
        assert!(all_have_balances, "all masternodes should have a balance");
    }

    #[test]
    #[stack_size(4 * 1024 * 1024)]
    fn run_chain_insert_one_new_identity_per_block_document_insertions_and_deletions_with_epoch_change(
    ) {
        let platform_version = PlatformVersion::latest();
        let created_contract = json_document_to_created_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            1,
            true,
            platform_version,
        )
        .expect("expected to get contract from a json document");

        let contract = created_contract.data_contract();

        let document_insertion_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentAction::DocumentActionInsertRandom(
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
            ),
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .to_owned_document_type(),
        };

        let document_deletion_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentAction::DocumentActionDelete,
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .to_owned_document_type(),
        };

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![(created_contract, None)],
                operations: vec![
                    Operation {
                        op_type: OperationType::Document(document_insertion_op),
                        frequency: Frequency {
                            times_per_block_range: 1..2,
                            chance_per_block: None,
                        },
                    },
                    Operation {
                        op_type: OperationType::Document(document_deletion_op),
                        frequency: Frequency {
                            times_per_block_range: 1..2,
                            chance_per_block: None,
                        },
                    },
                ],
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
        let block_count = 120;
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
        assert_eq!(outcome.identities.len() as u64, block_count);
        assert_eq!(outcome.masternode_identity_balances.len(), 100);
        let all_have_balances = outcome
            .masternode_identity_balances
            .iter()
            .all(|(_, balance)| *balance != 0);
        assert!(all_have_balances, "all masternodes should have a balance");

        let issues = outcome
            .abci_app
            .platform
            .drive
            .grove
            .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
            .expect("expected to have no issues");

        assert_eq!(
            issues.len(),
            0,
            "issues are {}",
            issues
                .iter()
                .map(|(hash, (a, b, c))| format!("{}: {} {} {}", hash, a, b, c))
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }

    #[test]
    fn run_chain_insert_one_new_identity_per_block_many_document_insertions_and_deletions_with_epoch_change(
    ) {
        let platform_version = PlatformVersion::latest();
        let created_contract = json_document_to_created_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            1,
            true,
            platform_version,
        )
        .expect("expected to get contract from a json document");

        let contract = created_contract.data_contract();

        let document_insertion_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentAction::DocumentActionInsertRandom(
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
            ),
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .to_owned_document_type(),
        };

        let document_deletion_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentAction::DocumentActionDelete,
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .to_owned_document_type(),
        };

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![(created_contract, None)],
                operations: vec![
                    Operation {
                        op_type: OperationType::Document(document_insertion_op),
                        frequency: Frequency {
                            times_per_block_range: 1..7,
                            chance_per_block: None,
                        },
                    },
                    Operation {
                        op_type: OperationType::Document(document_deletion_op),
                        frequency: Frequency {
                            times_per_block_range: 1..7,
                            chance_per_block: None,
                        },
                    },
                ],
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

        let block_count = 120;
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .with_initial_protocol_version(PROTOCOL_VERSION_1)
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
        assert_eq!(outcome.identities.len() as u64, block_count);
        assert_eq!(outcome.masternode_identity_balances.len(), 100);
        let all_have_balances = outcome
            .masternode_identity_balances
            .iter()
            .all(|(_, balance)| *balance != 0);
        assert!(all_have_balances, "all masternodes should have a balance");
        let state = outcome.abci_app.platform.state.load();
        let protocol_version = state.current_protocol_version_in_consensus();
        let platform_version =
            PlatformVersion::get(protocol_version).expect("expected platform version");
        assert_eq!(
            hex::encode(
                outcome
                    .abci_app
                    .platform
                    .drive
                    .grove
                    .root_hash(None, &platform_version.drive.grove_version)
                    .unwrap()
                    .unwrap()
            ),
            "0cc2c7a7749a0ce47a4abcd1f4db21d07734f96d09ffe08d6500a8d09a3455a1".to_string()
        )
    }

    #[test]
    fn run_chain_insert_one_new_identity_per_block_many_document_insertions_and_deletions_with_nonce_gaps_with_epoch_change(
    ) {
        let platform_version = PlatformVersion::latest();
        let created_contract = json_document_to_created_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            1,
            true,
            platform_version,
        )
        .expect("expected to get contract from a json document");

        let contract = created_contract.data_contract();

        let document_insertion_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentAction::DocumentActionInsertRandom(
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
            ),
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .to_owned_document_type(),
        };

        let document_deletion_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentAction::DocumentActionDelete,
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .to_owned_document_type(),
        };

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![(created_contract, None)],
                operations: vec![
                    Operation {
                        op_type: OperationType::Document(document_insertion_op),
                        frequency: Frequency {
                            times_per_block_range: 1..7,
                            chance_per_block: None,
                        },
                    },
                    Operation {
                        op_type: OperationType::Document(document_deletion_op),
                        frequency: Frequency {
                            times_per_block_range: 1..7,
                            chance_per_block: None,
                        },
                    },
                ],
                start_identities: StartIdentities::default(),
                start_addresses: StartAddresses::default(),
                identity_inserts: IdentityInsertInfo {
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                    ..Default::default()
                },

                identity_contract_nonce_gaps: Some(Frequency {
                    times_per_block_range: 1..3,
                    chance_per_block: Some(0.5),
                }),
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

        let block_count = 120;
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .with_initial_protocol_version(PROTOCOL_VERSION_1)
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
        assert_eq!(outcome.identities.len() as u64, block_count);
        assert_eq!(outcome.masternode_identity_balances.len(), 100);
        let all_have_balances = outcome
            .masternode_identity_balances
            .iter()
            .all(|(_, balance)| *balance != 0);
        assert!(all_have_balances, "all masternodes should have a balance");
        let state = outcome.abci_app.platform.state.load();
        let protocol_version = state.current_protocol_version_in_consensus();
        let platform_version =
            PlatformVersion::get(protocol_version).expect("expected platform version");
        assert_eq!(
            hex::encode(
                outcome
                    .abci_app
                    .platform
                    .drive
                    .grove
                    .root_hash(None, &platform_version.drive.grove_version)
                    .unwrap()
                    .unwrap()
            ),
            "5a08b133a19b11b09eaba6763ad2893c2bcbcc645fb698298790bb5d26e551e0".to_string()
        )
    }

    #[test]
    fn run_chain_insert_one_new_identity_per_block_many_document_insertions_and_deletions_with_max_nonce_gaps_with_epoch_change(
    ) {
        let platform_version = PlatformVersion::latest();
        let created_contract = json_document_to_created_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            1,
            true,
            platform_version,
        )
        .expect("expected to get contract from a json document");

        let contract = created_contract.data_contract();

        let document_insertion_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentAction::DocumentActionInsertRandom(
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
            ),
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .to_owned_document_type(),
        };

        let document_deletion_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentAction::DocumentActionDelete,
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .to_owned_document_type(),
        };

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![(created_contract, None)],
                operations: vec![
                    Operation {
                        op_type: OperationType::Document(document_insertion_op),
                        frequency: Frequency {
                            times_per_block_range: 1..7,
                            chance_per_block: None,
                        },
                    },
                    Operation {
                        op_type: OperationType::Document(document_deletion_op),
                        frequency: Frequency {
                            times_per_block_range: 1..7,
                            chance_per_block: None,
                        },
                    },
                ],
                start_identities: StartIdentities::default(),
                start_addresses: StartAddresses::default(),
                identity_inserts: IdentityInsertInfo {
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                    ..Default::default()
                },

                identity_contract_nonce_gaps: Some(Frequency {
                    times_per_block_range: 24..25,
                    chance_per_block: Some(1.0),
                }),
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

        let block_count = 10;
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
        for tx_results_per_block in outcome.state_transition_results_per_block.values() {
            for (state_transition, result) in tx_results_per_block {
                assert_eq!(
                    result.code, 0,
                    "state transition got code {} : {:?}",
                    result.code, state_transition
                );
            }
        }
    }

    #[test]
    fn run_chain_insert_one_new_identity_per_block_many_document_insertions_and_deletions_with_higher_than_max_nonce_gaps_with_epoch_change(
    ) {
        let platform_version = PlatformVersion::latest();
        let created_contract = json_document_to_created_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            1,
            true,
            platform_version,
        )
        .expect("expected to get contract from a json document");

        let contract = created_contract.data_contract();

        let document_insertion_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentAction::DocumentActionInsertRandom(
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
            ),
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .to_owned_document_type(),
        };

        let document_deletion_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentAction::DocumentActionDelete,
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .to_owned_document_type(),
        };

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![(created_contract, None)],
                operations: vec![
                    Operation {
                        op_type: OperationType::Document(document_insertion_op),
                        frequency: Frequency {
                            times_per_block_range: 1..7,
                            chance_per_block: None,
                        },
                    },
                    Operation {
                        op_type: OperationType::Document(document_deletion_op),
                        frequency: Frequency {
                            times_per_block_range: 1..7,
                            chance_per_block: None,
                        },
                    },
                ],
                start_identities: StartIdentities::default(),
                start_addresses: StartAddresses::default(),
                identity_inserts: IdentityInsertInfo {
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                    ..Default::default()
                },

                identity_contract_nonce_gaps: Some(Frequency {
                    times_per_block_range: 25..26,
                    chance_per_block: Some(1.0),
                }),
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

        let block_count = 10;
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
        for tx_results_per_block in outcome.state_transition_results_per_block.values() {
            for (state_transition, _unused_result) in tx_results_per_block {
                // We can't ever get a documents batch transition, because the proposer will remove it from a block
                assert!(!matches!(state_transition, StateTransition::Batch(_)));
            }
        }
    }

    #[test]
    #[stack_size(4 * 1024 * 1024)]
    fn run_chain_insert_many_new_identity_per_block_many_document_insertions_and_deletions_with_epoch_change(
    ) {
        let platform_version = PlatformVersion::latest();
        let created_contract = json_document_to_created_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            1,
            true,
            platform_version,
        )
        .expect("expected to get contract from a json document");

        let contract = created_contract.data_contract();

        let document_insertion_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentAction::DocumentActionInsertRandom(
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
            ),
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .to_owned_document_type(),
        };

        let document_deletion_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentAction::DocumentActionDelete,
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .to_owned_document_type(),
        };

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![(created_contract, None)],
                operations: vec![
                    Operation {
                        op_type: OperationType::Document(document_insertion_op),
                        frequency: Frequency {
                            times_per_block_range: 1..40,
                            chance_per_block: None,
                        },
                    },
                    Operation {
                        op_type: OperationType::Document(document_deletion_op),
                        frequency: Frequency {
                            times_per_block_range: 1..15,
                            chance_per_block: None,
                        },
                    },
                ],
                start_identities: StartIdentities::default(),
                start_addresses: StartAddresses::default(),
                identity_inserts: IdentityInsertInfo {
                    frequency: Frequency {
                        times_per_block_range: 1..30,
                        chance_per_block: None,
                    },
                    start_keys: 5,
                    extra_keys: Default::default(),
                    start_balance_range: dash_to_duffs!(1)..=dash_to_duffs!(1),
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

                epoch_time_length_s: 1576800,
                ..Default::default()
            },
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };
        let block_count = 30;
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
        assert_eq!(outcome.identities.len() as u64, 472);
        assert_eq!(outcome.masternode_identity_balances.len(), 100);
        let balance_count = outcome
            .masternode_identity_balances
            .into_iter()
            .filter(|(_, balance)| *balance != 0)
            .count();
        assert_eq!(balance_count, 19); // 1 epoch worth of proposers

        let issues = outcome
            .abci_app
            .platform
            .drive
            .grove
            .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
            .expect("expected to have no issues");

        assert_eq!(
            issues.len(),
            0,
            "issues are {}",
            issues
                .iter()
                .map(|(hash, (a, b, c))| format!("{}: {} {} {}", hash, a, b, c))
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }

    #[test]
    #[stack_size(4 * 1024 * 1024)]
    fn run_chain_insert_many_new_identity_per_block_many_document_insertions_and_updates_with_epoch_change(
    ) {
        let platform_version = PlatformVersion::latest();
        let created_contract = json_document_to_created_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            1,
            true,
            platform_version,
        )
        .expect("expected to get contract from a json document");

        let contract = created_contract.data_contract();

        let document_insertion_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentAction::DocumentActionInsertRandom(
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
            ),
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .to_owned_document_type(),
        };

        let document_replace_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentActionReplaceRandom,
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .to_owned_document_type(),
        };

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![(created_contract, None)],
                operations: vec![
                    Operation {
                        op_type: OperationType::Document(document_insertion_op),
                        frequency: Frequency {
                            times_per_block_range: 1..40,
                            chance_per_block: None,
                        },
                    },
                    Operation {
                        op_type: OperationType::Document(document_replace_op),
                        frequency: Frequency {
                            times_per_block_range: 1..5,
                            chance_per_block: None,
                        },
                    },
                ],
                start_identities: StartIdentities::default(),
                start_addresses: StartAddresses::default(),
                identity_inserts: IdentityInsertInfo {
                    frequency: Frequency {
                        times_per_block_range: 1..6,
                        chance_per_block: None,
                    },
                    start_keys: 5,
                    extra_keys: Default::default(),
                    start_balance_range: dash_to_duffs!(1)..=dash_to_duffs!(1),
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

                epoch_time_length_s: 1576800,
                ..Default::default()
            },
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };
        let block_count = 21;
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

        let issues = outcome
            .abci_app
            .platform
            .drive
            .grove
            .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
            .expect("expected to have no issues");

        assert_eq!(
            issues.len(),
            0,
            "issues are {}",
            issues
                .iter()
                .map(|(hash, (a, b, c))| format!("{}: {} {} {}", hash, a, b, c))
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }

    #[test]
    #[stack_size(4 * 1024 * 1024)]
    fn run_chain_insert_many_document_updates_with_epoch_change() {
        let platform_version = PlatformVersion::latest();
        let created_contract = json_document_to_created_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            1,
            true,
            platform_version,
        )
        .expect("expected to get contract from a json document");

        let contract = created_contract.data_contract();

        let document_insertion_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentAction::DocumentActionInsertRandom(
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
            ),
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .to_owned_document_type(),
        };

        let document_replace_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentActionReplaceRandom,
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .to_owned_document_type(),
        };

        let mut rng = StdRng::seed_from_u64(567);

        let mut simple_signer = SimpleSigner::default();

        let (mut identity1, keys1) = Identity::random_identity_with_main_keys_with_private_key::<
            Vec<_>,
        >(2, &mut rng, platform_version)
        .unwrap();

        simple_signer.add_identity_public_keys(keys1);

        let (mut identity2, keys2) = Identity::random_identity_with_main_keys_with_private_key::<
            Vec<_>,
        >(2, &mut rng, platform_version)
        .unwrap();

        simple_signer.add_identity_public_keys(keys2);

        let start_identities = create_state_transitions_for_identities(
            vec![&mut identity1, &mut identity2],
            &(dash_to_duffs!(1)..=dash_to_duffs!(1)),
            &simple_signer,
            &mut rng,
            platform_version,
        )
        .into_iter()
        .map(|(identity, transition)| (identity, Some(transition)))
        .collect();

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![(created_contract, None)],
                operations: vec![
                    Operation {
                        op_type: OperationType::Document(document_insertion_op),
                        frequency: Frequency {
                            times_per_block_range: 1..2,
                            chance_per_block: None,
                        },
                    },
                    Operation {
                        op_type: OperationType::Document(document_replace_op),
                        frequency: Frequency {
                            times_per_block_range: 1..2,
                            chance_per_block: None,
                        },
                    },
                ],
                start_identities: StartIdentities {
                    hard_coded: start_identities,
                    ..Default::default()
                },
                start_addresses: StartAddresses::default(),
                identity_inserts: Default::default(),

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

                epoch_time_length_s: 1576800,
                ..Default::default()
            },
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };
        let block_count = 21;
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

        let issues = outcome
            .abci_app
            .platform
            .drive
            .grove
            .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
            .expect("expected to have no issues");

        assert_eq!(
            issues.len(),
            0,
            "issues are {}",
            issues
                .iter()
                .map(|(hash, (a, b, c))| format!("{}: {} {} {}", hash, a, b, c))
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }

    #[test]
    #[stack_size(4 * 1024 * 1024)]
    fn run_chain_insert_many_new_identity_per_block_many_document_insertions_updates_and_deletions_with_epoch_change(
    ) {
        let platform_version = PlatformVersion::latest();
        let created_contract = json_document_to_created_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            1,
            true,
            platform_version,
        )
        .expect("expected to get contract from a json document");

        let contract = created_contract.data_contract();

        let document_insertion_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentAction::DocumentActionInsertRandom(
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
            ),
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .to_owned_document_type(),
        };

        let document_replace_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentActionReplaceRandom,
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .to_owned_document_type(),
        };

        let document_deletion_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentAction::DocumentActionDelete,
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .to_owned_document_type(),
        };

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![(created_contract, None)],
                operations: vec![
                    Operation {
                        op_type: OperationType::Document(document_insertion_op),
                        frequency: Frequency {
                            times_per_block_range: 1..40,
                            chance_per_block: None,
                        },
                    },
                    Operation {
                        op_type: OperationType::Document(document_replace_op),
                        frequency: Frequency {
                            times_per_block_range: 1..5,
                            chance_per_block: None,
                        },
                    },
                    Operation {
                        op_type: OperationType::Document(document_deletion_op),
                        frequency: Frequency {
                            times_per_block_range: 1..5,
                            chance_per_block: None,
                        },
                    },
                ],
                start_identities: StartIdentities::default(),
                start_addresses: StartAddresses::default(),
                identity_inserts: IdentityInsertInfo {
                    frequency: Frequency {
                        times_per_block_range: 1..6,
                        chance_per_block: None,
                    },
                    start_keys: 5,
                    extra_keys: Default::default(),
                    start_balance_range: dash_to_duffs!(1)..=dash_to_duffs!(1),
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

                epoch_time_length_s: 1576800,
                ..Default::default()
            },
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };
        let block_count = 100;
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
        assert_eq!(outcome.identities.len() as u64, 296);
        assert_eq!(outcome.masternode_identity_balances.len(), 100);
        let balance_count = outcome
            .masternode_identity_balances
            .into_iter()
            .filter(|(_, balance)| *balance != 0)
            .count();
        assert_eq!(balance_count, 92); // 1 epoch worth of proposers

        let issues = outcome
            .abci_app
            .platform
            .drive
            .grove
            .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
            .expect("expected to have no issues");

        assert_eq!(
            issues.len(),
            0,
            "issues are {}",
            issues
                .iter()
                .map(|(hash, (a, b, c))| format!("{}: {} {} {}", hash, a, b, c))
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }

    #[test]
    #[stack_size(4 * 1024 * 1024)]
    fn run_chain_insert_many_new_identity_per_block_many_document_insertions_updates_transfers_and_deletions_with_epoch_change(
    ) {
        let platform_version = PlatformVersion::latest();
        let created_contract = json_document_to_created_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            1,
            true,
            platform_version,
        )
        .expect("expected to get contract from a json document");

        let contract = created_contract.data_contract();

        let document_insertion_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentAction::DocumentActionInsertRandom(
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
            ),
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .to_owned_document_type(),
        };

        let document_replace_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentActionReplaceRandom,
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .to_owned_document_type(),
        };

        let document_transfer_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentActionTransferRandom,
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .to_owned_document_type(),
        };

        let document_deletion_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentAction::DocumentActionDelete,
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .to_owned_document_type(),
        };

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![(created_contract, None)],
                operations: vec![
                    Operation {
                        op_type: OperationType::Document(document_insertion_op),
                        frequency: Frequency {
                            times_per_block_range: 1..10,
                            chance_per_block: None,
                        },
                    },
                    Operation {
                        op_type: OperationType::Document(document_replace_op),
                        frequency: Frequency {
                            times_per_block_range: 1..5,
                            chance_per_block: None,
                        },
                    },
                    Operation {
                        op_type: OperationType::Document(document_transfer_op),
                        frequency: Frequency {
                            times_per_block_range: 1..5,
                            chance_per_block: None,
                        },
                    },
                    Operation {
                        op_type: OperationType::Document(document_deletion_op),
                        frequency: Frequency {
                            times_per_block_range: 1..5,
                            chance_per_block: None,
                        },
                    },
                ],
                start_identities: StartIdentities::default(),
                start_addresses: StartAddresses::default(),
                identity_inserts: IdentityInsertInfo {
                    frequency: Frequency {
                        times_per_block_range: 1..6,
                        chance_per_block: None,
                    },
                    start_keys: 5,
                    extra_keys: Default::default(),
                    start_balance_range: dash_to_duffs!(1)..=dash_to_duffs!(1),
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

                epoch_time_length_s: 1576800,
                ..Default::default()
            },
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };
        let block_count = 70;
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
        assert_eq!(outcome.identities.len() as u64, 201);
        assert_eq!(outcome.masternode_identity_balances.len(), 100);
        let balance_count = outcome
            .masternode_identity_balances
            .into_iter()
            .filter(|(_, balance)| *balance != 0)
            .count();
        assert_eq!(balance_count, 55); // 1 epoch worth of proposers
    }
}
