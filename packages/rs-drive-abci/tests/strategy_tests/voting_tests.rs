use crate::execution::run_chain_for_strategy;
use crate::frequency::Frequency;
use crate::operations::{DocumentAction, DocumentOp, Operation, OperationType};
use crate::query::QueryStrategy;
use crate::strategy::Strategy;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::tests::fixtures::get_dpns_data_contract_fixture;
use drive_abci::config::{PlatformConfig, PlatformTestConfig};
use drive_abci::test::helpers::setup::TestPlatformBuilder;
use tenderdash_abci::proto::types::CoreChainLock;

#[test]
fn run_chain_one_username_voting() {
    let dpns_contract = get_dpns_data_contract_fixture(None, 1);

    let contract = dpns_contract.data_contract();

    let document_insertion_op = DocumentOp {
        contract: contract.clone(),
        action: DocumentAction::DocumentActionInsert,
        document_type: contract
            .document_type_for_name("contactRequest")
            .expect("expected a profile document type")
            .to_owned_document_type(),
    };

    let strategy = Strategy {
        contracts_with_updates: vec![(dpns_contract, None)],
        operations: vec![Operation {
            op_type: OperationType::Document(document_insertion_op),
            frequency: Frequency {
                times_per_block_range: 1..2,
                chance_per_block: None,
                only_once: true,
            },
        }],
        identities_inserts: Frequency {
            times_per_block_range: 1..2,
            chance_per_block: None,
            only_once: false,
        },
        total_hpmns: 100,
        extra_normal_mns: 200,
        quorum_count: 24,
        upgrading_info: None,
        core_height_increase: Frequency {
            times_per_block_range: Default::default(),
            chance_per_block: None,
            only_once: false,
        },
        proposer_strategy: Default::default(),
        rotate_quorums: false,
        failure_testing: None,
        query_testing: Some(QueryStrategy {
            query_identities_by_public_key_hashes: Frequency {
                times_per_block_range: 1..5,
                chance_per_block: None,
                only_once: false,
            },
        }),
        verify_state_transition_results: true,
    };

    let config = PlatformConfig {
        verify_sum_trees: true,
        quorum_size: 100,
        validator_set_quorum_rotation_block_count: 25,
        block_spacing_ms: 3000,
        testing_configs: PlatformTestConfig::default(),
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

    let outcome = run_chain_for_strategy(&mut platform, 100, strategy, config, 15);

    assert_eq!(outcome.identities.len(), 100);
}
