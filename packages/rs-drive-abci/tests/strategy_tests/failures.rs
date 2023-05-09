#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::{continue_chain_for_strategy, run_chain_for_strategy};
    use crate::frequency::Frequency;
    use crate::operations::DocumentAction::DocumentActionReplace;
    use crate::operations::{
        DocumentAction, DocumentOp, IdentityUpdateOp, Operation, OperationType,
    };
    use crate::strategy::{FailureStrategy, MasternodeListChangesStrategy, Strategy};
    use dashcore_rpc::dashcore::hashes::Hash;
    use dashcore_rpc::dashcore::BlockHash;
    use dashcore_rpc::dashcore_rpc_json::ExtendedQuorumDetails;
    use dpp::data_contract::extra::common::json_document_to_contract;
    use drive_abci::config::{PlatformConfig, PlatformTestConfig};
    use drive_abci::rpc::core::QuorumListExtendedInfo;
    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use tenderdash_abci::proto::types::CoreChainLock;

    #[test]
    fn run_chain_block_failure_on_genesis_block_correctly_fixes_itself() {
        let mut strategy = Strategy {
            contracts_with_updates: vec![],
            operations: vec![],
            identities_inserts: Frequency {
                times_per_block_range: Default::default(),
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
            failure_testing: Some(FailureStrategy {
                deterministic_start_seed: Some(99),
                dont_finalize_block: true,
            }),
            query_testing: None,
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

        let mut core_block_heights = vec![10, 11];

        platform
            .core_rpc
            .expect_get_best_chain_lock()
            .returning(move || {
                let core_block_height = if core_block_heights.len() == 1 {
                    *core_block_heights.get(0).unwrap()
                } else {
                    core_block_heights.remove(0)
                };
                Ok(CoreChainLock {
                    core_block_height,
                    core_block_hash: [1; 32].to_vec(),
                    signature: [2; 96].to_vec(),
                })
            });
        run_chain_for_strategy(&mut platform, 1, strategy.clone(), config.clone(), 15);

        //platform block didn't complete, so it should get another init chain

        strategy.failure_testing = None;

        run_chain_for_strategy(&mut platform, 15, strategy, config, 15);
    }
}
