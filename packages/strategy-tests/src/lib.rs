
//! Strategies
//!

extern crate core;

use dashcore_rpc::dashcore::QuorumHash;

use dpp::bls_signatures::PrivateKey as BlsPrivateKey;

use drive_abci::test::helpers::setup::TestPlatformBuilder;
use drive_abci::{config::PlatformConfig, test::helpers::setup::TempPlatform};
use frequency::Frequency;

use std::collections::BTreeMap;

use strategy::{ChainExecutionOutcome, ChainExecutionParameters, Strategy, StrategyRandomness};

mod core_update_tests;
mod execution;
mod failures;
mod frequency;
mod masternode_list_item_helpers;
mod masternodes;
mod operations;
mod query;
mod strategy;
mod transitions;
mod upgrade_fork_tests;
mod verify_state_transitions;

pub type BlockHeight = u64;

use crate::execution::{continue_chain_for_strategy, run_chain_for_strategy};
use crate::operations::DocumentAction::DocumentActionReplace;
use crate::operations::{
    DocumentAction, DocumentOp, IdentityUpdateOp, Operation, OperationType,
};
use crate::query::QueryStrategy;
use crate::strategy::MasternodeListChangesStrategy;
use dashcore_rpc::dashcore::hashes::Hash;
use dashcore_rpc::dashcore::BlockHash;
use dashcore_rpc::dashcore_rpc_json::ExtendedQuorumDetails;
use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;

use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use dpp::data_contract::document_type::random_document::{
    DocumentFieldFillSize, DocumentFieldFillType,
};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::tests::json_document::json_document_to_created_contract;
use dpp::util::hash::hash_to_hex_string;
use dpp::version::PlatformVersion;
use drive_abci::config::PlatformTestConfig;
use drive_abci::logging::LogLevelPreset;
use drive_abci::platform_types::platform_state::v0::PlatformStateV0Methods;
use drive_abci::rpc::core::QuorumListExtendedInfo;
use itertools::Itertools;
use tenderdash_abci::proto::abci::{RequestInfo, ResponseInfo};
use tenderdash_abci::proto::types::CoreChainLock;
use tenderdash_abci::Application;

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


