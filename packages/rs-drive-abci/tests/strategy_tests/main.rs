//! Execution Tests
//!

extern crate core;

use dpp::bls_signatures::SecretKey as BlsPrivateKey;

use drive_abci::test::helpers::setup::TestPlatformBuilder;
use drive_abci::{config::PlatformConfig, test::helpers::setup::TempPlatform};
use strategy_tests::frequency::Frequency;

use dpp::dashcore::transaction::special_transaction::TransactionPayload::AssetUnlockPayloadType;
use dpp::dashcore::Transaction;
use std::collections::BTreeMap;

use strategy::{
    ChainExecutionOutcome, ChainExecutionParameters, NetworkStrategy, StrategyRandomness,
};
use strategy_tests::Strategy;

mod chain_lock_update;
mod core_update_tests;
mod execution;
mod failures;
mod masternode_list_item_helpers;
mod masternodes;
mod patch_platform_tests;
mod query;
mod strategy;
mod token_tests;
mod upgrade_fork_tests;
mod verify_state_transitions;
mod voting_tests;
mod withdrawal_tests;

pub type BlockHeight = u64;

fn asset_unlock_index(tx: &Transaction) -> u64 {
    let Some(AssetUnlockPayloadType(ref payload)) = tx.special_transaction_payload else {
        panic!("expected to get AssetUnlockPayloadType");
    };
    payload.base.index
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::{continue_chain_for_strategy, run_chain_for_strategy};
    use crate::query::QueryStrategy;
    use crate::strategy::{FailureStrategy, MasternodeListChangesStrategy};
    use dashcore_rpc::json::QuorumType;
    use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;
    use dpp::dashcore::hashes::Hash;
    use dpp::dashcore::BlockHash;
    use strategy_tests::operations::DocumentAction::{
        DocumentActionReplaceRandom, DocumentActionTransferRandom,
    };
    use strategy_tests::operations::{
        DocumentAction, DocumentOp, IdentityUpdateOp, Operation, OperationType,
    };
    use strategy_tests::{IdentityInsertInfo, StartIdentities};

    use crate::strategy::CoreHeightIncrease::RandomCoreHeightIncrease;
    use dpp::dashcore::ChainLock;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::document_type::random_document::{
        DocumentFieldFillSize, DocumentFieldFillType,
    };
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::tests::json_document::json_document_to_created_contract;
    use dpp::util::hash::hash_to_hex_string;
    use drive_abci::config::{
        ChainLockConfig, ExecutionConfig, InstantLockConfig, PlatformTestConfig, ValidatorSetConfig,
    };

    use drive_abci::logging::LogLevel;
    use drive_abci::platform_types::platform_state::v0::PlatformStateV0Methods;
    use itertools::Itertools;
    use rand::prelude::StdRng;
    use rand::SeedableRng;
    use tenderdash_abci::proto::abci::{RequestInfo, ResponseInfo};

    use dpp::dash_to_duffs;
    use dpp::data_contract::document_type::v0::random_document_type::{
        FieldMinMaxBounds, FieldTypeWeights, RandomDocumentTypeParameters,
    };
    use dpp::identity::{Identity, KeyType, Purpose, SecurityLevel};
    use dpp::state_transition::StateTransition;
    use platform_version::version::v1::PROTOCOL_VERSION_1;
    use platform_version::version::PlatformVersion;
    use simple_signer::signer::SimpleSigner;
    use strategy_tests::transitions::create_state_transitions_for_identities;
    use tenderdash_abci::Application;

    #[test]
    fn run_chain_nothing_happening() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo::default(),

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
            verify_state_transition_results: false,
            ..Default::default()
        };
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig::default_100_67(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,

                ..ExecutionConfig::default()
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
    fn run_chain_block_signing() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo::default(),

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
            verify_state_transition_results: false,
            ..Default::default()
        };
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig::default_100_67(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,

                ..ExecutionConfig::default()
            },
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        run_chain_for_strategy(
            &mut platform,
            50,
            strategy,
            config,
            13,
            &mut None,
            &mut None,
        );
    }

    #[test]
    fn run_chain_stop_and_restart() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo::default(),

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
            verify_state_transition_results: false,
            ..Default::default()
        };
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig::default_100_67(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,

                ..ExecutionConfig::default()
            },
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default(),
            ..Default::default()
        };
        let TempPlatform {
            mut platform,
            tempdir: _,
        } = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

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
            15,
            strategy.clone(),
            config.clone(),
            40,
            &mut None,
            &mut None,
        );

        let state = abci_app.platform.state.load();

        let protocol_version = state.current_protocol_version_in_consensus();

        let platform_version =
            PlatformVersion::get(protocol_version).expect("expected platform version");

        let known_root_hash = abci_app
            .platform
            .drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("expected root hash");

        abci_app
            .platform
            .reload_state_from_storage(platform_version)
            .expect("expected to recreate state");

        let ResponseInfo {
            data: _,
            version: _,
            app_version: _,
            last_block_height,
            last_block_app_hash,
        } = abci_app
            .info(RequestInfo {
                version: tenderdash_abci::proto::meta::TENDERDASH_VERSION.to_string(),
                block_version: 0,
                p2p_version: 0,
                abci_version: tenderdash_abci::proto::meta::ABCI_VERSION.to_string(),
            })
            .expect("expected to call info");

        assert_eq!(last_block_height, 15);
        assert_eq!(last_block_app_hash, known_root_hash);

        let state = abci_app.platform.state.load();

        let block_start = state
            .last_committed_block_info()
            .as_ref()
            .unwrap()
            .basic_info()
            .height
            + 1;

        continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start,
                core_height_start: 1,
                block_count: 30,
                proposers,
                validator_quorums: quorums,
                current_validator_quorum_hash: current_quorum_hash,
                current_proposer_versions: Some(current_proposer_versions),
                current_identity_nonce_counter: identity_nonce_counter,
                current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                current_votes: BTreeMap::default(),
                start_time_ms: 1681094380000,
                current_time_ms: end_time_ms,
                instant_lock_quorums,
                current_identities: Vec::new(),
            },
            strategy,
            config,
            StrategyRandomness::SeedEntropy(7),
        );
    }

    #[test]
    fn run_chain_stop_and_restart_multiround() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo::default(),

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
            failure_testing: Some(FailureStrategy {
                deterministic_start_seed: None,
                dont_finalize_block: false,
                expect_every_block_errors_with_codes: vec![],
                expect_specific_block_errors_with_codes: Default::default(),
                rounds_before_successful_block: Some(5),
            }),
            query_testing: None,
            verify_state_transition_results: false,
            ..Default::default()
        };
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig::default_100_67(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,

                ..ExecutionConfig::default()
            },
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default(),
            ..Default::default()
        };
        let TempPlatform {
            mut platform,
            tempdir: _,
        } = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

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
            15,
            strategy.clone(),
            config.clone(),
            40,
            &mut None,
            &mut None,
        );

        let state = abci_app.platform.state.load();

        let protocol_version = state.current_protocol_version_in_consensus();

        let platform_version =
            PlatformVersion::get(protocol_version).expect("expected platform version");

        let known_root_hash = abci_app
            .platform
            .drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("expected root hash");

        abci_app
            .platform
            .reload_state_from_storage(platform_version)
            .expect("expected to recreate state");

        let ResponseInfo {
            data: _,
            version: _,
            app_version: _,
            last_block_height,
            last_block_app_hash,
        } = abci_app
            .info(RequestInfo {
                version: tenderdash_abci::proto::meta::TENDERDASH_VERSION.to_string(),
                block_version: 0,
                p2p_version: 0,
                abci_version: tenderdash_abci::proto::meta::ABCI_VERSION.to_string(),
            })
            .expect("expected to call info");

        assert_eq!(last_block_height, 15);
        assert_eq!(last_block_app_hash, known_root_hash);

        let state = abci_app.platform.state.load();

        let block_start = state
            .last_committed_block_info()
            .as_ref()
            .unwrap()
            .basic_info()
            .height
            + 1;

        continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start,
                core_height_start: 1,
                block_count: 30,
                proposers,
                validator_quorums: quorums,
                current_validator_quorum_hash: current_quorum_hash,
                current_proposer_versions: Some(current_proposer_versions),
                current_identity_nonce_counter: identity_nonce_counter,
                current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                current_votes: BTreeMap::default(),
                start_time_ms: 1681094380000,
                current_time_ms: end_time_ms,
                instant_lock_quorums,
                current_identities: Vec::new(),
            },
            strategy,
            config,
            StrategyRandomness::SeedEntropy(7),
        );
    }

    #[test]
    fn run_chain_one_identity_in_solitude_first_protocol_version() {
        let platform_version = PlatformVersion::first();
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
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
    fn run_chain_core_height_randomly_increasing() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo::default(),

                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: None,
            core_height_increase: RandomCoreHeightIncrease(Frequency {
                times_per_block_range: 1..3,
                chance_per_block: Some(0.01),
            }),
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
            1000,
            strategy,
            config,
            15,
            &mut None,
            &mut None,
        );
    }

    #[test]
    fn run_chain_core_height_randomly_increasing_with_epoch_change() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo::default(),

                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: None,
            core_height_increase: RandomCoreHeightIncrease(Frequency {
                times_per_block_range: 1..3,
                chance_per_block: Some(0.5),
            }),
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
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(
            &mut platform,
            1000,
            strategy,
            config,
            15,
            &mut None,
            &mut None,
        );
        assert_eq!(outcome.masternode_identity_balances.len(), 100);
        let all_have_balances = outcome
            .masternode_identity_balances
            .iter()
            .all(|(_, balance)| *balance != 0);
        assert!(all_have_balances, "all masternodes should have a balance");
    }

    #[test]
    fn run_chain_core_height_randomly_increasing_with_quick_epoch_change() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo::default(),

                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: None,
            core_height_increase: RandomCoreHeightIncrease(Frequency {
                times_per_block_range: 1..3,
                chance_per_block: Some(0.5),
            }),
            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: true,
            ..Default::default()
        };
        let hour_in_s = 60 * 60;
        let three_mins_in_ms = 1000 * 60 * 3;
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig::default_100_67(),
            chain_lock: ChainLockConfig::default_100_67(),
            instant_lock: InstantLockConfig::default_100_67(),
            execution: ExecutionConfig {
                verify_sum_trees: true,

                epoch_time_length_s: hour_in_s,
                ..Default::default()
            },
            block_spacing_ms: three_mins_in_ms,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };

        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(
            &mut platform,
            1000,
            strategy,
            config,
            15,
            &mut None,
            &mut None,
        );
        assert_eq!(outcome.masternode_identity_balances.len(), 100);
        let all_have_balances = outcome
            .masternode_identity_balances
            .iter()
            .all(|(_, balance)| *balance != 0);
        assert!(all_have_balances, "all masternodes should have a balance");
        // 49 makes sense because we have about 20 blocks per epoch, and 1000/20 = 50 (but we didn't go over so we should be at 49)
        assert_eq!(outcome.end_epoch_index, 49);
    }

    #[test]
    fn run_chain_core_height_randomly_increasing_with_quorum_updates() {
        let platform_version = PlatformVersion::latest();
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo::default(),
                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 500,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: None,
            core_height_increase: RandomCoreHeightIncrease(Frequency {
                times_per_block_range: 5..6,
                chance_per_block: Some(0.5),
            }),
            proposer_strategy: Default::default(),
            rotate_quorums: true,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: true,
            ..Default::default()
        };
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 10,
                ..Default::default()
            },
            chain_lock: ChainLockConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 10,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            instant_lock: InstantLockConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 10,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            execution: ExecutionConfig {
                verify_sum_trees: true,

                ..Default::default()
            },
            block_spacing_ms: 300,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let ChainExecutionOutcome { abci_app, .. } = run_chain_for_strategy(
            &mut platform,
            2000,
            strategy,
            config,
            40,
            &mut None,
            &mut None,
        );

        // With these params if we didn't rotate we would have at most 240
        // of the 500 hpmns that could get paid, however we are expecting that most
        // will be able to propose a block (and then get paid later on).

        let platform = abci_app.platform;
        let counter = &platform.drive.cache.protocol_versions_counter.read();
        platform
            .drive
            .fetch_versions_with_counter(None, &platform_version.drive)
            .expect("expected to get versions");

        let state = abci_app.platform.state.load();

        assert_eq!(
            state
                .last_committed_block_info()
                .as_ref()
                .unwrap()
                .basic_info()
                .epoch
                .index,
            0
        );
        assert!(
            counter
                .get(&platform_version.protocol_version)
                .unwrap()
                .unwrap()
                > &240
        );
    }

    #[test]
    fn run_chain_core_height_randomly_increasing_with_quorum_updates_new_proposers() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo::default(),

                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: None,
            core_height_increase: RandomCoreHeightIncrease(Frequency {
                times_per_block_range: 1..2,
                chance_per_block: Some(0.2),
            }),
            proposer_strategy: MasternodeListChangesStrategy {
                new_hpmns: Frequency {
                    times_per_block_range: 1..3,
                    chance_per_block: Some(0.5),
                },
                ..Default::default()
            },
            rotate_quorums: true,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: true,
            ..Default::default()
        };
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 10,
                ..Default::default()
            },
            chain_lock: ChainLockConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 10,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            instant_lock: InstantLockConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 10,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            execution: ExecutionConfig {
                verify_sum_trees: true,

                ..Default::default()
            },
            block_spacing_ms: 300,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let ChainExecutionOutcome { abci_app, .. } = run_chain_for_strategy(
            &mut platform,
            300,
            strategy,
            config,
            43,
            &mut None,
            &mut None,
        );

        // With these params if we add new mns the hpmn masternode list would be 100, but we
        // can expect it to be much higher.

        let platform = abci_app.platform;
        let platform_state = platform.state.load();

        assert!(platform_state.hpmn_masternode_list().len() > 100);
    }

    #[test]
    fn run_chain_core_height_randomly_increasing_with_quorum_updates_changing_proposers() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo::default(),

                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: None,
            core_height_increase: RandomCoreHeightIncrease(Frequency {
                times_per_block_range: 1..2,
                chance_per_block: Some(0.2),
            }),
            proposer_strategy: MasternodeListChangesStrategy {
                new_hpmns: Frequency {
                    times_per_block_range: 1..3,
                    chance_per_block: Some(0.5),
                },
                removed_hpmns: Frequency {
                    times_per_block_range: 1..3,
                    chance_per_block: Some(0.5),
                },
                ..Default::default()
            },
            rotate_quorums: true,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: true,
            ..Default::default()
        };
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 10,
                ..Default::default()
            },
            chain_lock: ChainLockConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 10,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            instant_lock: InstantLockConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 10,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            execution: ExecutionConfig {
                verify_sum_trees: true,

                ..Default::default()
            },
            block_spacing_ms: 300,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let ChainExecutionOutcome { abci_app, .. } = run_chain_for_strategy(
            &mut platform,
            300,
            strategy,
            config,
            43,
            &mut None,
            &mut None,
        );

        // With these params if we add new mns the hpmn masternode list would be randomly different than 100.

        let platform = abci_app.platform;
        let platform_state = platform.state.load();

        assert_ne!(platform_state.hpmn_masternode_list().len(), 100);
    }

    #[test]
    fn run_chain_core_height_randomly_increasing_with_quorum_updates_updating_proposers() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo::default(),

                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: None,
            core_height_increase: RandomCoreHeightIncrease(Frequency {
                times_per_block_range: 1..2,
                chance_per_block: Some(0.2),
            }),
            proposer_strategy: MasternodeListChangesStrategy {
                updated_hpmns: Frequency {
                    times_per_block_range: 1..3,
                    chance_per_block: Some(0.5),
                },
                ..Default::default()
            },
            rotate_quorums: true,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: true,
            ..Default::default()
        };
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 10,
                ..Default::default()
            },
            chain_lock: ChainLockConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 10,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            instant_lock: InstantLockConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 10,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            execution: ExecutionConfig {
                verify_sum_trees: true,

                ..Default::default()
            },
            block_spacing_ms: 300,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let ChainExecutionOutcome {
            abci_app,
            proposers,
            ..
        } = run_chain_for_strategy(
            &mut platform,
            300,
            strategy,
            config,
            43,
            &mut None,
            &mut None,
        );

        // With these params if we add new mns the hpmn masternode list would be randomly different than 100.

        let platform_version = PlatformVersion::latest();
        let platform = abci_app.platform;
        let _platform_state = platform.state.load();

        // We need to find if any masternode has ever had their keys disabled.

        let hpmns = platform
            .drive
            .fetch_full_identities(
                proposers
                    .into_iter()
                    .map(|proposer| proposer.masternode.pro_tx_hash.to_byte_array())
                    .collect::<Vec<_>>()
                    .as_slice(),
                None,
                platform_version,
            )
            .expect("expected to fetch identities");

        let has_disabled_keys = hpmns.values().any(|identity| {
            identity
                .as_ref()
                .map(|identity| {
                    identity
                        .public_keys()
                        .values()
                        .any(|key| key.disabled_at().is_some())
                })
                .unwrap_or_default()
        });
        assert!(has_disabled_keys);
    }

    #[test]
    fn run_chain_insert_one_new_identity_per_block_with_block_signing() {
        drive_abci::logging::init_for_tests(LogLevel::Silent);

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
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
    fn run_chain_insert_many_new_identity_per_block_many_document_insertions_and_deletions_with_epoch_change(
    ) {
        // Define the desired stack size
        let stack_size = 4 * 1024 * 1024; //Let's set the stack size to be higher than the default 2MB

        let builder = std::thread::Builder::new()
            .stack_size(stack_size)
            .name("custom_stack_size_thread".into());

        let handler = builder
            .spawn(|| {
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
                    .visualize_verify_grovedb(
                        None,
                        true,
                        false,
                        &platform_version.drive.grove_version,
                    )
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
            })
            .expect("Failed to create thread with custom stack size");
        // Wait for the thread to finish and assert that it didn't panic.
        handler.join().expect("Thread has panicked");
    }

    #[test]
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

        simple_signer.add_keys(keys1);

        let (mut identity2, keys2) = Identity::random_identity_with_main_keys_with_private_key::<
            Vec<_>,
        >(2, &mut rng, platform_version)
        .unwrap();

        simple_signer.add_keys(keys2);

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
    fn run_chain_insert_many_new_identity_per_block_many_document_insertions_updates_and_deletions_with_epoch_change(
    ) {
        // Define the desired stack size
        let stack_size = 4 * 1024 * 1024; //Let's set the stack size to be higher than the default 2MB

        let builder = std::thread::Builder::new()
            .stack_size(stack_size)
            .name("custom_stack_size_thread".into());

        let handler = builder
            .spawn(|| {
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
                    .visualize_verify_grovedb(
                        None,
                        true,
                        false,
                        &platform_version.drive.grove_version,
                    )
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
            })
            .expect("Failed to create thread with custom stack size");
        // Wait for the thread to finish and assert that it didn't panic.
        handler.join().expect("Thread has panicked");
    }

    #[test]
    fn run_chain_insert_many_new_identity_per_block_many_document_insertions_updates_transfers_and_deletions_with_epoch_change(
    ) {
        // Define the desired stack size
        let stack_size = 4 * 1024 * 1024; //Let's set the stack size to be higher than the default 2MB

        let builder = std::thread::Builder::new()
            .stack_size(stack_size)
            .name("custom_stack_size_thread".into());

        let handler = builder
            .spawn(|| {
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
            })
            .expect("Failed to create thread with custom stack size");
        // Wait for the thread to finish and assert that it didn't panic.
        handler.join().expect("Thread has panicked");
    }

    #[test]
    fn run_chain_top_up_identities() {
        let platform_version = PlatformVersion::latest();
        drive_abci::logging::init_for_tests(LogLevel::Silent);

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![Operation {
                    op_type: OperationType::IdentityTopUp(dash_to_duffs!(1)..=dash_to_duffs!(1)),
                    frequency: Frequency {
                        times_per_block_range: 1..3,
                        chance_per_block: None,
                    },
                }],
                start_identities: StartIdentities::default(),
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
            sign_instant_locks: true,
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

        let max_initial_balance = 100000000000u64; // TODO: some centralized way for random test data (`arbitrary` maybe?)
        let balances = outcome
            .abci_app
            .platform
            .drive
            .fetch_identities_balances(
                &outcome
                    .identities
                    .into_iter()
                    .map(|identity| identity.id().to_buffer())
                    .collect(),
                None,
                platform_version,
            )
            .expect("expected to fetch balances");

        assert!(balances
            .into_iter()
            .any(|(_, balance)| balance > max_initial_balance));
    }

    #[test]
    fn run_chain_update_identities_add_keys() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![Operation {
                    op_type: OperationType::IdentityUpdate(
                        IdentityUpdateOp::IdentityUpdateAddKeys(3),
                    ),
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                }],
                start_identities: StartIdentities::default(),
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
            // because we can add an identity and add keys to it in the same block
            // the result would be different then expected
            verify_state_transition_results: false,
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
        let state = outcome.abci_app.platform.state.load();
        let protocol_version = state.current_protocol_version_in_consensus();
        let platform_version = PlatformVersion::get(protocol_version).unwrap();

        let identities = outcome
            .abci_app
            .platform
            .drive
            .fetch_full_identities(
                outcome
                    .identities
                    .into_iter()
                    .map(|identity| identity.id().to_buffer())
                    .collect::<Vec<_>>()
                    .as_slice(),
                None,
                platform_version,
            )
            .expect("expected to fetch balances");

        assert!(identities
            .into_iter()
            .any(|(_, identity)| { identity.expect("expected identity").public_keys().len() > 7 }));
    }

    #[test]
    fn run_chain_update_identities_remove_keys() {
        let platform_version = PlatformVersion::latest();
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![Operation {
                    op_type: OperationType::IdentityUpdate(
                        IdentityUpdateOp::IdentityUpdateDisableKey(3),
                    ),
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                }],
                start_identities: StartIdentities::default(),
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
            // because we can add an identity and remove keys to it in the same block
            // the result would be different then expected
            verify_state_transition_results: false,
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

        let identities = outcome
            .abci_app
            .platform
            .drive
            .fetch_full_identities(
                outcome
                    .identities
                    .into_iter()
                    .map(|identity| identity.id().to_buffer())
                    .collect::<Vec<_>>()
                    .as_slice(),
                None,
                platform_version,
            )
            .expect("expected to fetch balances");

        assert!(identities.into_iter().any(|(_, identity)| {
            identity
                .expect("expected identity")
                .public_keys()
                .iter()
                .any(|(_, public_key)| public_key.is_disabled())
        }));
    }

    #[test]
    fn run_chain_rotation_is_deterministic_1_block() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo {
                    //we do this to create some paying transactions
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                    start_keys: 5,
                    extra_keys: Default::default(),
                    start_balance_range: dash_to_duffs!(1)..=dash_to_duffs!(1),
                },

                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 50,
            extra_normal_mns: 0,
            validator_quorum_count: 10,
            upgrading_info: None,
            core_height_increase: RandomCoreHeightIncrease(Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            }),
            proposer_strategy: Default::default(),
            rotate_quorums: true,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: false,
            ..Default::default()
        };
        let day_in_ms = 1000 * 60 * 60 * 24;
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 3,
                ..Default::default()
            },
            chain_lock: ChainLockConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 3,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            instant_lock: InstantLockConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 3,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            execution: ExecutionConfig {
                verify_sum_trees: true,
                ..Default::default()
            },
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };

        let mut platforms = Vec::new();
        let mut outcomes = Vec::new();

        for _ in 0..2 {
            let platform = TestPlatformBuilder::new()
                .with_config(config.clone())
                .build_with_mock_rpc();
            platforms.push(platform);
        }

        for platform in &mut platforms {
            platform
                .core_rpc
                .expect_get_best_chain_lock()
                .returning(move || {
                    Ok(ChainLock {
                        block_height: 10,
                        block_hash: BlockHash::from_byte_array([1; 32]),
                        signature: [2; 96].into(),
                    })
                });

            let outcome = run_chain_for_strategy(
                platform,
                1,
                strategy.clone(),
                config.clone(),
                7,
                &mut None,
                &mut None,
            );
            outcomes.push(outcome);
        }

        let first_proposers_fingerprint = hash_to_hex_string(
            outcomes[0]
                .proposers
                .iter()
                .map(|masternode_list_item_with_updates| {
                    hex::encode(masternode_list_item_with_updates.masternode.pro_tx_hash)
                })
                .join("|"),
        );

        assert!(outcomes.iter().all(|outcome| {
            let last_proposers_fingerprint = hash_to_hex_string(
                outcome
                    .proposers
                    .iter()
                    .map(|masternode_list_item_with_updates| {
                        hex::encode(masternode_list_item_with_updates.masternode.pro_tx_hash)
                    })
                    .join("|"),
            );

            first_proposers_fingerprint == last_proposers_fingerprint
        }));

        let first_masternodes_fingerprint = hash_to_hex_string(
            outcomes[0]
                .masternode_identity_balances
                .keys()
                .map(hex::encode)
                .join("|"),
        );

        assert!(outcomes.iter().all(|outcome| {
            let last_masternodes_fingerprint = hash_to_hex_string(
                outcome
                    .masternode_identity_balances
                    .keys()
                    .map(hex::encode)
                    .join("|"),
            );

            first_masternodes_fingerprint == last_masternodes_fingerprint
        }));

        let first_validator_set_fingerprint = hash_to_hex_string(
            outcomes[0]
                .current_quorum()
                .validator_set
                .iter()
                .map(|validator| hex::encode(validator.pro_tx_hash))
                .join("|"),
        );

        assert!(outcomes.iter().all(|outcome| {
            let last_validator_set_fingerprint = hash_to_hex_string(
                outcome
                    .current_quorum()
                    .validator_set
                    .iter()
                    .map(|validator| hex::encode(validator.pro_tx_hash))
                    .join("|"),
            );

            first_validator_set_fingerprint == last_validator_set_fingerprint
        }));

        let state = outcomes[0].abci_app.platform.state.load();
        let protocol_version = state.current_protocol_version_in_consensus();
        let platform_version =
            PlatformVersion::get(protocol_version).expect("expected platform version");

        let first_last_app_hash = outcomes[0]
            .abci_app
            .platform
            .drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("should return app hash");

        assert!(outcomes.iter().all(|outcome| {
            let last_app_hash = outcome
                .abci_app
                .platform
                .drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .expect("should return app hash");

            last_app_hash == first_last_app_hash
        }));
    }

    #[test]
    fn run_chain_heavy_rotation_deterministic_before_payout() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo {
                    //we do this to create some paying transactions
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                    start_keys: 5,
                    extra_keys: Default::default(),
                    start_balance_range: dash_to_duffs!(1)..=dash_to_duffs!(1),
                },

                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 500,
            extra_normal_mns: 0,
            validator_quorum_count: 100,
            upgrading_info: None,
            core_height_increase: RandomCoreHeightIncrease(Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            }),
            proposer_strategy: Default::default(),
            rotate_quorums: true,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: false,
            ..Default::default()
        };
        let day_in_ms = 1000 * 60 * 60 * 24;
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 3,
                ..Default::default()
            },
            chain_lock: ChainLockConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 3,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            instant_lock: InstantLockConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 3,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            execution: ExecutionConfig {
                verify_sum_trees: true,
                epoch_time_length_s: 1576800,
                ..Default::default()
            },
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };
        let mut platform_a = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();
        let mut platform_b = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();
        platform_a
            .core_rpc
            .expect_get_best_chain_lock()
            .returning(move || {
                Ok(ChainLock {
                    block_height: 10,
                    block_hash: BlockHash::from_byte_array([1; 32]),
                    signature: [2; 96].into(),
                })
            });
        platform_b
            .core_rpc
            .expect_get_best_chain_lock()
            .returning(move || {
                Ok(ChainLock {
                    block_height: 10,
                    block_hash: BlockHash::from_byte_array([1; 32]),
                    signature: [2; 96].into(),
                })
            });

        let outcome_a = run_chain_for_strategy(
            &mut platform_a,
            18,
            strategy.clone(),
            config.clone(),
            7,
            &mut None,
            &mut None,
        );
        let outcome_b = run_chain_for_strategy(
            &mut platform_b,
            18,
            strategy,
            config,
            7,
            &mut None,
            &mut None,
        );
        assert_eq!(outcome_a.end_epoch_index, outcome_b.end_epoch_index); // 100/18
        assert_eq!(outcome_a.masternode_identity_balances.len(), 500); // 500 nodes
        assert_eq!(outcome_b.masternode_identity_balances.len(), 500); // 500 nodes
        assert_eq!(outcome_a.end_epoch_index, 0); // 100/18
        let masternodes_fingerprint_a = hash_to_hex_string(
            outcome_a
                .masternode_identity_balances
                .keys()
                .map(hex::encode)
                .join("|"),
        );
        assert_eq!(
            masternodes_fingerprint_a,
            "0154fd29f0062819ee6b8063ea02c9f3296ed9af33a4538ae98087edb1a75029".to_string()
        );
        let masternodes_fingerprint_b = hash_to_hex_string(
            outcome_b
                .masternode_identity_balances
                .keys()
                .map(hex::encode)
                .join("|"),
        );
        assert_eq!(
            masternodes_fingerprint_b,
            "0154fd29f0062819ee6b8063ea02c9f3296ed9af33a4538ae98087edb1a75029".to_string()
        );

        let state = outcome_a.abci_app.platform.state.load();
        let protocol_version = state.current_protocol_version_in_consensus();
        let platform_version =
            PlatformVersion::get(protocol_version).expect("expected platform version");

        let last_app_hash_a = outcome_a
            .abci_app
            .platform
            .drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("should return app hash");

        let last_app_hash_b = outcome_b
            .abci_app
            .platform
            .drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("should return app hash");

        assert_eq!(last_app_hash_a, last_app_hash_b);

        let balance_count = outcome_a
            .masternode_identity_balances
            .into_iter()
            .filter(|(_, balance)| *balance != 0)
            .count();
        assert_eq!(balance_count, 0);
    }

    #[test]
    fn run_chain_proposer_proposes_a_chainlock_that_would_remove_themselves_from_the_list_deterministic(
    ) {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo {
                    //we do this to create some paying transactions
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                    start_keys: 5,
                    extra_keys: Default::default(),
                    start_balance_range: dash_to_duffs!(1)..=dash_to_duffs!(1),
                },

                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 500,
            extra_normal_mns: 0,
            validator_quorum_count: 100,
            upgrading_info: None,
            core_height_increase: RandomCoreHeightIncrease(Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            }),
            proposer_strategy: Default::default(),
            rotate_quorums: true,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: false,
            ..Default::default()
        };
        let day_in_ms = 1000 * 60 * 60 * 24;
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 3,
                ..Default::default()
            },
            chain_lock: ChainLockConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 3,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            instant_lock: InstantLockConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 3,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            execution: ExecutionConfig {
                verify_sum_trees: true,
                ..Default::default()
            },
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };
        let mut platform_a = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();
        let mut platform_b = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();
        platform_a
            .core_rpc
            .expect_get_best_chain_lock()
            .returning(move || {
                Ok(ChainLock {
                    block_height: 10,
                    block_hash: BlockHash::from_byte_array([1; 32]),
                    signature: [2; 96].into(),
                })
            });
        platform_b
            .core_rpc
            .expect_get_best_chain_lock()
            .returning(move || {
                Ok(ChainLock {
                    block_height: 10,
                    block_hash: BlockHash::from_byte_array([1; 32]),
                    signature: [2; 96].into(),
                })
            });

        let outcome_a = run_chain_for_strategy(
            &mut platform_a,
            100,
            strategy.clone(),
            config.clone(),
            7,
            &mut None,
            &mut None,
        );
        let outcome_b = run_chain_for_strategy(
            &mut platform_b,
            100,
            strategy,
            config,
            7,
            &mut None,
            &mut None,
        );
        assert_eq!(outcome_a.end_epoch_index, outcome_b.end_epoch_index); // 100/18
        assert_eq!(outcome_a.masternode_identity_balances.len(), 500); // 500 nodes
        assert_eq!(outcome_b.masternode_identity_balances.len(), 500); // 500 nodes
                                                                       //assert_eq!(outcome_a.end_epoch_index, 1); // 100/18
        let masternodes_fingerprint_a = hash_to_hex_string(
            outcome_a
                .masternode_identity_balances
                .keys()
                .map(hex::encode)
                .join("|"),
        );
        assert_eq!(
            masternodes_fingerprint_a,
            "0154fd29f0062819ee6b8063ea02c9f3296ed9af33a4538ae98087edb1a75029".to_string()
        );
        let masternodes_fingerprint_b = hash_to_hex_string(
            outcome_b
                .masternode_identity_balances
                .keys()
                .map(hex::encode)
                .join("|"),
        );
        assert_eq!(
            masternodes_fingerprint_b,
            "0154fd29f0062819ee6b8063ea02c9f3296ed9af33a4538ae98087edb1a75029".to_string()
        );

        let state = outcome_a.abci_app.platform.state.load();
        let protocol_version = state.current_protocol_version_in_consensus();
        let platform_version =
            PlatformVersion::get(protocol_version).expect("expected platform version");

        let last_app_hash_a = outcome_a
            .abci_app
            .platform
            .drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("should return app hash");

        let last_app_hash_b = outcome_b
            .abci_app
            .platform
            .drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("should return app hash");

        assert_eq!(last_app_hash_a, last_app_hash_b);

        let balance_count = outcome_a
            .masternode_identity_balances
            .into_iter()
            .filter(|(_, balance)| *balance != 0)
            .count();
        // we have a maximum 90 quorums, that could have been used, 7 were used twice
        assert_eq!(balance_count, 83);
    }

    #[test]
    fn run_chain_stop_and_restart_with_rotation() {
        drive_abci::logging::init_for_tests(LogLevel::Silent);

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo::default(),

                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 500,
            extra_normal_mns: 0,
            validator_quorum_count: 100,
            upgrading_info: None,

            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: false,
            ..Default::default()
        };
        let day_in_ms = 1000 * 60 * 60 * 24;
        let config = PlatformConfig {
            validator_set: ValidatorSetConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 3,
                ..Default::default()
            },
            chain_lock: ChainLockConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 3,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            instant_lock: InstantLockConfig {
                quorum_type: QuorumType::Llmq100_67,
                quorum_size: 3,
                quorum_window: 24,
                quorum_active_signers: 24,
                quorum_rotation: false,
            },
            execution: ExecutionConfig {
                verify_sum_trees: true,
                epoch_time_length_s: 1576800,
                ..Default::default()
            },
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_minimal_verifications(),
            ..Default::default()
        };
        let TempPlatform {
            mut platform,
            tempdir: _,
        } = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let ChainExecutionOutcome {
            abci_app,
            proposers,
            validator_quorums: quorums,
            current_validator_quorum_hash: current_quorum_hash,
            current_proposer_versions,
            end_time_ms,
            identity_nonce_counter,
            identity_contract_nonce_counter,
            ..
        } = run_chain_for_strategy(
            &mut platform,
            100,
            strategy.clone(),
            config.clone(),
            89,
            &mut None,
            &mut None,
        );

        let state = abci_app.platform.state.load();
        let protocol_version = state.current_protocol_version_in_consensus();

        let platform_version = PlatformVersion::get(protocol_version).unwrap();

        let known_root_hash = abci_app
            .platform
            .drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("expected root hash");

        abci_app
            .platform
            .reload_state_from_storage(platform_version)
            .expect("expected to recreate state");

        let ResponseInfo {
            data: _,
            version: _,
            app_version: _,
            last_block_height,
            last_block_app_hash,
        } = abci_app
            .info(RequestInfo {
                version: tenderdash_abci::proto::meta::TENDERDASH_VERSION.to_string(),
                block_version: 0,
                p2p_version: 0,
                abci_version: tenderdash_abci::proto::meta::ABCI_VERSION.to_string(),
            })
            .expect("expected to call info");

        assert_eq!(last_block_height, 100);
        assert_eq!(last_block_app_hash, known_root_hash);

        let state = abci_app.platform.state.load();

        let block_start = state
            .last_committed_block_info()
            .as_ref()
            .unwrap()
            .basic_info()
            .height
            + 1;

        continue_chain_for_strategy(
            abci_app,
            ChainExecutionParameters {
                block_start,
                core_height_start: 10,
                block_count: 30,
                proposers,
                validator_quorums: quorums,
                current_validator_quorum_hash: current_quorum_hash,
                instant_lock_quorums: Default::default(),
                current_proposer_versions: Some(current_proposer_versions),
                current_identity_nonce_counter: identity_nonce_counter,
                current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                current_votes: BTreeMap::default(),
                start_time_ms: 1681094380000,
                current_time_ms: end_time_ms,
                current_identities: Vec::new(),
            },
            strategy,
            config,
            StrategyRandomness::SeedEntropy(block_start),
        );
    }

    #[test]
    fn run_chain_transfer_between_identities() {
        let platform_version = PlatformVersion::latest();
        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![],
                operations: vec![Operation {
                    op_type: OperationType::IdentityTransfer(None),
                    frequency: Frequency {
                        times_per_block_range: 1..3,
                        chance_per_block: None,
                    },
                }],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo {
                    //we do this to create some paying transactions
                    frequency: Frequency {
                        times_per_block_range: 6..10,
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
            15,
            strategy,
            config,
            15,
            &mut None,
            &mut None,
        );

        let _balances = &outcome
            .abci_app
            .platform
            .drive
            .fetch_identities_balances(
                &outcome
                    .identities
                    .iter()
                    .map(|identity| identity.id().to_buffer())
                    .collect(),
                None,
                platform_version,
            )
            .expect("expected to fetch balances");

        assert_eq!(outcome.identities.len(), 106);
    }

    // Test should filter out transactions exceeding max tx bytes per block
    #[test]
    fn run_transactions_exceeding_max_block_size() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                identity_inserts: IdentityInsertInfo {
                    frequency: Frequency {
                        times_per_block_range: 5..6,
                        chance_per_block: None,
                    },
                    start_keys: 5,
                    extra_keys: Default::default(),
                    start_balance_range: dash_to_duffs!(1)..=dash_to_duffs!(1),
                },

                ..Default::default()
            },
            max_tx_bytes_per_block: 3500,
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
        let state_transitions = outcome
            .state_transition_results_per_block
            .get(&1)
            .expect("expected state transition results");

        // Only three out of five transitions should've made to the block
        assert_eq!(state_transitions.len(), 3);
    }
}
