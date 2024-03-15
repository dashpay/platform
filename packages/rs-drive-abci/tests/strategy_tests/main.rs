// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! Execution Tests
//!

extern crate core;

use dashcore_rpc::dashcore::QuorumHash;

use dpp::bls_signatures::PrivateKey as BlsPrivateKey;

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
mod query;
mod strategy;
mod upgrade_fork_tests;
mod verify_state_transitions;

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
    use crate::execution::{continue_chain_for_strategy, run_chain_for_strategy, GENESIS_TIME_MS};
    use crate::query::QueryStrategy;
    use crate::strategy::{FailureStrategy, MasternodeListChangesStrategy};
    use dashcore_rpc::dashcore::hashes::Hash;
    use dashcore_rpc::dashcore::BlockHash;
    use dashcore_rpc::dashcore_rpc_json::{AssetUnlockStatus, ExtendedQuorumDetails};
    use dashcore_rpc::json::AssetUnlockStatusResult;
    use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;
    use std::sync::{Arc, Mutex};
    use strategy_tests::operations::DocumentAction::DocumentActionReplace;
    use strategy_tests::operations::{
        DocumentAction, DocumentOp, IdentityUpdateOp, Operation, OperationType,
    };
    use strategy_tests::StartIdentities;

    use crate::strategy::CoreHeightIncrease::RandomCoreHeightIncrease;
    use dpp::dashcore::bls_sig_utils::BLSSignature;
    use dpp::dashcore::ChainLock;
    use dpp::dashcore::Txid;
    use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
    use dpp::data_contract::document_type::random_document::{
        DocumentFieldFillSize, DocumentFieldFillType,
    };
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::system_data_contracts::withdrawals_contract;
    use dpp::tests::json_document::json_document_to_created_contract;
    use dpp::util::hash::hash_to_hex_string;
    use dpp::version::PlatformVersion;
    use drive::drive::config::DEFAULT_QUERY_LIMIT;
    use drive::drive::identity::withdrawals::WithdrawalTransactionIndex;
    use drive_abci::config::{ExecutionConfig, PlatformTestConfig};

    use drive_abci::logging::LogLevel;
    use drive_abci::platform_types::platform_state::v0::PlatformStateV0Methods;
    use drive_abci::rpc::core::QuorumListExtendedInfo;
    use itertools::Itertools;
    use tenderdash_abci::proto::abci::{RequestInfo, ResponseInfo};

    use dpp::state_transition::StateTransition;
    use tenderdash_abci::Application;

    #[allow(dead_code)]
    #[deprecated(note = "This function is marked as unused.")]
    #[allow(deprecated)]
    pub fn generate_quorums_extended_info(n: u32) -> QuorumListExtendedInfo {
        let mut quorums = QuorumListExtendedInfo::new();

        for i in 0..n {
            let i_bytes = [i as u8; 32];

            let hash = QuorumHash::from_byte_array(i_bytes);

            let details = ExtendedQuorumDetails {
                creation_height: i,
                health_ratio: (i as f32) / (n as f32),
                mined_block_hash: BlockHash::from_slice(&i_bytes).unwrap(),
                num_valid_members: i,
                quorum_index: Some(i),
            };

            if let Some(v) = quorums.insert(hash, details) {
                panic!("duplicate record {:?}={:?}", hash, v)
            }
        }
        quorums
    }

    #[test]
    fn run_chain_nothing_happening() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                contracts_with_updates: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identities_inserts: Frequency {
                    times_per_block_range: Default::default(),
                    chance_per_block: None,
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
            verify_state_transition_results: false,
            ..Default::default()
        };
        let config = PlatformConfig {
            validator_set_quorum_size: 100,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 25,
                ..ExecutionConfig::default()
            },
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        run_chain_for_strategy(&mut platform, 100, strategy, config, 15);
    }

    #[test]
    fn run_chain_block_signing() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                contracts_with_updates: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identities_inserts: Frequency {
                    times_per_block_range: Default::default(),
                    chance_per_block: None,
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
            verify_state_transition_results: false,
            ..Default::default()
        };
        let config = PlatformConfig {
            validator_set_quorum_size: 100,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 25,
                ..ExecutionConfig::default()
            },
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        run_chain_for_strategy(&mut platform, 50, strategy, config, 13);
    }

    #[test]
    fn run_chain_stop_and_restart() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                contracts_with_updates: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identities_inserts: Frequency {
                    times_per_block_range: Default::default(),
                    chance_per_block: None,
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
            verify_state_transition_results: false,
            ..Default::default()
        };
        let config = PlatformConfig {
            validator_set_quorum_size: 100,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 25,
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
            quorums,
            current_quorum_hash,
            current_proposer_versions,
            end_time_ms,
            identity_nonce_counter,
            identity_contract_nonce_counter,
            ..
        } = run_chain_for_strategy(&mut platform, 15, strategy.clone(), config.clone(), 40);

        let known_root_hash = abci_app
            .platform
            .drive
            .grove
            .root_hash(None)
            .unwrap()
            .expect("expected root hash");

        let state = abci_app.platform.state.load();

        let protocol_version = state.current_protocol_version_in_consensus();

        let platform_version =
            PlatformVersion::get(protocol_version).expect("expected platform version");

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
                quorums,
                current_quorum_hash,
                current_proposer_versions: Some(current_proposer_versions),
                current_identity_nonce_counter: identity_nonce_counter,
                current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                start_time_ms: 1681094380000,
                current_time_ms: end_time_ms,
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
                contracts_with_updates: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identities_inserts: Frequency {
                    times_per_block_range: Default::default(),
                    chance_per_block: None,
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
            validator_set_quorum_size: 100,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 25,
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
            quorums,
            current_quorum_hash,
            current_proposer_versions,
            end_time_ms,
            identity_nonce_counter,
            identity_contract_nonce_counter,
            ..
        } = run_chain_for_strategy(&mut platform, 15, strategy.clone(), config.clone(), 40);

        let known_root_hash = abci_app
            .platform
            .drive
            .grove
            .root_hash(None)
            .unwrap()
            .expect("expected root hash");

        let state = abci_app.platform.state.load();

        let protocol_version = state.current_protocol_version_in_consensus();

        let platform_version =
            PlatformVersion::get(protocol_version).expect("expected platform version");

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
                quorums,
                current_quorum_hash,
                current_proposer_versions: Some(current_proposer_versions),
                current_identity_nonce_counter: identity_nonce_counter,
                current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                start_time_ms: 1681094380000,
                current_time_ms: end_time_ms,
            },
            strategy,
            config,
            StrategyRandomness::SeedEntropy(7),
        );
    }

    #[test]
    fn run_chain_one_identity_in_solitude() {
        let platform_version = PlatformVersion::latest();
        let strategy = NetworkStrategy {
            strategy: Strategy {
                contracts_with_updates: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identities_inserts: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: None,
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
            validator_set_quorum_size: 100,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 25,
                ..Default::default()
            },
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(&mut platform, 100, strategy, config, 15);

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

        assert_eq!(balance, 99864802180)
    }

    #[test]
    fn run_chain_core_height_randomly_increasing() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                contracts_with_updates: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identities_inserts: Frequency {
                    times_per_block_range: Default::default(),
                    chance_per_block: None,
                },
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
            validator_set_quorum_size: 100,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 25,
                ..Default::default()
            },
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        run_chain_for_strategy(&mut platform, 1000, strategy, config, 15);
    }

    #[test]
    fn run_chain_core_height_randomly_increasing_with_epoch_change() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                contracts_with_updates: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identities_inserts: Frequency {
                    times_per_block_range: Default::default(),
                    chance_per_block: None,
                },
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
            validator_set_quorum_size: 100,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 100,
                ..Default::default()
            },
            block_spacing_ms: hour_in_ms,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };

        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(&mut platform, 1000, strategy, config, 15);
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
                contracts_with_updates: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identities_inserts: Frequency {
                    times_per_block_range: Default::default(),
                    chance_per_block: None,
                },
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
            validator_set_quorum_size: 100,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 100,
                epoch_time_length_s: hour_in_s,
                ..Default::default()
            },
            block_spacing_ms: three_mins_in_ms,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };

        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(&mut platform, 1000, strategy, config, 15);
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
                contracts_with_updates: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identities_inserts: Frequency {
                    times_per_block_range: Default::default(),
                    chance_per_block: None,
                },
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
            validator_set_quorum_size: 10,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 25,
                ..Default::default()
            },
            block_spacing_ms: 300,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let ChainExecutionOutcome { abci_app, .. } =
            run_chain_for_strategy(&mut platform, 2000, strategy, config, 40);

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
        assert!(counter.get(&1).unwrap() > &240);
    }

    #[test]
    fn run_chain_core_height_randomly_increasing_with_quorum_updates_new_proposers() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                contracts_with_updates: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identities_inserts: Frequency {
                    times_per_block_range: Default::default(),
                    chance_per_block: None,
                },
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
            validator_set_quorum_size: 10,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 25,
                ..Default::default()
            },
            block_spacing_ms: 300,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let ChainExecutionOutcome { abci_app, .. } =
            run_chain_for_strategy(&mut platform, 300, strategy, config, 43);

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
                contracts_with_updates: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identities_inserts: Frequency {
                    times_per_block_range: Default::default(),
                    chance_per_block: None,
                },
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
            validator_set_quorum_size: 10,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 25,
                ..Default::default()
            },
            block_spacing_ms: 300,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let ChainExecutionOutcome { abci_app, .. } =
            run_chain_for_strategy(&mut platform, 300, strategy, config, 43);

        // With these params if we add new mns the hpmn masternode list would be randomly different than 100.

        let platform = abci_app.platform;
        let platform_state = platform.state.load();

        assert_ne!(platform_state.hpmn_masternode_list().len(), 100);
    }

    #[test]
    fn run_chain_core_height_randomly_increasing_with_quorum_updates_updating_proposers() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                contracts_with_updates: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identities_inserts: Frequency {
                    times_per_block_range: Default::default(),
                    chance_per_block: None,
                },
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
            validator_set_quorum_size: 10,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 25,
                ..Default::default()
            },
            block_spacing_ms: 300,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let ChainExecutionOutcome {
            abci_app,
            proposers,
            ..
        } = run_chain_for_strategy(&mut platform, 300, strategy, config, 43);

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
        // drive_abci::logging::Loggers::default().try_install().ok();
        let strategy = NetworkStrategy {
            strategy: Strategy {
                contracts_with_updates: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identities_inserts: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: None,
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
            ..Default::default()
        };
        let config = PlatformConfig {
            validator_set_quorum_size: 100,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 25,
                ..Default::default()
            },
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(&mut platform, 100, strategy, config, 15);

        assert_eq!(outcome.identities.len(), 100);
    }

    #[test]
    fn run_chain_insert_one_new_identity_per_block_with_epoch_change() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                contracts_with_updates: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identities_inserts: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: None,
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
            validator_set_quorum_size: 100,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 100,
                ..Default::default()
            },
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };

        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(&mut platform, 150, strategy, config, 15);
        assert_eq!(outcome.identities.len(), 150);
        assert_eq!(outcome.masternode_identity_balances.len(), 100);
        let all_have_balances = outcome
            .masternode_identity_balances
            .iter()
            .all(|(_, balance)| *balance != 0);
        assert!(all_have_balances, "all masternodes should have a balance");
        assert_eq!(
            hex::encode(
                outcome
                    .abci_app
                    .platform
                    .drive
                    .grove
                    .root_hash(None)
                    .unwrap()
                    .unwrap()
            ),
            "b91e7b8759189050aa92a00dc5fb240689bb0a58e3a2388c80f8d18156d4eb0b".to_string()
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
                contracts_with_updates: vec![(contract, None)],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identities_inserts: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: None,
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
            validator_set_quorum_size: 100,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 25,
                ..Default::default()
            },
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(&mut platform, 1, strategy, config, 15);

        outcome
            .abci_app
            .platform
            .drive
            .fetch_contract(
                outcome
                    .strategy
                    .strategy
                    .contracts_with_updates
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
                contracts_with_updates: vec![(
                    contract,
                    Some(BTreeMap::from([
                        (3, contract_update_1),
                        (8, contract_update_2),
                    ])),
                )],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identities_inserts: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: None,
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
            validator_set_quorum_size: 100,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 25,
                ..Default::default()
            },
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(&mut platform, 10, strategy, config, 15);

        outcome
            .abci_app
            .platform
            .drive
            .fetch_contract(
                outcome
                    .strategy
                    .strategy
                    .contracts_with_updates
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
                contracts_with_updates: vec![(created_contract, None)],
                operations: vec![Operation {
                    op_type: OperationType::Document(document_op),
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                }],
                start_identities: StartIdentities::default(),
                identities_inserts: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: None,
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
            validator_set_quorum_size: 100,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 25,
                ..Default::default()
            },
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        run_chain_for_strategy(&mut platform, 100, strategy, config, 15);
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
                contracts_with_updates: vec![(created_contract, None)],
                operations: vec![Operation {
                    op_type: OperationType::Document(document_op),
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                }],
                start_identities: StartIdentities::default(),
                identities_inserts: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: None,
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
            validator_set_quorum_size: 100,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 100,
                ..Default::default()
            },
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };
        let block_count = 120;
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(&mut platform, block_count, strategy, config, 15);
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
                contracts_with_updates: vec![(created_contract, None)],
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
                identities_inserts: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: None,
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
            validator_set_quorum_size: 100,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 100,
                ..Default::default()
            },
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };
        let block_count = 120;
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(&mut platform, block_count, strategy, config, 15);
        assert_eq!(outcome.identities.len() as u64, block_count);
        assert_eq!(outcome.masternode_identity_balances.len(), 100);
        let all_have_balances = outcome
            .masternode_identity_balances
            .iter()
            .all(|(_, balance)| *balance != 0);
        assert!(all_have_balances, "all masternodes should have a balance");
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
                contracts_with_updates: vec![(created_contract, None)],
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
                identities_inserts: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: None,
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
            validator_set_quorum_size: 100,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 100,
                ..Default::default()
            },
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };

        let block_count = 120;
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(&mut platform, block_count, strategy, config, 15);
        assert_eq!(outcome.identities.len() as u64, block_count);
        assert_eq!(outcome.masternode_identity_balances.len(), 100);
        let all_have_balances = outcome
            .masternode_identity_balances
            .iter()
            .all(|(_, balance)| *balance != 0);
        assert!(all_have_balances, "all masternodes should have a balance");

        assert_eq!(
            hex::encode(
                outcome
                    .abci_app
                    .platform
                    .drive
                    .grove
                    .root_hash(None)
                    .unwrap()
                    .unwrap()
            ),
            "2200b6da58af74075a6280091d94ad13773e3f7847e4e230b5d5398ede9cf8a5".to_string()
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
                contracts_with_updates: vec![(created_contract, None)],
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
                identities_inserts: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: None,
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
            validator_set_quorum_size: 100,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 100,
                ..Default::default()
            },
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };

        let block_count = 120;
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(&mut platform, block_count, strategy, config, 15);
        assert_eq!(outcome.identities.len() as u64, block_count);
        assert_eq!(outcome.masternode_identity_balances.len(), 100);
        let all_have_balances = outcome
            .masternode_identity_balances
            .iter()
            .all(|(_, balance)| *balance != 0);
        assert!(all_have_balances, "all masternodes should have a balance");
        assert_eq!(
            hex::encode(
                outcome
                    .abci_app
                    .platform
                    .drive
                    .grove
                    .root_hash(None)
                    .unwrap()
                    .unwrap()
            ),
            "87ea052870576604c828f0675517942942f577dcc6fe51d2aff57b062682c899".to_string()
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
                contracts_with_updates: vec![(created_contract, None)],
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
                identities_inserts: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: None,
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
            validator_set_quorum_size: 100,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 100,
                ..Default::default()
            },
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };

        let block_count = 10;
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(&mut platform, block_count, strategy, config, 15);
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
                contracts_with_updates: vec![(created_contract, None)],
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
                identities_inserts: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: None,
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
            validator_set_quorum_size: 100,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 100,
                ..Default::default()
            },
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };

        let block_count = 10;
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(&mut platform, block_count, strategy, config, 15);
        for tx_results_per_block in outcome.state_transition_results_per_block.values() {
            for (state_transition, _unused_result) in tx_results_per_block {
                // We can't ever get a documents batch transition, because the proposer will remove it from a block
                assert!(!matches!(
                    state_transition,
                    StateTransition::DocumentsBatch(_)
                ));
            }
        }
    }

    #[test]
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
                contracts_with_updates: vec![(created_contract, None)],
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
                identities_inserts: Frequency {
                    times_per_block_range: 1..30,
                    chance_per_block: None,
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
            validator_set_quorum_size: 100,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 100,
                epoch_time_length_s: 1576800,
                ..Default::default()
            },
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };
        let block_count = 30;
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(&mut platform, block_count, strategy, config, 15);
        assert_eq!(outcome.identities.len() as u64, 421);
        assert_eq!(outcome.masternode_identity_balances.len(), 100);
        let balance_count = outcome
            .masternode_identity_balances
            .into_iter()
            .filter(|(_, balance)| *balance != 0)
            .count();
        assert_eq!(balance_count, 19); // 1 epoch worth of proposers
    }

    #[test]
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
            action: DocumentActionReplace,
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
                contracts_with_updates: vec![(created_contract, None)],
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
                identities_inserts: Frequency {
                    times_per_block_range: 1..6,
                    chance_per_block: None,
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
            validator_set_quorum_size: 100,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 100,
                epoch_time_length_s: 1576800,
                ..Default::default()
            },
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };
        let block_count = 30;
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(&mut platform, block_count, strategy, config, 15);
        assert_eq!(outcome.identities.len() as u64, 86);
        assert_eq!(outcome.masternode_identity_balances.len(), 100);
        let balance_count = outcome
            .masternode_identity_balances
            .into_iter()
            .filter(|(_, balance)| *balance != 0)
            .count();
        assert_eq!(balance_count, 19); // 1 epoch worth of proposers
    }

    #[test]
    fn run_chain_top_up_identities() {
        drive_abci::logging::init_for_tests(LogLevel::Silent);

        let strategy = NetworkStrategy {
            strategy: Strategy {
                contracts_with_updates: vec![],
                operations: vec![Operation {
                    op_type: OperationType::IdentityTopUp,
                    frequency: Frequency {
                        times_per_block_range: 1..3,
                        chance_per_block: None,
                    },
                }],
                start_identities: StartIdentities::default(),
                identities_inserts: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: None,
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
            validator_set_quorum_size: 100,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 25,
                ..Default::default()
            },
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(&mut platform, 10, strategy, config, 15);

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
                contracts_with_updates: vec![],
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
                identities_inserts: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: None,
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
            validator_set_quorum_size: 100,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 25,
                ..Default::default()
            },
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(&mut platform, 10, strategy, config, 15);
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
                contracts_with_updates: vec![],
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
                identities_inserts: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: None,
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
            validator_set_quorum_size: 100,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 25,
                ..Default::default()
            },
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(&mut platform, 10, strategy, config, 15);

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
    fn run_chain_top_up_and_withdraw_from_identities() {
        let platform_version = PlatformVersion::latest();
        let strategy = NetworkStrategy {
            strategy: Strategy {
                contracts_with_updates: vec![],
                operations: vec![
                    Operation {
                        op_type: OperationType::IdentityTopUp,
                        frequency: Frequency {
                            times_per_block_range: 1..4,
                            chance_per_block: None,
                        },
                    },
                    Operation {
                        op_type: OperationType::IdentityWithdrawal,
                        frequency: Frequency {
                            times_per_block_range: 1..4,
                            chance_per_block: None,
                        },
                    },
                ],
                start_identities: StartIdentities::default(),
                identities_inserts: Frequency {
                    times_per_block_range: 1..2,
                    chance_per_block: None,
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
            // because we can add an identity and withdraw from it in the same block
            // the result would be different then expected
            verify_state_transition_results: false,
            ..Default::default()
        };
        let config = PlatformConfig {
            validator_set_quorum_size: 100,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 25,
                ..Default::default()
            },
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };

        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        platform
            .core_rpc
            .expect_send_raw_transaction()
            .returning(move |_| Ok(Txid::all_zeros()));

        struct CoreState {
            asset_unlock_statuses: BTreeMap<WithdrawalTransactionIndex, AssetUnlockStatusResult>,
            chain_lock: ChainLock,
        }

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
        // - Block 2: tops up identity and initiates withdrawals
        let (
            ChainExecutionOutcome {
                abci_app,
                proposers,
                quorums,
                current_quorum_hash,
                current_proposer_versions,
                end_time_ms,
                identity_nonce_counter,
                identity_contract_nonce_counter,
                ..
            },
            last_block_pooled_withdrawals_amount,
        ) = {
            let outcome =
                run_chain_for_strategy(&mut platform, 2, strategy.clone(), config.clone(), 1);

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
            let pooled_withdrawals = withdrawal_documents_pooled.len();

            (outcome, pooled_withdrawals)
        };

        // Run block 3
        // Should broadcast previously pooled withdrawals to core
        let ChainExecutionOutcome {
            abci_app,
            proposers,
            quorums,
            current_quorum_hash,
            current_proposer_versions,
            end_time_ms,
            withdrawals: last_block_withdrawals,
            identity_nonce_counter,
            identity_contract_nonce_counter,
            ..
        } = {
            let outcome = continue_chain_for_strategy(
                abci_app,
                ChainExecutionParameters {
                    block_start: 3,
                    core_height_start: 1,
                    block_count: 1,
                    proposers,
                    quorums,
                    current_quorum_hash,
                    current_proposer_versions: Some(current_proposer_versions),
                    current_identity_nonce_counter: identity_nonce_counter,
                    current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                    start_time_ms: GENESIS_TIME_MS,
                    current_time_ms: end_time_ms,
                },
                strategy.clone(),
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

        // Run block 4
        // Should change pooled status to broadcasted
        let last_block_broadcased_withdrawals_amount = last_block_withdrawals.len();
        let (
            ChainExecutionOutcome {
                abci_app,
                proposers,
                quorums,
                current_quorum_hash,
                current_proposer_versions,
                end_time_ms,
                withdrawals: last_block_withdrawals,
                identity_nonce_counter,
                identity_contract_nonce_counter,
                ..
            },
            last_block_broadcased_withdrawals_amount,
        ) = {
            let outcome = continue_chain_for_strategy(
                abci_app,
                ChainExecutionParameters {
                    block_start: 4,
                    core_height_start: 1,
                    block_count: 1,
                    proposers,
                    quorums,
                    current_quorum_hash,
                    current_proposer_versions: Some(current_proposer_versions),
                    current_identity_nonce_counter: identity_nonce_counter,
                    current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                    start_time_ms: GENESIS_TIME_MS,
                    current_time_ms: end_time_ms + 1000,
                },
                strategy.clone(),
                config.clone(),
                StrategyRandomness::SeedEntropy(3),
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

            // In this block we should have new withdrawals pooled
            assert!(!withdrawal_documents_pooled.is_empty());

            // And extra withdrawals broadcasted
            let withdrawals_broadcasted_expected =
                last_block_broadcased_withdrawals_amount + outcome.withdrawals.len();
            assert_eq!(
                withdrawal_documents_broadcasted.len(),
                withdrawals_broadcasted_expected
            );

            (outcome, withdrawal_documents_broadcasted.len())
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

            // Then increase chainlocked height, so that withdrawals for chainlocked tranasctions
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

        // Run block 5
        // Previously broadcasted transactions should be settled after block 5,
        // and their corresponding statuses should be changed to COMPLETED
        let (
            ChainExecutionOutcome {
                abci_app,
                proposers,
                quorums,
                current_quorum_hash,
                current_proposer_versions,
                end_time_ms,
                withdrawals: last_block_withdrawals,
                identity_nonce_counter,
                identity_contract_nonce_counter,
                ..
            },
            last_block_withdrawals_completed_amount,
        ) = {
            let outcome = continue_chain_for_strategy(
                abci_app,
                ChainExecutionParameters {
                    block_start: 5,
                    core_height_start: 1,
                    block_count: 1,
                    proposers,
                    quorums,
                    current_quorum_hash,
                    current_proposer_versions: Some(current_proposer_versions),
                    current_identity_nonce_counter: identity_nonce_counter,
                    current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                    start_time_ms: GENESIS_TIME_MS,
                    current_time_ms: end_time_ms + 1000,
                },
                strategy.clone(),
                config.clone(),
                StrategyRandomness::SeedEntropy(4),
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
            assert!(!withdrawal_documents_pooled.is_empty());

            // And some withdrawals completed
            let withdrawals_completed_expected =
                // Withdrawals issued on {previous_block - 1} considered completed
                last_block_broadcased_withdrawals_amount - last_block_withdrawals.len();
            assert_eq!(
                withdrawal_documents_completed.len(),
                withdrawals_completed_expected
            );

            // And extra withdrawals broadcasted
            let withdrawals_broadcasted_expected =
                // Withdrawals issued on previous block + withdrawals from this block are still in broadcasted state
                last_block_withdrawals.len() + outcome.withdrawals.len();

            assert_eq!(
                withdrawal_documents_broadcasted.len(),
                withdrawals_broadcasted_expected
            );

            (outcome, withdrawal_documents_completed.len())
        };

        // Update state of the core before proceeding to the next block
        {
            // Simulate transactions being added to the core mempool
            let mut core_state = shared_core_state.lock().unwrap();

            let number_of_blocks_before_expiration: u32 = 48;
            chain_locked_height += number_of_blocks_before_expiration;

            core_state.chain_lock.block_height = chain_locked_height;

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

        // Run block 6.
        // Tests withdrawal expiration
        let ChainExecutionOutcome { .. } = {
            let outcome = continue_chain_for_strategy(
                abci_app,
                ChainExecutionParameters {
                    block_start: 6,
                    core_height_start: 1,
                    block_count: 1,
                    proposers,
                    quorums,
                    current_quorum_hash,
                    current_proposer_versions: Some(current_proposer_versions),
                    current_identity_nonce_counter: identity_nonce_counter,
                    current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                    start_time_ms: GENESIS_TIME_MS,
                    current_time_ms: end_time_ms + 1000,
                },
                strategy.clone(),
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

            // In this block we should have new withdrawals pooled
            assert!(!withdrawal_documents_pooled.is_empty());

            // Amount of completed withdrawals stays the same as in the last block
            assert_eq!(
                withdrawal_documents_completed.len(),
                last_block_withdrawals_completed_amount
            );

            // And some withdrawals got expired
            let withdrawals_expired_expected =
                // Withdrawals issued on {previous_block - 1}, but not chainlocked yet, considered expired
                last_block_broadcased_withdrawals_amount - last_block_withdrawals.len();

            assert_eq!(
                withdrawal_documents_expired.len(),
                withdrawals_expired_expected
            );

            // And extra withdrawals broadcasted
            let withdrawals_broadcasted_expected =
                // Withdrawals issued on previous block + withdrawals from this block are still in broadcasted state
                last_block_withdrawals.len() + outcome.withdrawals.len();

            assert_eq!(
                withdrawal_documents_broadcasted.len(),
                withdrawals_broadcasted_expected
            );

            outcome
        };
    }

    #[test]
    fn run_chain_rotation_is_deterministic_1_block() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                contracts_with_updates: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identities_inserts: Frequency {
                    //we do this to create some paying transactions
                    times_per_block_range: 1..2,
                    chance_per_block: None,
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
            validator_set_quorum_size: 3,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 1,
                ..Default::default()
            },
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
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

            let outcome = run_chain_for_strategy(platform, 1, strategy.clone(), config.clone(), 7);
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

        let first_last_app_hash = outcomes[0]
            .abci_app
            .platform
            .drive
            .grove
            .root_hash(None)
            .unwrap()
            .expect("should return app hash");

        assert!(outcomes.iter().all(|outcome| {
            let last_app_hash = outcome
                .abci_app
                .platform
                .drive
                .grove
                .root_hash(None)
                .unwrap()
                .expect("should return app hash");

            last_app_hash == first_last_app_hash
        }));
    }

    #[test]
    fn run_chain_heavy_rotation_deterministic_before_payout() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                contracts_with_updates: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identities_inserts: Frequency {
                    //we do this to create some paying transactions
                    times_per_block_range: 1..2,
                    chance_per_block: None,
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
            validator_set_quorum_size: 3,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 1,
                epoch_time_length_s: 1576800,
                ..Default::default()
            },
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
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

        let outcome_a =
            run_chain_for_strategy(&mut platform_a, 18, strategy.clone(), config.clone(), 7);
        let outcome_b = run_chain_for_strategy(&mut platform_b, 18, strategy, config, 7);
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

        let last_app_hash_a = outcome_a
            .abci_app
            .platform
            .drive
            .grove
            .root_hash(None)
            .unwrap()
            .expect("should return app hash");

        let last_app_hash_b = outcome_b
            .abci_app
            .platform
            .drive
            .grove
            .root_hash(None)
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
                contracts_with_updates: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identities_inserts: Frequency {
                    //we do this to create some paying transactions
                    times_per_block_range: 1..2,
                    chance_per_block: None,
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
            validator_set_quorum_size: 3,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 1,
                ..Default::default()
            },
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
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

        let outcome_a =
            run_chain_for_strategy(&mut platform_a, 100, strategy.clone(), config.clone(), 7);
        let outcome_b = run_chain_for_strategy(&mut platform_b, 100, strategy, config, 7);
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

        let last_app_hash_a = outcome_a
            .abci_app
            .platform
            .drive
            .grove
            .root_hash(None)
            .unwrap()
            .expect("should return app hash");

        let last_app_hash_b = outcome_b
            .abci_app
            .platform
            .drive
            .grove
            .root_hash(None)
            .unwrap()
            .expect("should return app hash");

        assert_eq!(last_app_hash_a, last_app_hash_b);

        let balance_count = outcome_a
            .masternode_identity_balances
            .into_iter()
            .filter(|(_, balance)| *balance != 0)
            .count();
        // we have a maximum 90 quorums, that could have been used, 6 were used twice
        assert_eq!(balance_count, 84);
    }

    #[test]
    fn run_chain_stop_and_restart_with_rotation() {
        drive_abci::logging::init_for_tests(LogLevel::Silent);

        let strategy = NetworkStrategy {
            strategy: Strategy {
                contracts_with_updates: vec![],
                operations: vec![],
                start_identities: StartIdentities::default(),
                identities_inserts: Frequency {
                    times_per_block_range: Default::default(),
                    chance_per_block: None,
                },
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
            validator_set_quorum_size: 3,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 1,
                epoch_time_length_s: 1576800,
                ..Default::default()
            },
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
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
            quorums,
            current_quorum_hash,
            current_proposer_versions,
            end_time_ms,
            identity_nonce_counter,
            identity_contract_nonce_counter,
            ..
        } = run_chain_for_strategy(&mut platform, 100, strategy.clone(), config.clone(), 89);

        let state = abci_app.platform.state.load();
        let protocol_version = state.current_protocol_version_in_consensus();

        let platform_version = PlatformVersion::get(protocol_version).unwrap();

        let known_root_hash = abci_app
            .platform
            .drive
            .grove
            .root_hash(None)
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
                quorums,
                current_quorum_hash,
                current_proposer_versions: Some(current_proposer_versions),
                current_identity_nonce_counter: identity_nonce_counter,
                current_identity_contract_nonce_counter: identity_contract_nonce_counter,
                start_time_ms: 1681094380000,
                current_time_ms: end_time_ms,
            },
            strategy,
            config,
            StrategyRandomness::SeedEntropy(block_start),
        );
    }

    #[test]
    fn run_chain_transfer_between_identities() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                contracts_with_updates: vec![],
                operations: vec![Operation {
                    op_type: OperationType::IdentityTransfer,
                    frequency: Frequency {
                        times_per_block_range: 1..3,
                        chance_per_block: None,
                    },
                }],
                start_identities: StartIdentities::default(),
                identities_inserts: Frequency {
                    times_per_block_range: 6..10,
                    chance_per_block: None,
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
            validator_set_quorum_size: 100,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 25,
                ..Default::default()
            },
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };

        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(&mut platform, 15, strategy, config, 15);

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
            )
            .expect("expected to fetch balances");

        assert_eq!(outcome.identities.len(), 110);
    }

    // Test should filter out transactions exceeding max tx bytes per block
    #[test]
    fn run_transactions_exceeding_max_block_size() {
        let strategy = NetworkStrategy {
            strategy: Strategy {
                identities_inserts: Frequency {
                    times_per_block_range: 5..6,
                    chance_per_block: None,
                },
                ..Default::default()
            },
            max_tx_bytes_per_block: 3500,
            ..Default::default()
        };
        let config = PlatformConfig {
            validator_set_quorum_size: 100,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 25,
                ..Default::default()
            },
            block_spacing_ms: 3000,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            ..Default::default()
        };
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let outcome = run_chain_for_strategy(&mut platform, 1, strategy, config, 15);
        let state_transitions = outcome
            .state_transition_results_per_block
            .get(&1)
            .expect("expected state transition results");

        // Only three out of five transitions should've made to the block
        assert_eq!(state_transitions.len(), 3);
    }
}
