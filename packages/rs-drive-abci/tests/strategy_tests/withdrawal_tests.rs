mod tests {
    use crate::asset_unlock_index;
    use crate::execution::{continue_chain_for_strategy, run_chain_for_strategy, GENESIS_TIME_MS};
    use crate::strategy::{
        ChainExecutionOutcome, ChainExecutionParameters, NetworkStrategy, StrategyRandomness,
    };
    use dpp::dashcore::bls_sig_utils::BLSSignature;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::{BlockHash, ChainLock, Txid};
    use dpp::dashcore_rpc::dashcore_rpc_json::{AssetUnlockStatus, AssetUnlockStatusResult};
    use dpp::data_contracts::withdrawals_contract;
    use dpp::identity::{KeyType, Purpose, SecurityLevel};
    use dpp::withdrawal::WithdrawalTransactionIndex;
    use dpp::{dash_to_credits, dash_to_duffs};
    use drive::config::DEFAULT_QUERY_LIMIT;
    use drive::drive::balances::TOTAL_SYSTEM_CREDITS_STORAGE_KEY;
    use drive::drive::identity::withdrawals::paths::{
        get_withdrawal_root_path, WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
    };
    use drive::drive::system::misc_path;
    use drive::util::grove_operations::DirectQueryType;
    use drive_abci::config::{
        ChainLockConfig, ExecutionConfig, InstantLockConfig, PlatformConfig, PlatformTestConfig,
        ValidatorSetConfig,
    };
    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use platform_version::version::mocks::v3_test::TEST_PLATFORM_V3;
    use platform_version::version::PlatformVersion;
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};
    use strategy_tests::frequency::Frequency;
    use strategy_tests::operations::{Operation, OperationType};
    use strategy_tests::{IdentityInsertInfo, StartIdentities, Strategy};

    struct CoreState {
        asset_unlock_statuses: BTreeMap<WithdrawalTransactionIndex, AssetUnlockStatusResult>,
        chain_lock: ChainLock,
    }

    #[test]
    fn run_chain_withdraw_from_identities() {
        // TEST_PLATFORM_V3 is like v4, but without the single quorum can sign withdrawals restriction
        let platform_version = PlatformVersion::get(TEST_PLATFORM_V3.protocol_version)
            .expect("expected to get platform version");
        let start_strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![Operation {
                    op_type: OperationType::IdentityTopUp(dash_to_duffs!(10)..=dash_to_duffs!(10)),
                    frequency: Frequency {
                        times_per_block_range: 1..4,
                        chance_per_block: None,
                    },
                }],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo {
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                    start_keys: 3,
                    extra_keys: [(
                        Purpose::TRANSFER,
                        [(SecurityLevel::CRITICAL, vec![KeyType::ECDSA_SECP256K1])].into(),
                    )]
                    .into(),
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

        let hour_in_ms = 1000 * 60 * 60;
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig::default_100_67(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,

                ..Default::default()
            },
            block_spacing_ms: hour_in_ms,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };

        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .with_initial_protocol_version(TEST_PLATFORM_V3.protocol_version)
            .build_with_mock_rpc();

        platform
            .core_rpc
            .expect_send_raw_transaction()
            .returning(move |_| Ok(Txid::all_zeros()));

        let mut chain_locked_height = 1;

        // Have to go with a complicated shared object for the core state because we need to change
        // rpc response along the way but we can't mutate `platform.core_rpc` later
        // because platform reference is moved into the AbciApplication.
        let shared_core_state = Arc::new(Mutex::new(CoreState {
            asset_unlock_statuses: BTreeMap::new(),
            chain_lock: ChainLock {
                block_height: chain_locked_height,
                block_hash: BlockHash::from_byte_array([1; 32]),
                signature: BLSSignature::from([2; 96]),
            },
        }));

        // Set up Core RPC responses
        {
            let core_state = shared_core_state.clone();

            platform
                .core_rpc
                .expect_get_asset_unlock_statuses()
                .returning(move |indices, _| {
                    Ok(indices
                        .iter()
                        .map(|index| {
                            core_state
                                .lock()
                                .unwrap()
                                .asset_unlock_statuses
                                .get(index)
                                .cloned()
                                .unwrap()
                        })
                        .collect())
                });

            let core_state = shared_core_state.clone();
            platform
                .core_rpc
                .expect_get_best_chain_lock()
                .returning(move || Ok(core_state.lock().unwrap().chain_lock.clone()));
        }

        // Run first two blocks:
        // - Block 1: creates identity
        // - Block 2: tops up identity
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
            identities,
            signer,
            ..
        } = {
            let outcome = run_chain_for_strategy(
                &mut platform,
                2,
                start_strategy,
                config.clone(),
                1,
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

            // Withdrawal transactions are not populated to block execution context yet
            assert_eq!(outcome.withdrawals.len(), 0);

            // Withdrawal documents with pooled status should exist.
            let withdrawal_documents_pooled = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::POOLED.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();
            assert!(withdrawal_documents_pooled.is_empty());

            let locked_amount = outcome
                .abci_app
                .platform
                .drive
                .grove_get_sum_tree_total_value(
                    (&get_withdrawal_root_path()).into(),
                    &WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
                    DirectQueryType::StatefulDirectQuery,
                    None,
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("expected to get locked amount");

            assert_eq!(locked_amount, 0);

            let pooled_withdrawals = withdrawal_documents_pooled.len();

            assert_eq!(pooled_withdrawals, 0);

            outcome
        };

        let continue_strategy_only_withdrawal = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![Operation {
                    op_type: OperationType::IdentityWithdrawal(
                        dash_to_credits!(0.1)..=dash_to_credits!(0.1),
                    ),
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                }],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo::default(),
                identity_contract_nonce_gaps: None,
                signer: Some(signer.clone()),
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

        let continue_strategy_no_operations = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
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
            verify_state_transition_results: true,
            ..Default::default()
        };

        // Run Block 3: initiates a withdrawal
        let (
            ChainExecutionOutcome {
                abci_app,
                proposers,
                validator_quorums: quorums,
                current_validator_quorum_hash: current_quorum_hash,
                current_proposer_versions,
                end_time_ms,
                identity_nonce_counter,
                identity_contract_nonce_counter,
                instant_lock_quorums,
                identities,
                ..
            },
            last_block_pooled_withdrawals_amount,
        ) = {
            let outcome = continue_chain_for_strategy(
                abci_app,
                ChainExecutionParameters {
                    block_start: 3,
                    core_height_start: 1,
                    block_count: 1,
                    proposers,
                    validator_quorums: quorums,
                    current_validator_quorum_hash: current_quorum_hash,
                    current_proposer_versions: Some(current_proposer_versions),
                    current_identity_nonce_counter: identity_nonce_counter,
                    current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                    current_votes: BTreeMap::default(),
                    start_time_ms: GENESIS_TIME_MS,
                    current_time_ms: end_time_ms,
                    instant_lock_quorums,
                    current_identities: identities,
                },
                continue_strategy_only_withdrawal.clone(),
                config.clone(),
                StrategyRandomness::SeedEntropy(2),
            );

            for tx_results_per_block in outcome.state_transition_results_per_block.values() {
                assert_eq!(tx_results_per_block.len(), 1);
                for (state_transition, result) in tx_results_per_block {
                    assert_eq!(
                        result.code, 0,
                        "state transition got code {} : {:?}",
                        result.code, state_transition
                    );
                }
            }

            // Withdrawal transactions are not populated to block execution context yet
            assert_eq!(outcome.withdrawals.len(), 0);

            // Withdrawal documents with pooled status should exist.
            let withdrawal_documents_pooled = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::POOLED.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();
            assert!(!withdrawal_documents_pooled.is_empty());

            let locked_amount = outcome
                .abci_app
                .platform
                .drive
                .grove_get_sum_tree_total_value(
                    (&get_withdrawal_root_path()).into(),
                    &WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
                    DirectQueryType::StatefulDirectQuery,
                    None,
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("expected to get locked amount");

            assert_eq!(locked_amount, dash_to_credits!(0.1) as i64);

            let pooled_withdrawals = withdrawal_documents_pooled.len();

            (outcome, pooled_withdrawals)
        };

        // Run block 4
        // Should broadcast previously pooled withdrawals to core
        let ChainExecutionOutcome {
            abci_app,
            proposers,
            validator_quorums: quorums,
            current_validator_quorum_hash: current_quorum_hash,
            current_proposer_versions,
            end_time_ms,
            withdrawals: last_block_withdrawals,
            identity_nonce_counter,
            identity_contract_nonce_counter,
            instant_lock_quorums,
            identities,
            ..
        } = {
            let outcome = continue_chain_for_strategy(
                abci_app,
                ChainExecutionParameters {
                    block_start: 4,
                    core_height_start: 1,
                    block_count: 1,
                    proposers,
                    validator_quorums: quorums,
                    current_validator_quorum_hash: current_quorum_hash,
                    current_proposer_versions: Some(current_proposer_versions),
                    current_identity_nonce_counter: identity_nonce_counter,
                    current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                    current_votes: BTreeMap::default(),
                    start_time_ms: GENESIS_TIME_MS,
                    current_time_ms: end_time_ms,
                    instant_lock_quorums,
                    current_identities: identities,
                },
                continue_strategy_no_operations.clone(),
                config.clone(),
                StrategyRandomness::SeedEntropy(3),
            );

            // Withdrawal documents with pooled status should exist.
            let withdrawal_documents_broadcasted = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::BROADCASTED.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();

            // In this block all previously pooled withdrawals should be broadcasted
            assert_eq!(
                outcome.withdrawals.len(),
                last_block_pooled_withdrawals_amount
            );
            assert_eq!(
                withdrawal_documents_broadcasted.len(),
                last_block_pooled_withdrawals_amount
            );

            let locked_amount = outcome
                .abci_app
                .platform
                .drive
                .grove_get_sum_tree_total_value(
                    (&get_withdrawal_root_path()).into(),
                    &WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
                    DirectQueryType::StatefulDirectQuery,
                    None,
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("expected to get locked amount");

            assert_eq!(locked_amount, dash_to_credits!(0.1) as i64);

            outcome
        };

        // Update core state before running next block.
        // Asset unlocks broadcasted in the last block should have Unknown status
        {
            let mut core_state = shared_core_state.lock().unwrap();
            last_block_withdrawals.iter().for_each(|tx| {
                let index = asset_unlock_index(tx);

                core_state.asset_unlock_statuses.insert(
                    index,
                    AssetUnlockStatusResult {
                        index,
                        status: AssetUnlockStatus::Unknown,
                    },
                );
            });
        }

        // Run block 5
        // Should change do nothing, because core doesn't report any change
        let ChainExecutionOutcome {
            abci_app,
            proposers,
            validator_quorums: quorums,
            current_validator_quorum_hash: current_quorum_hash,
            current_proposer_versions,
            end_time_ms,
            withdrawals: last_block_withdrawals,
            identity_nonce_counter,
            identity_contract_nonce_counter,
            instant_lock_quorums,
            identities,
            ..
        } = {
            let outcome = continue_chain_for_strategy(
                abci_app,
                ChainExecutionParameters {
                    block_start: 5,
                    core_height_start: 1,
                    block_count: 1,
                    proposers,
                    validator_quorums: quorums,
                    current_validator_quorum_hash: current_quorum_hash,
                    current_proposer_versions: Some(current_proposer_versions),
                    current_identity_nonce_counter: identity_nonce_counter,
                    current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                    current_votes: BTreeMap::default(),
                    start_time_ms: GENESIS_TIME_MS,
                    current_time_ms: end_time_ms + 1000,
                    instant_lock_quorums,
                    current_identities: identities,
                },
                continue_strategy_no_operations.clone(),
                config.clone(),
                StrategyRandomness::SeedEntropy(4),
            );

            let withdrawal_documents_completed = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::COMPLETE.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();

            // things have not changed
            assert!(withdrawal_documents_completed.is_empty());

            let locked_amount = outcome
                .abci_app
                .platform
                .drive
                .grove_get_sum_tree_total_value(
                    (&get_withdrawal_root_path()).into(),
                    &WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
                    DirectQueryType::StatefulDirectQuery,
                    None,
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("expected to get locked amount");

            assert_eq!(locked_amount, dash_to_credits!(0.1) as i64);

            outcome
        };

        // Update core state for newly broadcasted transactions
        {
            let mut core_state = shared_core_state.lock().unwrap();

            // First, set all previously broadcasted transactions to Chainlocked
            core_state
                .asset_unlock_statuses
                .iter_mut()
                .for_each(|(index, status_result)| {
                    // Do not settle yet transactions that were broadcasted in the last block
                    status_result.index = *index;
                    status_result.status = AssetUnlockStatus::Chainlocked;
                });

            // Then increase chainlocked height, so that withdrawals for chainlocked transactions
            // could be completed in the next block
            // TODO: do we need this var?
            chain_locked_height += 1;
            core_state.chain_lock.block_height = chain_locked_height;

            // Then set all newly broadcasted transactions to Unknown
            last_block_withdrawals.iter().for_each(|tx| {
                let index = asset_unlock_index(tx);

                core_state.asset_unlock_statuses.insert(
                    index,
                    AssetUnlockStatusResult {
                        index,
                        status: AssetUnlockStatus::Unknown,
                    },
                );
            });

            drop(core_state);
        }

        // Run block 6
        // Previously broadcasted transactions should be settled after block 5,
        // and their corresponding statuses should be changed to COMPLETED
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
            identities,
            ..
        } = {
            let outcome = continue_chain_for_strategy(
                abci_app,
                ChainExecutionParameters {
                    block_start: 6,
                    core_height_start: 1,
                    block_count: 1,
                    proposers,
                    validator_quorums: quorums,
                    current_validator_quorum_hash: current_quorum_hash,
                    current_proposer_versions: Some(current_proposer_versions),
                    current_identity_nonce_counter: identity_nonce_counter,
                    current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                    current_votes: BTreeMap::default(),
                    start_time_ms: GENESIS_TIME_MS,
                    current_time_ms: end_time_ms + 1000,
                    instant_lock_quorums,
                    current_identities: identities,
                },
                continue_strategy_no_operations.clone(),
                config.clone(),
                StrategyRandomness::SeedEntropy(5),
            );

            let withdrawal_documents_pooled = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::POOLED.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();

            let withdrawal_documents_broadcasted = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::BROADCASTED.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();

            let withdrawal_documents_completed = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::COMPLETE.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();

            // In this block we should have new withdrawals pooled
            assert!(withdrawal_documents_pooled.is_empty());
            assert!(withdrawal_documents_broadcasted.is_empty());

            assert_eq!(withdrawal_documents_completed.len(), 1);

            let locked_amount = outcome
                .abci_app
                .platform
                .drive
                .grove_get_sum_tree_total_value(
                    (&get_withdrawal_root_path()).into(),
                    &WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
                    DirectQueryType::StatefulDirectQuery,
                    None,
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("expected to get locked amount");

            assert_eq!(locked_amount, dash_to_credits!(0.1) as i64);

            outcome
        };

        let outcome = continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start: 7,
                core_height_start: 1,
                block_count: 24,
                proposers,
                validator_quorums: quorums,
                current_validator_quorum_hash: current_quorum_hash,
                current_proposer_versions: Some(current_proposer_versions),
                current_identity_nonce_counter: identity_nonce_counter,
                current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                current_votes: BTreeMap::default(),
                start_time_ms: GENESIS_TIME_MS,
                current_time_ms: end_time_ms + 1000,
                instant_lock_quorums,
                current_identities: identities,
            },
            continue_strategy_no_operations.clone(),
            config.clone(),
            StrategyRandomness::SeedEntropy(6),
        );

        // We should have unlocked the amounts by now
        let locked_amount = outcome
            .abci_app
            .platform
            .drive
            .grove_get_sum_tree_total_value(
                (&get_withdrawal_root_path()).into(),
                &WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
                DirectQueryType::StatefulDirectQuery,
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected to get locked amount");

        assert_eq!(locked_amount, 0);
    }

    #[test]
    fn run_chain_withdrawal_expired() {
        // TEST_PLATFORM_V3 is like v4, but without the single quorum can sign withdrawals restriction
        let platform_version = PlatformVersion::get(TEST_PLATFORM_V3.protocol_version)
            .expect("expected to get platform version");
        let start_strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![Operation {
                    op_type: OperationType::IdentityTopUp(dash_to_duffs!(10)..=dash_to_duffs!(10)),
                    frequency: Frequency {
                        times_per_block_range: 1..4,
                        chance_per_block: None,
                    },
                }],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo {
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                    start_keys: 3,
                    extra_keys: [(
                        Purpose::TRANSFER,
                        [(SecurityLevel::CRITICAL, vec![KeyType::ECDSA_SECP256K1])].into(),
                    )]
                    .into(),
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

        let hour_in_ms = 1000 * 60 * 60;
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig::default_100_67(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,

                ..Default::default()
            },
            block_spacing_ms: hour_in_ms,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };

        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .with_initial_protocol_version(TEST_PLATFORM_V3.protocol_version)
            .build_with_mock_rpc();

        platform
            .core_rpc
            .expect_send_raw_transaction()
            .returning(move |_| Ok(Txid::all_zeros()));

        // Have to go with a complicated shared object for the core state because we need to change
        // rpc response along the way but we can't mutate `platform.core_rpc` later
        // because platform reference is moved into the AbciApplication.
        let shared_core_state = Arc::new(Mutex::new(CoreState {
            asset_unlock_statuses: BTreeMap::new(),
            chain_lock: ChainLock {
                block_height: 1,
                block_hash: BlockHash::from_byte_array([1; 32]),
                signature: BLSSignature::from([2; 96]),
            },
        }));

        // Set up Core RPC responses
        {
            let core_state = shared_core_state.clone();

            platform
                .core_rpc
                .expect_get_asset_unlock_statuses()
                .returning(move |indices, _| {
                    Ok(indices
                        .iter()
                        .map(|index| {
                            core_state
                                .lock()
                                .unwrap()
                                .asset_unlock_statuses
                                .get(index)
                                .cloned()
                                .unwrap()
                        })
                        .collect())
                });

            let core_state = shared_core_state.clone();
            platform
                .core_rpc
                .expect_get_best_chain_lock()
                .returning(move || Ok(core_state.lock().unwrap().chain_lock.clone()));
        }

        // Run first two blocks:
        // - Block 1: creates identity
        // - Block 2: tops up identity
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
            identities,
            signer,
            ..
        } = {
            let outcome = run_chain_for_strategy(
                &mut platform,
                2,
                start_strategy,
                config.clone(),
                1,
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

            // Withdrawal transactions are not populated to block execution context yet
            assert_eq!(outcome.withdrawals.len(), 0);

            // Withdrawal documents with pooled status should exist.
            let withdrawal_documents_pooled = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::POOLED.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();
            assert!(withdrawal_documents_pooled.is_empty());

            let locked_amount = outcome
                .abci_app
                .platform
                .drive
                .grove_get_sum_tree_total_value(
                    (&get_withdrawal_root_path()).into(),
                    &WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
                    DirectQueryType::StatefulDirectQuery,
                    None,
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("expected to get locked amount");

            assert_eq!(locked_amount, 0);

            let pooled_withdrawals = withdrawal_documents_pooled.len();

            assert_eq!(pooled_withdrawals, 0);

            outcome
        };

        let continue_strategy_only_withdrawal = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![Operation {
                    op_type: OperationType::IdentityWithdrawal(
                        dash_to_credits!(0.1)..=dash_to_credits!(0.1),
                    ),
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                }],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo::default(),
                identity_contract_nonce_gaps: None,
                signer: Some(signer.clone()),
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

        let continue_strategy_no_operations = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
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
            verify_state_transition_results: true,
            ..Default::default()
        };

        // Run Block 3: initiates a withdrawal
        let (
            ChainExecutionOutcome {
                abci_app,
                proposers,
                validator_quorums: quorums,
                current_validator_quorum_hash: current_quorum_hash,
                current_proposer_versions,
                end_time_ms,
                identity_nonce_counter,
                identity_contract_nonce_counter,
                instant_lock_quorums,
                identities,
                ..
            },
            last_block_pooled_withdrawals_amount,
        ) = {
            let outcome = continue_chain_for_strategy(
                abci_app,
                ChainExecutionParameters {
                    block_start: 3,
                    core_height_start: 1,
                    block_count: 1,
                    proposers,
                    validator_quorums: quorums,
                    current_validator_quorum_hash: current_quorum_hash,
                    current_proposer_versions: Some(current_proposer_versions),
                    current_identity_nonce_counter: identity_nonce_counter,
                    current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                    current_votes: BTreeMap::default(),
                    start_time_ms: GENESIS_TIME_MS,
                    current_time_ms: end_time_ms,
                    instant_lock_quorums,
                    current_identities: identities,
                },
                continue_strategy_only_withdrawal.clone(),
                config.clone(),
                StrategyRandomness::SeedEntropy(2),
            );

            for tx_results_per_block in outcome.state_transition_results_per_block.values() {
                assert_eq!(tx_results_per_block.len(), 1);
                for (state_transition, result) in tx_results_per_block {
                    assert_eq!(
                        result.code, 0,
                        "state transition got code {} : {:?}",
                        result.code, state_transition
                    );
                }
            }

            // Withdrawal transactions are not populated to block execution context yet
            assert_eq!(outcome.withdrawals.len(), 0);

            // Withdrawal documents with pooled status should exist.
            let withdrawal_documents_pooled = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::POOLED.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();
            assert!(!withdrawal_documents_pooled.is_empty());

            let locked_amount = outcome
                .abci_app
                .platform
                .drive
                .grove_get_sum_tree_total_value(
                    (&get_withdrawal_root_path()).into(),
                    &WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
                    DirectQueryType::StatefulDirectQuery,
                    None,
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("expected to get locked amount");

            assert_eq!(locked_amount, dash_to_credits!(0.1) as i64);

            let pooled_withdrawals = withdrawal_documents_pooled.len();

            (outcome, pooled_withdrawals)
        };

        // Run block 4
        // Should broadcast previously pooled withdrawals to core
        let ChainExecutionOutcome {
            abci_app,
            proposers,
            validator_quorums: quorums,
            current_validator_quorum_hash: current_quorum_hash,
            current_proposer_versions,
            end_time_ms,
            withdrawals: last_block_withdrawals,
            identity_nonce_counter,
            identity_contract_nonce_counter,
            instant_lock_quorums,
            identities,
            ..
        } = {
            let outcome = continue_chain_for_strategy(
                abci_app,
                ChainExecutionParameters {
                    block_start: 4,
                    core_height_start: 1,
                    block_count: 1,
                    proposers,
                    validator_quorums: quorums,
                    current_validator_quorum_hash: current_quorum_hash,
                    current_proposer_versions: Some(current_proposer_versions),
                    current_identity_nonce_counter: identity_nonce_counter,
                    current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                    current_votes: BTreeMap::default(),
                    start_time_ms: GENESIS_TIME_MS,
                    current_time_ms: end_time_ms,
                    instant_lock_quorums,
                    current_identities: identities,
                },
                continue_strategy_no_operations.clone(),
                config.clone(),
                StrategyRandomness::SeedEntropy(2),
            );

            // Withdrawal documents with pooled status should exist.
            let withdrawal_documents_broadcasted = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::BROADCASTED.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();

            // In this block all previously pooled withdrawals should be broadcasted
            assert_eq!(
                outcome.withdrawals.len(),
                last_block_pooled_withdrawals_amount
            );
            assert_eq!(
                withdrawal_documents_broadcasted.len(),
                last_block_pooled_withdrawals_amount
            );

            outcome
        };

        // Update state of the core before proceeding to the next block
        {
            // Simulate transactions being added to the core mempool
            let mut core_state = shared_core_state.lock().unwrap();

            core_state.chain_lock.block_height = 50;

            last_block_withdrawals.iter().for_each(|tx| {
                let index = asset_unlock_index(tx);

                core_state.asset_unlock_statuses.insert(
                    index,
                    AssetUnlockStatusResult {
                        index,
                        status: AssetUnlockStatus::Unknown,
                    },
                );
            });
        }

        // Run block 5
        // Tests withdrawal expiration
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
            identities,
            ..
        } = {
            let outcome = continue_chain_for_strategy(
                abci_app,
                ChainExecutionParameters {
                    block_start: 5,
                    core_height_start: 50,
                    block_count: 1,
                    proposers,
                    validator_quorums: quorums,
                    current_validator_quorum_hash: current_quorum_hash,
                    current_proposer_versions: Some(current_proposer_versions),
                    current_identity_nonce_counter: identity_nonce_counter,
                    current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                    current_votes: BTreeMap::default(),
                    start_time_ms: GENESIS_TIME_MS,
                    current_time_ms: end_time_ms + 1000,
                    instant_lock_quorums,
                    current_identities: identities,
                },
                continue_strategy_no_operations.clone(),
                config.clone(),
                StrategyRandomness::SeedEntropy(5),
            );

            let withdrawal_documents_pooled = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::POOLED.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();

            let withdrawal_documents_broadcasted = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::BROADCASTED.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();

            let withdrawal_documents_completed = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::COMPLETE.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();

            let withdrawal_documents_expired = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::EXPIRED.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();

            assert!(withdrawal_documents_pooled.is_empty());
            assert!(withdrawal_documents_completed.is_empty());

            assert_eq!(withdrawal_documents_expired.len(), 1);

            assert!(withdrawal_documents_broadcasted.is_empty());

            outcome
        };

        // Run block 6
        // Should broadcast previously expired transaction
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
            identities,
            ..
        } = {
            let outcome = continue_chain_for_strategy(
                abci_app,
                ChainExecutionParameters {
                    block_start: 6,
                    core_height_start: 50,
                    block_count: 1,
                    proposers,
                    validator_quorums: quorums,
                    current_validator_quorum_hash: current_quorum_hash,
                    current_proposer_versions: Some(current_proposer_versions),
                    current_identity_nonce_counter: identity_nonce_counter,
                    current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                    current_votes: BTreeMap::default(),
                    start_time_ms: GENESIS_TIME_MS,
                    current_time_ms: end_time_ms,
                    instant_lock_quorums,
                    current_identities: identities,
                },
                continue_strategy_no_operations.clone(),
                config.clone(),
                StrategyRandomness::SeedEntropy(2),
            );

            let withdrawal_documents_pooled = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::POOLED.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();

            let withdrawal_documents_broadcasted = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::BROADCASTED.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();

            let withdrawal_documents_completed = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::COMPLETE.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();

            let withdrawal_documents_expired = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::EXPIRED.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();

            assert!(withdrawal_documents_pooled.is_empty());
            assert!(withdrawal_documents_completed.is_empty());

            assert_eq!(withdrawal_documents_broadcasted.len(), 1);

            assert!(withdrawal_documents_expired.is_empty());

            outcome
        };

        // Update core state saying transaction is chainlocked
        {
            let mut core_state = shared_core_state.lock().unwrap();

            // First, set all previously broadcasted transactions to Chainlocked
            core_state
                .asset_unlock_statuses
                .iter_mut()
                .for_each(|(index, status_result)| {
                    // Do not settle yet transactions that were broadcasted in the last block
                    status_result.index = *index;
                    status_result.status = AssetUnlockStatus::Chainlocked;
                });

            core_state.chain_lock.block_height = 51;

            drop(core_state);
        }

        let outcome = continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start: 7,
                core_height_start: 51,
                block_count: 1,
                proposers,
                validator_quorums: quorums,
                current_validator_quorum_hash: current_quorum_hash,
                current_proposer_versions: Some(current_proposer_versions),
                current_identity_nonce_counter: identity_nonce_counter,
                current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                current_votes: BTreeMap::default(),
                start_time_ms: GENESIS_TIME_MS,
                current_time_ms: end_time_ms,
                instant_lock_quorums,
                current_identities: identities,
            },
            continue_strategy_no_operations.clone(),
            config.clone(),
            StrategyRandomness::SeedEntropy(2),
        );

        let withdrawal_documents_pooled = outcome
            .abci_app
            .platform
            .drive
            .fetch_oldest_withdrawal_documents_by_status(
                withdrawals_contract::WithdrawalStatus::POOLED.into(),
                DEFAULT_QUERY_LIMIT,
                None,
                platform_version,
            )
            .unwrap();

        let withdrawal_documents_broadcasted = outcome
            .abci_app
            .platform
            .drive
            .fetch_oldest_withdrawal_documents_by_status(
                withdrawals_contract::WithdrawalStatus::BROADCASTED.into(),
                DEFAULT_QUERY_LIMIT,
                None,
                platform_version,
            )
            .unwrap();

        let withdrawal_documents_completed = outcome
            .abci_app
            .platform
            .drive
            .fetch_oldest_withdrawal_documents_by_status(
                withdrawals_contract::WithdrawalStatus::COMPLETE.into(),
                DEFAULT_QUERY_LIMIT,
                None,
                platform_version,
            )
            .unwrap();

        let withdrawal_documents_expired = outcome
            .abci_app
            .platform
            .drive
            .fetch_oldest_withdrawal_documents_by_status(
                withdrawals_contract::WithdrawalStatus::EXPIRED.into(),
                DEFAULT_QUERY_LIMIT,
                None,
                platform_version,
            )
            .unwrap();

        assert_eq!(withdrawal_documents_completed.len(), 1);
        assert!(withdrawal_documents_pooled.is_empty());
        assert!(withdrawal_documents_broadcasted.is_empty());
        assert!(withdrawal_documents_expired.is_empty());
    }

    #[test]
    fn run_chain_withdraw_from_identities_too_many_withdrawals_within_a_day_hitting_limit() {
        // TEST_PLATFORM_V3 is like v4, but without the single quorum can sign withdrawals restriction
        let platform_version = PlatformVersion::get(TEST_PLATFORM_V3.protocol_version)
            .expect("expected to get platform version");
        let start_strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![Operation {
                    op_type: OperationType::IdentityTopUp(dash_to_duffs!(10)..=dash_to_duffs!(10)),
                    frequency: Frequency {
                        times_per_block_range: 10..11,
                        chance_per_block: None,
                    },
                }],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo {
                    frequency: Frequency {
                        times_per_block_range: 10..11,
                        chance_per_block: None,
                    },
                    start_keys: 3,
                    extra_keys: [(
                        Purpose::TRANSFER,
                        [(SecurityLevel::CRITICAL, vec![KeyType::ECDSA_SECP256K1])].into(),
                    )]
                    .into(),
                    start_balance_range: dash_to_duffs!(200)..=dash_to_duffs!(200),
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

        let minute_in_ms = 1000 * 60;
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig::default_100_67(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,

                ..Default::default()
            },
            block_spacing_ms: minute_in_ms,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };

        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(TEST_PLATFORM_V3.protocol_version)
            .with_config(config.clone())
            .build_with_mock_rpc();

        platform
            .core_rpc
            .expect_send_raw_transaction()
            .returning(move |_| Ok(Txid::all_zeros()));

        let chain_locked_height = 1;

        // Have to go with a complicated shared object for the core state because we need to change
        // rpc response along the way but we can't mutate `platform.core_rpc` later
        // because platform reference is moved into the AbciApplication.
        let shared_core_state = Arc::new(Mutex::new(CoreState {
            asset_unlock_statuses: BTreeMap::new(),
            chain_lock: ChainLock {
                block_height: chain_locked_height,
                block_hash: BlockHash::from_byte_array([1; 32]),
                signature: BLSSignature::from([2; 96]),
            },
        }));

        // Set up Core RPC responses
        {
            let core_state = shared_core_state.clone();

            platform
                .core_rpc
                .expect_get_asset_unlock_statuses()
                .returning(move |indices, _| {
                    Ok(indices
                        .iter()
                        .map(|index| {
                            core_state
                                .lock()
                                .unwrap()
                                .asset_unlock_statuses
                                .get(index)
                                .cloned()
                                .unwrap()
                        })
                        .collect())
                });

            let core_state = shared_core_state.clone();
            platform
                .core_rpc
                .expect_get_best_chain_lock()
                .returning(move || Ok(core_state.lock().unwrap().chain_lock.clone()));
        }

        // Run first two blocks:
        // - Block 1: creates identity
        // - Block 2: tops up identity
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
            identities,
            signer,
            ..
        } = {
            let outcome = run_chain_for_strategy(
                &mut platform,
                2,
                start_strategy,
                config.clone(),
                1,
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

            // Withdrawal transactions are not populated to block execution context yet
            assert_eq!(outcome.withdrawals.len(), 0);

            // Withdrawal documents with pooled status should exist.
            let withdrawal_documents_pooled = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::POOLED.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();
            assert!(withdrawal_documents_pooled.is_empty());

            let locked_amount = outcome
                .abci_app
                .platform
                .drive
                .grove_get_sum_tree_total_value(
                    (&get_withdrawal_root_path()).into(),
                    &WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
                    DirectQueryType::StatefulDirectQuery,
                    None,
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("expected to get locked amount");

            assert_eq!(locked_amount, 0);

            let pooled_withdrawals = withdrawal_documents_pooled.len();

            assert_eq!(pooled_withdrawals, 0);

            let total_credits_balance = outcome
                .abci_app
                .platform
                .drive
                .calculate_total_credits_balance(None, &platform_version.drive)
                .expect("expected to get total credits balance");

            assert_eq!(
                total_credits_balance.total_identity_balances,
                409997575280380
            ); // Around 4100 Dash.

            assert_eq!(
                total_credits_balance.total_in_trees().unwrap(),
                410000000000000
            ); // Around 4100 Dash.

            let total_credits_in_platform = outcome
                .abci_app
                .platform
                .drive
                .grove_get_raw_value_u64_from_encoded_var_vec(
                    (&misc_path()).into(),
                    TOTAL_SYSTEM_CREDITS_STORAGE_KEY,
                    DirectQueryType::StatefulDirectQuery,
                    None,
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("expected to get total credits in platform");

            assert_eq!(total_credits_in_platform, Some(410000000000000));

            outcome
        };

        let continue_strategy_only_withdrawal = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![Operation {
                    op_type: OperationType::IdentityWithdrawal(
                        dash_to_credits!(25)..=dash_to_credits!(25),
                    ),
                    frequency: Frequency {
                        times_per_block_range: 4..5, // 25 Dash x 4 Withdrawals = 100 Dash
                        chance_per_block: None,
                    },
                }],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo::default(),
                identity_contract_nonce_gaps: None,
                signer: Some(signer.clone()),
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

        let continue_strategy_no_operations = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
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
            verify_state_transition_results: true,
            ..Default::default()
        };

        // Run Block 3 onwards: initiates withdrawals
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
            identities,
            ..
        } = {
            let outcome = continue_chain_for_strategy(
                abci_app,
                ChainExecutionParameters {
                    block_start: 3,
                    core_height_start: 1,
                    block_count: 20,
                    proposers,
                    validator_quorums: quorums,
                    current_validator_quorum_hash: current_quorum_hash,
                    current_proposer_versions: Some(current_proposer_versions),
                    current_identity_nonce_counter: identity_nonce_counter,
                    current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                    current_votes: BTreeMap::default(),
                    start_time_ms: GENESIS_TIME_MS,
                    current_time_ms: end_time_ms,
                    instant_lock_quorums,
                    current_identities: identities,
                },
                continue_strategy_only_withdrawal.clone(),
                config.clone(),
                StrategyRandomness::SeedEntropy(2),
            );

            for tx_results_per_block in outcome.state_transition_results_per_block.values() {
                assert_eq!(tx_results_per_block.len(), 4);
                for (state_transition, result) in tx_results_per_block {
                    assert_eq!(
                        result.code, 0,
                        "state transition got code {} : {:?}",
                        result.code, state_transition
                    );
                }
            }

            // Withdrawal documents with pooled status should not exist.
            let withdrawal_documents_pooled = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::POOLED.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();

            // None are currently pooled since we have no more room
            assert!(withdrawal_documents_pooled.is_empty());

            // Withdrawal documents with queued status should exist.
            let withdrawal_documents_queued = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::QUEUED.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();

            // We have 66 out of 100 queued
            assert_eq!(withdrawal_documents_queued.len(), 66);

            // Withdrawal documents with queued status should exist.
            let withdrawal_documents_completed = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::COMPLETE.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();

            // None have completed because core didn't acknowledge them
            assert_eq!(withdrawal_documents_completed.len(), 0);

            // Withdrawal documents with EXPIRED status should not exist yet.
            let withdrawal_documents_expired = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::EXPIRED.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();

            // We have none expired yet
            assert_eq!(withdrawal_documents_expired.len(), 0);

            // Withdrawal documents with broadcasted status should exist.
            let withdrawal_documents_broadcasted = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::BROADCASTED.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();

            // We have 14 broadcasted (66 queued + 14 broadcasted = 80 total)
            // 14 broadcasted = 14 * 25 Dash = 350 Dash
            // This is explained by
            // withdrawal block 1 : 4 withdrawals taking us to 4000 Dash and 100 Dash Taken
            //   limit is 100, 4 broadcasted
            // withdrawal block 2 : 4 withdrawals taking us to 3900 Dash and 100 Dash Taken
            //   limit is 390 - 100 = 290, 4 broadcasted
            // withdrawal block 3 : 4 withdrawals taking us to 3800 Dash and 100 Dash Taken
            //   limit is 380 - 200 = 180, 4 broadcasted
            // withdrawal block 4 : 4 withdrawals taking us to 3700 Dash and 100 Dash Taken
            //   limit is 370 - 300 = 70, 2 broadcasted
            assert_eq!(withdrawal_documents_broadcasted.len(), 14);

            let locked_amount = outcome
                .abci_app
                .platform
                .drive
                .grove_get_sum_tree_total_value(
                    (&get_withdrawal_root_path()).into(),
                    &WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
                    DirectQueryType::StatefulDirectQuery,
                    None,
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("expected to get locked amount");

            assert_eq!(locked_amount, dash_to_credits!(350) as i64);

            outcome
        };

        let hour_in_ms = 1000 * 60 * 60;

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
            identities,
            ..
        } = {
            let outcome = continue_chain_for_strategy(
                abci_app,
                ChainExecutionParameters {
                    block_start: 23,
                    core_height_start: 1,
                    block_count: 48,
                    proposers,
                    validator_quorums: quorums,
                    current_validator_quorum_hash: current_quorum_hash,
                    current_proposer_versions: Some(current_proposer_versions),
                    current_identity_nonce_counter: identity_nonce_counter,
                    current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                    current_votes: BTreeMap::default(),
                    start_time_ms: GENESIS_TIME_MS,
                    current_time_ms: end_time_ms + 1000,
                    instant_lock_quorums,
                    current_identities: identities,
                },
                continue_strategy_no_operations.clone(),
                PlatformConfig {
                    validator_set: ValidatorSetConfig::default_100_67(),
                    chain_lock: ChainLockConfig::default_100_67(),
                    instant_lock: InstantLockConfig::default_100_67(),
                    execution: ExecutionConfig {
                        verify_sum_trees: true,

                        ..Default::default()
                    },
                    block_spacing_ms: hour_in_ms,
                    testing_configs: PlatformTestConfig::default_minimal_verifications(),
                    ..Default::default()
                },
                StrategyRandomness::SeedEntropy(9),
            );

            // We should have unlocked the amounts by now
            let locked_amount = outcome
                .abci_app
                .platform
                .drive
                .grove_get_sum_tree_total_value(
                    (&get_withdrawal_root_path()).into(),
                    &WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
                    DirectQueryType::StatefulDirectQuery,
                    None,
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("expected to get locked amount");

            // There's about 2000 Dash left in platform, it's normal we are locking 1/10th of it
            assert_eq!(locked_amount, 20000000000000);

            // Withdrawal documents with pooled status should not exist.
            let withdrawal_documents_pooled = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::POOLED.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();

            // None are currently pooled since we have no more room
            assert!(withdrawal_documents_pooled.is_empty());

            // Withdrawal documents with queued status should exist.
            let withdrawal_documents_queued = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::QUEUED.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();

            // We have 58 out of 100 queued
            assert_eq!(withdrawal_documents_queued.len(), 58);

            // Withdrawal documents with queued status should exist.
            let withdrawal_documents_completed = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::COMPLETE.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();

            // None have completed because core didn't acknowledge them
            assert_eq!(withdrawal_documents_completed.len(), 0);

            // Withdrawal documents with EXPIRED status should not exist yet.
            let withdrawal_documents_expired = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::EXPIRED.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();

            // We have none expired yet, because the core height never went up
            assert_eq!(withdrawal_documents_expired.len(), 0);

            // Withdrawal documents with broadcasted status should exist.
            let withdrawal_documents_broadcasted = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::BROADCASTED.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();

            assert_eq!(withdrawal_documents_broadcasted.len(), 22);
            outcome
        };

        let outcome = continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start: 71,
                core_height_start: 1,
                block_count: 250,
                proposers,
                validator_quorums: quorums,
                current_validator_quorum_hash: current_quorum_hash,
                current_proposer_versions: Some(current_proposer_versions),
                current_identity_nonce_counter: identity_nonce_counter,
                current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                current_votes: BTreeMap::default(),
                start_time_ms: GENESIS_TIME_MS,
                current_time_ms: end_time_ms + 1000,
                instant_lock_quorums,
                current_identities: identities,
            },
            continue_strategy_no_operations.clone(),
            PlatformConfig {
                validator_set: ValidatorSetConfig::default_100_67(),
                chain_lock: ChainLockConfig::default_100_67(),
                instant_lock: InstantLockConfig::default_100_67(),
                execution: ExecutionConfig {
                    verify_sum_trees: true,

                    ..Default::default()
                },
                block_spacing_ms: hour_in_ms,
                testing_configs: PlatformTestConfig::default_minimal_verifications(),
                ..Default::default()
            },
            StrategyRandomness::SeedEntropy(9),
        );

        // We should have unlocked the amounts by now
        let locked_amount = outcome
            .abci_app
            .platform
            .drive
            .grove_get_sum_tree_total_value(
                (&get_withdrawal_root_path()).into(),
                &WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
                DirectQueryType::StatefulDirectQuery,
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected to get locked amount");

        // We have nothing locked left
        assert_eq!(locked_amount, 0);

        // Withdrawal documents with pooled status should not exist.
        let withdrawal_documents_pooled = outcome
            .abci_app
            .platform
            .drive
            .fetch_oldest_withdrawal_documents_by_status(
                withdrawals_contract::WithdrawalStatus::POOLED.into(),
                DEFAULT_QUERY_LIMIT,
                None,
                platform_version,
            )
            .unwrap();

        // None are currently pooled since we have no more room
        assert!(withdrawal_documents_pooled.is_empty());

        // Withdrawal documents with queued status should exist.
        let withdrawal_documents_queued = outcome
            .abci_app
            .platform
            .drive
            .fetch_oldest_withdrawal_documents_by_status(
                withdrawals_contract::WithdrawalStatus::QUEUED.into(),
                DEFAULT_QUERY_LIMIT,
                None,
                platform_version,
            )
            .unwrap();

        // Nothing is left in the queue
        assert_eq!(withdrawal_documents_queued.len(), 0);

        // Withdrawal documents with queued status should exist.
        let withdrawal_documents_completed = outcome
            .abci_app
            .platform
            .drive
            .fetch_oldest_withdrawal_documents_by_status(
                withdrawals_contract::WithdrawalStatus::COMPLETE.into(),
                DEFAULT_QUERY_LIMIT,
                None,
                platform_version,
            )
            .unwrap();

        // None have completed because core didn't acknowledge them
        assert_eq!(withdrawal_documents_completed.len(), 0);

        // Withdrawal documents with EXPIRED status should not exist yet.
        let withdrawal_documents_expired = outcome
            .abci_app
            .platform
            .drive
            .fetch_oldest_withdrawal_documents_by_status(
                withdrawals_contract::WithdrawalStatus::EXPIRED.into(),
                DEFAULT_QUERY_LIMIT,
                None,
                platform_version,
            )
            .unwrap();

        // We have none expired yet, because the core height never went up
        assert_eq!(withdrawal_documents_expired.len(), 0);

        // Withdrawal documents with broadcasted status should exist.
        let withdrawal_documents_broadcasted = outcome
            .abci_app
            .platform
            .drive
            .fetch_oldest_withdrawal_documents_by_status(
                withdrawals_contract::WithdrawalStatus::BROADCASTED.into(),
                DEFAULT_QUERY_LIMIT,
                None,
                platform_version,
            )
            .unwrap();

        assert_eq!(withdrawal_documents_broadcasted.len(), 80);
    }

    #[test]
    fn run_chain_withdraw_from_identities_many_small_withdrawals() {
        // TEST_PLATFORM_V3 is like v4, but without the single quorum can sign withdrawals restriction
        let platform_version = PlatformVersion::get(TEST_PLATFORM_V3.protocol_version)
            .expect("expected to get platform version");
        let start_strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![Operation {
                    op_type: OperationType::IdentityTopUp(dash_to_duffs!(10)..=dash_to_duffs!(10)),
                    frequency: Frequency {
                        times_per_block_range: 10..11,
                        chance_per_block: None,
                    },
                }],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo {
                    frequency: Frequency {
                        times_per_block_range: 10..11,
                        chance_per_block: None,
                    },
                    start_keys: 3,
                    extra_keys: [(
                        Purpose::TRANSFER,
                        [(SecurityLevel::CRITICAL, vec![KeyType::ECDSA_SECP256K1])].into(),
                    )]
                    .into(),
                    start_balance_range: dash_to_duffs!(200)..=dash_to_duffs!(200),
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

        let minute_in_ms = 1000 * 60;
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig::default_100_67(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,

                ..Default::default()
            },
            block_spacing_ms: minute_in_ms,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };

        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .with_initial_protocol_version(TEST_PLATFORM_V3.protocol_version)
            .build_with_mock_rpc();

        platform
            .core_rpc
            .expect_send_raw_transaction()
            .returning(move |_| Ok(Txid::all_zeros()));

        let chain_locked_height = 1;

        // Have to go with a complicated shared object for the core state because we need to change
        // rpc response along the way but we can't mutate `platform.core_rpc` later
        // because platform reference is moved into the AbciApplication.
        let shared_core_state = Arc::new(Mutex::new(CoreState {
            asset_unlock_statuses: BTreeMap::new(),
            chain_lock: ChainLock {
                block_height: chain_locked_height,
                block_hash: BlockHash::from_byte_array([1; 32]),
                signature: BLSSignature::from([2; 96]),
            },
        }));

        // Set up Core RPC responses
        {
            let core_state = shared_core_state.clone();

            platform
                .core_rpc
                .expect_get_asset_unlock_statuses()
                .returning(move |indices, _| {
                    Ok(indices
                        .iter()
                        .map(|index| {
                            core_state
                                .lock()
                                .unwrap()
                                .asset_unlock_statuses
                                .get(index)
                                .cloned()
                                .unwrap()
                        })
                        .collect())
                });

            let core_state = shared_core_state.clone();
            platform
                .core_rpc
                .expect_get_best_chain_lock()
                .returning(move || Ok(core_state.lock().unwrap().chain_lock.clone()));
        }

        // Run first two blocks:
        // - Block 1: creates identity
        // - Block 2: tops up identity
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
            identities,
            signer,
            ..
        } = {
            let outcome = run_chain_for_strategy(
                &mut platform,
                2,
                start_strategy,
                config.clone(),
                1,
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

            // Withdrawal transactions are not populated to block execution context yet
            assert_eq!(outcome.withdrawals.len(), 0);

            // Withdrawal documents with pooled status should exist.
            let withdrawal_documents_pooled = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::POOLED.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();
            assert!(withdrawal_documents_pooled.is_empty());

            let locked_amount = outcome
                .abci_app
                .platform
                .drive
                .grove_get_sum_tree_total_value(
                    (&get_withdrawal_root_path()).into(),
                    &WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
                    DirectQueryType::StatefulDirectQuery,
                    None,
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("expected to get locked amount");

            assert_eq!(locked_amount, 0);

            let pooled_withdrawals = withdrawal_documents_pooled.len();

            assert_eq!(pooled_withdrawals, 0);

            let total_credits_balance = outcome
                .abci_app
                .platform
                .drive
                .calculate_total_credits_balance(None, &platform_version.drive)
                .expect("expected to get total credits balance");

            assert_eq!(
                total_credits_balance.total_identity_balances,
                409997575280380
            ); // Around 4100 Dash.

            assert_eq!(
                total_credits_balance.total_in_trees().unwrap(),
                410000000000000
            ); // Around 4100 Dash.

            let total_credits_in_platform = outcome
                .abci_app
                .platform
                .drive
                .grove_get_raw_value_u64_from_encoded_var_vec(
                    (&misc_path()).into(),
                    TOTAL_SYSTEM_CREDITS_STORAGE_KEY,
                    DirectQueryType::StatefulDirectQuery,
                    None,
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("expected to get total credits in platform");

            assert_eq!(total_credits_in_platform, Some(410000000000000));

            outcome
        };

        let continue_strategy_only_withdrawal = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![Operation {
                    op_type: OperationType::IdentityWithdrawal(
                        dash_to_credits!(0.0025)..=dash_to_credits!(0.0025),
                    ),
                    frequency: Frequency {
                        times_per_block_range: 40..41, // 0.0025 Dash x 40 Withdrawals = 0.1 Dash
                        chance_per_block: None,
                    },
                }],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo::default(),
                identity_contract_nonce_gaps: None,
                signer: Some(signer.clone()),
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

        let continue_strategy_no_operations = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
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
            verify_state_transition_results: true,
            ..Default::default()
        };

        // Run Block 3-23 onwards: initiates withdrawals
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
            identities,
            ..
        } = {
            let outcome = continue_chain_for_strategy(
                abci_app,
                ChainExecutionParameters {
                    block_start: 3,
                    core_height_start: 1,
                    block_count: 20,
                    proposers,
                    validator_quorums: quorums,
                    current_validator_quorum_hash: current_quorum_hash,
                    current_proposer_versions: Some(current_proposer_versions),
                    current_identity_nonce_counter: identity_nonce_counter,
                    current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                    current_votes: BTreeMap::default(),
                    start_time_ms: GENESIS_TIME_MS,
                    current_time_ms: end_time_ms,
                    instant_lock_quorums,
                    current_identities: identities,
                },
                continue_strategy_only_withdrawal.clone(),
                config.clone(),
                StrategyRandomness::SeedEntropy(2),
            );

            for tx_results_per_block in outcome.state_transition_results_per_block.values() {
                assert_eq!(tx_results_per_block.len(), 20);
                for (state_transition, result) in tx_results_per_block {
                    assert_eq!(
                        result.code, 0,
                        "state transition got code {} : {:?}",
                        result.code, state_transition
                    );
                }
            }

            // Withdrawal documents with pooled status should not exist.
            let withdrawal_documents_pooled = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::POOLED.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();

            // We should have 4 pooled, which is the limit per block
            assert_eq!(withdrawal_documents_pooled.len(), 4);

            // Withdrawal documents with queued status should exist.
            let withdrawal_documents_queued = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::QUEUED.into(),
                    10000,
                    None,
                    platform_version,
                )
                .unwrap();

            // We have 320 queued
            assert_eq!(withdrawal_documents_queued.len(), 320);

            // Withdrawal documents with queued status should exist.
            let withdrawal_documents_completed = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::COMPLETE.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();

            // None have completed because core didn't acknowledge them
            assert_eq!(withdrawal_documents_completed.len(), 0);

            // Withdrawal documents with EXPIRED status should not exist yet.
            let withdrawal_documents_expired = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::EXPIRED.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();

            // We have none expired yet
            assert_eq!(withdrawal_documents_expired.len(), 0);

            // Withdrawal documents with broadcasted status should exist.
            let withdrawal_documents_broadcasted = outcome
                .abci_app
                .platform
                .drive
                .fetch_oldest_withdrawal_documents_by_status(
                    withdrawals_contract::WithdrawalStatus::BROADCASTED.into(),
                    DEFAULT_QUERY_LIMIT,
                    None,
                    platform_version,
                )
                .unwrap();

            // We have 76 broadcasted (4 pooled + 76 broadcasted = 80 total for 4 per block in 20 blocks)
            assert_eq!(withdrawal_documents_broadcasted.len(), 76);

            let locked_amount = outcome
                .abci_app
                .platform
                .drive
                .grove_get_sum_tree_total_value(
                    (&get_withdrawal_root_path()).into(),
                    &WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
                    DirectQueryType::StatefulDirectQuery,
                    None,
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("expected to get locked amount");

            // 0.2 is 0.0025 amount * 80 withdrawals
            assert_eq!(locked_amount, dash_to_credits!(0.2) as i64);

            outcome
        };

        let hour_in_ms = 1000 * 60 * 60;

        let outcome = continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start: 23,
                core_height_start: 1,
                block_count: 88,
                proposers,
                validator_quorums: quorums,
                current_validator_quorum_hash: current_quorum_hash,
                current_proposer_versions: Some(current_proposer_versions),
                current_identity_nonce_counter: identity_nonce_counter,
                current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                current_votes: BTreeMap::default(),
                start_time_ms: GENESIS_TIME_MS,
                current_time_ms: end_time_ms + 1000,
                instant_lock_quorums,
                current_identities: identities,
            },
            continue_strategy_no_operations.clone(),
            PlatformConfig {
                validator_set: ValidatorSetConfig::default_100_67(),
                chain_lock: ChainLockConfig::default_100_67(),
                instant_lock: InstantLockConfig::default_100_67(),
                execution: ExecutionConfig {
                    verify_sum_trees: true,

                    ..Default::default()
                },
                block_spacing_ms: hour_in_ms,
                testing_configs: PlatformTestConfig::default_minimal_verifications(),
                ..Default::default()
            },
            StrategyRandomness::SeedEntropy(9),
        );

        // We should have unlocked the amounts by now
        let locked_amount = outcome
            .abci_app
            .platform
            .drive
            .grove_get_sum_tree_total_value(
                (&get_withdrawal_root_path()).into(),
                &WITHDRAWAL_TRANSACTIONS_SUM_AMOUNT_TREE_KEY,
                DirectQueryType::StatefulDirectQuery,
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected to get locked amount");

        assert_eq!(locked_amount, 18000000000);

        // Withdrawal documents with pooled status should not exist.
        let withdrawal_documents_pooled = outcome
            .abci_app
            .platform
            .drive
            .fetch_oldest_withdrawal_documents_by_status(
                withdrawals_contract::WithdrawalStatus::POOLED.into(),
                DEFAULT_QUERY_LIMIT,
                None,
                platform_version,
            )
            .unwrap();

        // None are currently pooled since we finished the queue
        assert!(withdrawal_documents_pooled.is_empty());

        // Withdrawal documents with queued status should exist.
        let withdrawal_documents_queued = outcome
            .abci_app
            .platform
            .drive
            .fetch_oldest_withdrawal_documents_by_status(
                withdrawals_contract::WithdrawalStatus::QUEUED.into(),
                DEFAULT_QUERY_LIMIT,
                None,
                platform_version,
            )
            .unwrap();

        // Queue has been finished
        assert_eq!(withdrawal_documents_queued.len(), 0);

        // Withdrawal documents with queued status should exist.
        let withdrawal_documents_completed = outcome
            .abci_app
            .platform
            .drive
            .fetch_oldest_withdrawal_documents_by_status(
                withdrawals_contract::WithdrawalStatus::COMPLETE.into(),
                DEFAULT_QUERY_LIMIT,
                None,
                platform_version,
            )
            .unwrap();

        // None have completed because core didn't acknowledge them
        assert_eq!(withdrawal_documents_completed.len(), 0);

        // Withdrawal documents with EXPIRED status should not exist yet.
        let withdrawal_documents_expired = outcome
            .abci_app
            .platform
            .drive
            .fetch_oldest_withdrawal_documents_by_status(
                withdrawals_contract::WithdrawalStatus::EXPIRED.into(),
                DEFAULT_QUERY_LIMIT,
                None,
                platform_version,
            )
            .unwrap();

        // We have none expired yet, because the core height never went up
        assert_eq!(withdrawal_documents_expired.len(), 0);

        // Withdrawal documents with broadcasted status should exist.
        let withdrawal_documents_broadcasted = outcome
            .abci_app
            .platform
            .drive
            .fetch_oldest_withdrawal_documents_by_status(
                withdrawals_contract::WithdrawalStatus::BROADCASTED.into(),
                1000,
                None,
                platform_version,
            )
            .unwrap();

        assert_eq!(withdrawal_documents_broadcasted.len(), 400);
    }
}