fn run_chain_insert_one_new_identity_and_a_contract() {
    let platform_version = PlatformVersion::latest();
    let contract = json_document_to_created_contract(
        "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
        true,
        platform_version,
    )
    .expect("expected to get contract from a json document");

    let strategy = Strategy {
        contracts_with_updates: vec![(contract, None)],
        operations: vec![],
        start_identities: vec![],
        identities_inserts: Frequency {
            times_per_block_range: 1..2,
            chance_per_block: None,
        },
        total_hpmns: 100,
        extra_normal_mns: 0,
        quorum_count: 24,
        upgrading_info: None,
        core_height_increase: Frequency {
            times_per_block_range: Default::default(),
            chance_per_block: None,
        },
        proposer_strategy: Default::default(),
        rotate_quorums: false,
        failure_testing: None,
        query_testing: None,
        verify_state_transition_results: true,
        signer: None,
    };
    let config = PlatformConfig {
        verify_sum_trees: true,
        quorum_size: 100,
        validator_set_quorum_rotation_block_count: 25,
        block_spacing_ms: 3000,
        testing_configs: PlatformTestConfig::default_with_no_block_signing(),
        ..Default::default()
    };
    let mut platform = TestPlatformBuilder::new()
        .with_config(config.clone())
        .build_with_mock_rpc();
    platform
        .core_rpc
        .expect_get_best_chain_lock()
        .returning(move || {
            Ok(CoreChainLock {
                core_block_height: 10,
                core_block_hash: [1; 32].to_vec(),
                signature: [2; 96].to_vec(),
            })
        });
    let outcome = run_chain_for_strategy(&mut platform, 1, strategy, config, 15);

    outcome
        .abci_app
        .platform
        .drive
        .fetch_contract(
            outcome
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


fn run_chain_insert_one_new_identity_and_a_contract_with_updates() {
    let platform_version = PlatformVersion::latest();
    let contract = json_document_to_created_contract(
        "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
        true,
        platform_version,
    )
    .expect("expected to get contract from a json document");

    let mut contract_update_1 = json_document_to_created_contract(
        "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable-update-1.json",
        true,
        platform_version,
    )
    .expect("expected to get contract from a json document");

    //todo: versions should start at 0 (so this should be 1)
    contract_update_1.data_contract_mut().set_version(2);

    let mut contract_update_2 = json_document_to_created_contract(
        "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable-update-2.json",
        true,
        platform_version,
    )
    .expect("expected to get contract from a json document");

    contract_update_2.data_contract_mut().set_version(3);

    let strategy = Strategy {
        contracts_with_updates: vec![(
            contract,
            Some(BTreeMap::from([
                (3, contract_update_1),
                (8, contract_update_2),
            ])),
        )],
        operations: vec![],
        start_identities: vec![],
        identities_inserts: Frequency {
            times_per_block_range: 1..2,
            chance_per_block: None,
        },
        total_hpmns: 100,
        extra_normal_mns: 0,
        quorum_count: 24,
        upgrading_info: None,
        core_height_increase: Frequency {
            times_per_block_range: Default::default(),
            chance_per_block: None,
        },
        proposer_strategy: Default::default(),
        rotate_quorums: false,
        failure_testing: None,
        query_testing: None,
        verify_state_transition_results: true,
        signer: None,
    };
    let config = PlatformConfig {
        verify_sum_trees: true,
        quorum_size: 100,
        validator_set_quorum_rotation_block_count: 25,
        block_spacing_ms: 3000,
        testing_configs: PlatformTestConfig::default_with_no_block_signing(),
        ..Default::default()
    };
    let mut platform = TestPlatformBuilder::new()
        .with_config(config.clone())
        .build_with_mock_rpc();
    platform
        .core_rpc
        .expect_get_best_chain_lock()
        .returning(move || {
            Ok(CoreChainLock {
                core_block_height: 10,
                core_block_hash: [1; 32].to_vec(),
                signature: [2; 96].to_vec(),
            })
        });
    let outcome = run_chain_for_strategy(&mut platform, 10, strategy, config, 15);

    outcome
        .abci_app
        .platform
        .drive
        .fetch_contract(
            outcome
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


fn run_chain_insert_one_new_identity_per_block_and_one_new_document() {
    let platform_version = PlatformVersion::latest();
    let created_contract = json_document_to_created_contract(
        "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
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

    let strategy = Strategy {
        contracts_with_updates: vec![(created_contract, None)],
        operations: vec![Operation {
            op_type: OperationType::Document(document_op),
            frequency: Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            },
        }],
        start_identities: vec![],
        identities_inserts: Frequency {
            times_per_block_range: 1..2,
            chance_per_block: None,
        },
        total_hpmns: 100,
        extra_normal_mns: 0,
        quorum_count: 24,
        upgrading_info: None,
        core_height_increase: Frequency {
            times_per_block_range: Default::default(),
            chance_per_block: None,
        },
        proposer_strategy: Default::default(),
        rotate_quorums: false,
        failure_testing: None,
        query_testing: None,
        verify_state_transition_results: true,
        signer: None,
    };
    let config = PlatformConfig {
        verify_sum_trees: true,
        quorum_size: 100,
        validator_set_quorum_rotation_block_count: 25,
        block_spacing_ms: 3000,
        testing_configs: PlatformTestConfig::default_with_no_block_signing(),
        ..Default::default()
    };
    let mut platform = TestPlatformBuilder::new()
        .with_config(config.clone())
        .build_with_mock_rpc();
    platform
        .core_rpc
        .expect_get_best_chain_lock()
        .returning(move || {
            Ok(CoreChainLock {
                core_block_height: 10,
                core_block_hash: [1; 32].to_vec(),
                signature: [2; 96].to_vec(),
            })
        });
    run_chain_for_strategy(&mut platform, 100, strategy, config, 15);
}


fn run_chain_insert_one_new_identity_per_block_and_a_document_with_epoch_change() {
    let platform_version = PlatformVersion::latest();
    let created_contract = json_document_to_created_contract(
        "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
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

    let strategy = Strategy {
        contracts_with_updates: vec![(created_contract, None)],
        operations: vec![Operation {
            op_type: OperationType::Document(document_op),
            frequency: Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            },
        }],
        start_identities: vec![],
        identities_inserts: Frequency {
            times_per_block_range: 1..2,
            chance_per_block: None,
        },
        total_hpmns: 100,
        extra_normal_mns: 0,
        quorum_count: 24,
        upgrading_info: None,
        core_height_increase: Frequency {
            times_per_block_range: Default::default(),
            chance_per_block: None,
        },
        proposer_strategy: Default::default(),
        rotate_quorums: false,
        failure_testing: None,
        query_testing: None,
        verify_state_transition_results: true,
        signer: None,
    };
    let day_in_ms = 1000 * 60 * 60 * 24;
    let config = PlatformConfig {
        verify_sum_trees: true,
        quorum_size: 100,
        validator_set_quorum_rotation_block_count: 100,
        block_spacing_ms: day_in_ms,
        testing_configs: PlatformTestConfig::default_with_no_block_signing(),
        ..Default::default()
    };
    let block_count = 120;
    let mut platform = TestPlatformBuilder::new()
        .with_config(config.clone())
        .build_with_mock_rpc();
    platform
        .core_rpc
        .expect_get_best_chain_lock()
        .returning(move || {
            Ok(CoreChainLock {
                core_block_height: 10,
                core_block_hash: [1; 32].to_vec(),
                signature: [2; 96].to_vec(),
            })
        });
    let outcome = run_chain_for_strategy(&mut platform, block_count, strategy, config, 15);
    assert_eq!(outcome.identities.len() as u64, block_count);
    assert_eq!(outcome.masternode_identity_balances.len(), 100);
    let all_have_balances = outcome
        .masternode_identity_balances
        .iter()
        .all(|(_, balance)| *balance != 0);
    assert!(all_have_balances, "all masternodes should have a balance");
}


fn run_chain_insert_one_new_identity_per_block_document_insertions_and_deletions_with_epoch_change(
) {
    let platform_version = PlatformVersion::latest();
    let created_contract = json_document_to_created_contract(
        "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
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

    let strategy = Strategy {
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
        start_identities: vec![],
        identities_inserts: Frequency {
            times_per_block_range: 1..2,
            chance_per_block: None,
        },
        total_hpmns: 100,
        extra_normal_mns: 0,
        quorum_count: 24,
        upgrading_info: None,
        core_height_increase: Frequency {
            times_per_block_range: Default::default(),
            chance_per_block: None,
        },
        proposer_strategy: Default::default(),
        rotate_quorums: false,
        failure_testing: None,
        query_testing: None,
        verify_state_transition_results: true,
        signer: None,
    };
    let day_in_ms = 1000 * 60 * 60 * 24;
    let config = PlatformConfig {
        verify_sum_trees: true,
        quorum_size: 100,
        validator_set_quorum_rotation_block_count: 100,
        block_spacing_ms: day_in_ms,
        testing_configs: PlatformTestConfig::default_with_no_block_signing(),
        ..Default::default()
    };
    let block_count = 120;
    let mut platform = TestPlatformBuilder::new()
        .with_config(config.clone())
        .build_with_mock_rpc();
    platform
        .core_rpc
        .expect_get_best_chain_lock()
        .returning(move || {
            Ok(CoreChainLock {
                core_block_height: 10,
                core_block_hash: [1; 32].to_vec(),
                signature: [2; 96].to_vec(),
            })
        });
    let outcome = run_chain_for_strategy(&mut platform, block_count, strategy, config, 15);
    assert_eq!(outcome.identities.len() as u64, block_count);
    assert_eq!(outcome.masternode_identity_balances.len(), 100);
    let all_have_balances = outcome
        .masternode_identity_balances
        .iter()
        .all(|(_, balance)| *balance != 0);
    assert!(all_have_balances, "all masternodes should have a balance");
}


fn run_chain_insert_one_new_identity_per_block_many_document_insertions_and_deletions_with_epoch_change(
) {
    let platform_version = PlatformVersion::latest();
    let created_contract = json_document_to_created_contract(
        "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
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

    let strategy = Strategy {
        contracts_with_updates: vec![(created_contract, None)],
        operations: vec![
            Operation {
                op_type: OperationType::Document(document_insertion_op),
                frequency: Frequency {
                    times_per_block_range: 1..10,
                    chance_per_block: None,
                },
            },
            Operation {
                op_type: OperationType::Document(document_deletion_op),
                frequency: Frequency {
                    times_per_block_range: 1..10,
                    chance_per_block: None,
                },
            },
        ],
        start_identities: vec![],
        identities_inserts: Frequency {
            times_per_block_range: 1..2,
            chance_per_block: None,
        },
        total_hpmns: 100,
        extra_normal_mns: 0,
        quorum_count: 24,
        upgrading_info: None,
        core_height_increase: Frequency {
            times_per_block_range: Default::default(),
            chance_per_block: None,
        },
        proposer_strategy: Default::default(),
        rotate_quorums: false,
        failure_testing: None,
        query_testing: None,
        verify_state_transition_results: true,
        signer: None,
    };
    let day_in_ms = 1000 * 60 * 60 * 24;
    let config = PlatformConfig {
        verify_sum_trees: true,
        quorum_size: 100,
        validator_set_quorum_rotation_block_count: 100,
        block_spacing_ms: day_in_ms,
        testing_configs: PlatformTestConfig::default_with_no_block_signing(),
        ..Default::default()
    };

    let block_count = 120;
    let mut platform = TestPlatformBuilder::new()
        .with_config(config.clone())
        .build_with_mock_rpc();
    platform
        .core_rpc
        .expect_get_best_chain_lock()
        .returning(move || {
            Ok(CoreChainLock {
                core_block_height: 10,
                core_block_hash: [1; 32].to_vec(),
                signature: [2; 96].to_vec(),
            })
        });
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
        "5fbeeffc449c8cea80095ced4debc4b33f2246bfc108c567d07b1a45747fd8df".to_string()
    )
}


fn run_chain_insert_many_new_identity_per_block_many_document_insertions_and_deletions_with_epoch_change(
) {
    let platform_version = PlatformVersion::latest();
    let created_contract = json_document_to_created_contract(
        "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
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

    let strategy = Strategy {
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
        start_identities: vec![],
        identities_inserts: Frequency {
            times_per_block_range: 1..30,
            chance_per_block: None,
        },
        total_hpmns: 100,
        extra_normal_mns: 0,
        quorum_count: 24,
        upgrading_info: None,
        core_height_increase: Frequency {
            times_per_block_range: Default::default(),
            chance_per_block: None,
        },
        proposer_strategy: Default::default(),
        rotate_quorums: false,
        failure_testing: None,
        query_testing: None,
        verify_state_transition_results: true,
        signer: None,
    };

    let day_in_ms = 1000 * 60 * 60 * 24;

    let config = PlatformConfig {
        verify_sum_trees: true,
        quorum_size: 100,
        validator_set_quorum_rotation_block_count: 100,
        block_spacing_ms: day_in_ms,
        testing_configs: PlatformTestConfig::default_with_no_block_signing(),
        ..Default::default()
    };
    let block_count = 30;
    let mut platform = TestPlatformBuilder::new()
        .with_config(config.clone())
        .build_with_mock_rpc();
    platform
        .core_rpc
        .expect_get_best_chain_lock()
        .returning(move || {
            Ok(CoreChainLock {
                core_block_height: 10,
                core_block_hash: [1; 32].to_vec(),
                signature: [2; 96].to_vec(),
            })
        });
    let outcome = run_chain_for_strategy(&mut platform, block_count, strategy, config, 15);
    assert_eq!(outcome.identities.len() as u64, 417);
    assert_eq!(outcome.masternode_identity_balances.len(), 100);
    let balance_count = outcome
        .masternode_identity_balances
        .into_iter()
        .filter(|(_, balance)| *balance != 0)
        .count();
    assert_eq!(balance_count, 19); // 1 epoch worth of proposers
}


fn run_chain_insert_many_new_identity_per_block_many_document_insertions_updates_and_deletions_with_epoch_change(
) {
    let platform_version = PlatformVersion::latest();
    let created_contract = json_document_to_created_contract(
        "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
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

    let strategy = Strategy {
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
        start_identities: vec![],
        identities_inserts: Frequency {
            times_per_block_range: 1..6,
            chance_per_block: None,
        },
        total_hpmns: 100,
        extra_normal_mns: 0,
        quorum_count: 24,
        upgrading_info: None,
        core_height_increase: Frequency {
            times_per_block_range: Default::default(),
            chance_per_block: None,
        },
        proposer_strategy: Default::default(),
        rotate_quorums: false,
        failure_testing: None,
        query_testing: None,
        verify_state_transition_results: true,
        signer: None,
    };

    let day_in_ms = 1000 * 60 * 60 * 24;

    let config = PlatformConfig {
        verify_sum_trees: true,
        quorum_size: 100,
        validator_set_quorum_rotation_block_count: 100,
        block_spacing_ms: day_in_ms,
        testing_configs: PlatformTestConfig::default_with_no_block_signing(),
        ..Default::default()
    };
    let block_count = 30;
    let mut platform = TestPlatformBuilder::new()
        .with_config(config.clone())
        .build_with_mock_rpc();
    platform
        .core_rpc
        .expect_get_best_chain_lock()
        .returning(move || {
            Ok(CoreChainLock {
                core_block_height: 10,
                core_block_hash: [1; 32].to_vec(),
                signature: [2; 96].to_vec(),
            })
        });
    let outcome = run_chain_for_strategy(&mut platform, block_count, strategy, config, 15);
    assert_eq!(outcome.identities.len() as u64, 80);
    assert_eq!(outcome.masternode_identity_balances.len(), 100);
    let balance_count = outcome
        .masternode_identity_balances
        .into_iter()
        .filter(|(_, balance)| *balance != 0)
        .count();
    assert_eq!(balance_count, 19); // 1 epoch worth of proposers
}


fn run_chain_top_up_identities() {
    drive_abci::logging::init_for_tests(LogLevelPreset::Silent);

    let strategy = Strategy {
        contracts_with_updates: vec![],
        operations: vec![Operation {
            op_type: OperationType::IdentityTopUp,
            frequency: Frequency {
                times_per_block_range: 1..3,
                chance_per_block: None,
            },
        }],
        start_identities: vec![],
        identities_inserts: Frequency {
            times_per_block_range: 1..2,
            chance_per_block: None,
        },
        total_hpmns: 100,
        extra_normal_mns: 0,
        quorum_count: 24,
        upgrading_info: None,
        core_height_increase: Frequency {
            times_per_block_range: Default::default(),
            chance_per_block: None,
        },
        proposer_strategy: Default::default(),
        rotate_quorums: false,
        failure_testing: None,
        query_testing: None,
        verify_state_transition_results: true,
        signer: None,
    };
    let config = PlatformConfig {
        verify_sum_trees: true,
        quorum_size: 100,
        validator_set_quorum_rotation_block_count: 25,
        block_spacing_ms: 3000,
        testing_configs: PlatformTestConfig::default_with_no_block_signing(),
        ..Default::default()
    };
    let mut platform = TestPlatformBuilder::new()
        .with_config(config.clone())
        .build_with_mock_rpc();
    platform
        .core_rpc
        .expect_get_best_chain_lock()
        .returning(move || {
            Ok(CoreChainLock {
                core_block_height: 10,
                core_block_hash: [1; 32].to_vec(),
                signature: [2; 96].to_vec(),
            })
        });
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


fn run_chain_update_identities_add_keys() {
    let strategy = Strategy {
        contracts_with_updates: vec![],
        operations: vec![Operation {
            op_type: OperationType::IdentityUpdate(IdentityUpdateOp::IdentityUpdateAddKeys(3)),
            frequency: Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            },
        }],
        start_identities: vec![],
        identities_inserts: Frequency {
            times_per_block_range: 1..2,
            chance_per_block: None,
        },
        total_hpmns: 100,
        extra_normal_mns: 0,
        quorum_count: 24,
        upgrading_info: None,
        core_height_increase: Frequency {
            times_per_block_range: Default::default(),
            chance_per_block: None,
        },
        proposer_strategy: Default::default(),
        rotate_quorums: false,
        failure_testing: None,
        query_testing: None,
        // because we can add an identity and add keys to it in the same block
        // the result would be different then expected
        verify_state_transition_results: false,
        signer: None,
    };
    let config = PlatformConfig {
        verify_sum_trees: true,
        quorum_size: 100,
        validator_set_quorum_rotation_block_count: 25,
        block_spacing_ms: 3000,
        testing_configs: PlatformTestConfig::default_with_no_block_signing(),
        ..Default::default()
    };
    let mut platform = TestPlatformBuilder::new()
        .with_config(config.clone())
        .build_with_mock_rpc();
    platform
        .core_rpc
        .expect_get_best_chain_lock()
        .returning(move || {
            Ok(CoreChainLock {
                core_block_height: 10,
                core_block_hash: [1; 32].to_vec(),
                signature: [2; 96].to_vec(),
            })
        });
    let outcome = run_chain_for_strategy(&mut platform, 10, strategy, config, 15);
    let state = outcome.abci_app.platform.state.read().unwrap();
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


fn run_chain_update_identities_remove_keys() {
    let platform_version = PlatformVersion::latest();
    let strategy = Strategy {
        contracts_with_updates: vec![],
        operations: vec![Operation {
            op_type: OperationType::IdentityUpdate(IdentityUpdateOp::IdentityUpdateDisableKey(
                3,
            )),
            frequency: Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            },
        }],
        start_identities: vec![],
        identities_inserts: Frequency {
            times_per_block_range: 1..2,
            chance_per_block: None,
        },
        total_hpmns: 100,
        extra_normal_mns: 0,
        quorum_count: 24,
        upgrading_info: None,
        core_height_increase: Frequency {
            times_per_block_range: Default::default(),
            chance_per_block: None,
        },
        proposer_strategy: Default::default(),
        rotate_quorums: false,
        failure_testing: None,
        query_testing: None,
        // because we can add an identity and remove keys to it in the same block
        // the result would be different then expected
        verify_state_transition_results: false,
        signer: None,
    };
    let config = PlatformConfig {
        verify_sum_trees: true,
        quorum_size: 100,
        validator_set_quorum_rotation_block_count: 25,
        block_spacing_ms: 3000,
        testing_configs: PlatformTestConfig::default_with_no_block_signing(),
        ..Default::default()
    };
    let mut platform = TestPlatformBuilder::new()
        .with_config(config.clone())
        .build_with_mock_rpc();
    platform
        .core_rpc
        .expect_get_best_chain_lock()
        .returning(move || {
            Ok(CoreChainLock {
                core_block_height: 10,
                core_block_hash: [1; 32].to_vec(),
                signature: [2; 96].to_vec(),
            })
        });
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


fn run_chain_top_up_and_withdraw_from_identities() {
    let strategy = Strategy {
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
        start_identities: vec![],
        identities_inserts: Frequency {
            times_per_block_range: 1..2,
            chance_per_block: None,
        },
        total_hpmns: 100,
        extra_normal_mns: 0,
        quorum_count: 24,
        upgrading_info: None,
        core_height_increase: Frequency {
            times_per_block_range: Default::default(),
            chance_per_block: None,
        },
        proposer_strategy: Default::default(),
        rotate_quorums: false,
        failure_testing: None,
        query_testing: None,
        // because we can add an identity and withdraw from it in the same block
        // the result would be different then expected
        verify_state_transition_results: false,
        signer: None,
    };
    let config = PlatformConfig {
        verify_sum_trees: true,
        quorum_size: 100,
        validator_set_quorum_rotation_block_count: 25,
        block_spacing_ms: 3000,
        testing_configs: PlatformTestConfig::default_with_no_block_signing(),
        ..Default::default()
    };
    let mut platform = TestPlatformBuilder::new()
        .with_config(config.clone())
        .build_with_mock_rpc();
    platform
        .core_rpc
        .expect_get_best_chain_lock()
        .returning(move || {
            Ok(CoreChainLock {
                core_block_height: 10,
                core_block_hash: [1; 32].to_vec(),
                signature: [2; 96].to_vec(),
            })
        });
    let outcome = run_chain_for_strategy(&mut platform, 10, strategy, config, 15);

    assert_eq!(outcome.identities.len(), 10);
    assert_eq!(outcome.withdrawals.len(), 18);
}


fn run_chain_transfer_between_identities() {
    let strategy = Strategy {
        contracts_with_updates: vec![],
        operations: vec![Operation {
            op_type: OperationType::IdentityTransfer,
            frequency: Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
            },
        }],
        start_identities: vec![],
        identities_inserts: Frequency {
            times_per_block_range: 1..2,
            chance_per_block: None,
        },
        total_hpmns: 100,
        extra_normal_mns: 0,
        quorum_count: 24,
        upgrading_info: None,
        core_height_increase: Frequency {
            times_per_block_range: Default::default(),
            chance_per_block: None,
        },
        proposer_strategy: Default::default(),
        rotate_quorums: false,
        failure_testing: None,
        query_testing: None,
        verify_state_transition_results: true,
        signer: None,
    };

    let config = PlatformConfig {
        verify_sum_trees: true,
        quorum_size: 100,
        validator_set_quorum_rotation_block_count: 25,
        block_spacing_ms: 3000,
        testing_configs: PlatformTestConfig::default_with_no_block_signing(),
        ..Default::default()
    };

    let mut platform = TestPlatformBuilder::new()
        .with_config(config.clone())
        .build_with_mock_rpc();
    platform
        .core_rpc
        .expect_get_best_chain_lock()
        .returning(move || {
            Ok(CoreChainLock {
                core_block_height: 10,
                core_block_hash: [1; 32].to_vec(),
                signature: [2; 96].to_vec(),
            })
        });
    let outcome = run_chain_for_strategy(&mut platform, 10, strategy, config, 15);

    let balances = &outcome
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

    assert_eq!(outcome.identities.len(), 10);

    let len = outcome.identities.len();

    for identity in &outcome.identities[..len - 1] {
        let new_balance = balances[&identity.id().to_buffer()];
        // All identity balances decreased
        // as we transferred funds to the last identity
        assert_eq!(new_balance, 0);
    }

    let last_identity = &outcome.identities[len - 1];
    let last_identity_balance = balances[&last_identity.id().to_buffer()];
    // We transferred funds to the last identity, so we need to check that last identity balance was increased
    assert!(last_identity_balance > 100000000000u64);
}

