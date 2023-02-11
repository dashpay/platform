#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        run_chain_for_strategy, ChainExecutionOutcome, Frequency, Strategy, UpgradingInfo,
    };
    use drive::dpp::data_contract::extra::common::json_document_to_cbor;
    use drive::dpp::data_contract::DriveContractExt;
    use drive_abci::config::PlatformConfig;
    use std::collections::HashMap;

    #[test]
    fn run_chain_version_upgrade() {
        let strategy = Strategy {
            contracts: vec![],
            operations: vec![],
            identities_inserts: Frequency {
                times_per_block_range: Default::default(),
                chance_per_block: None,
            },
            total_hpmns: 460,
            upgrading_info: Some(UpgradingInfo {
                current_protocol_version: 1,
                proposed_protocol_versions_with_weight: vec![(2, 1)],
                upgrade_three_quarters_life: 0.75,
            }),
        };
        let config = PlatformConfig {
            drive_config: Default::default(),
            verify_sum_trees: true,
            quorum_size: 100,
            quorum_rotation_block_count: 125,
        };
        let hour_in_ms = 1000 * 60 * 60;
        let ChainExecutionOutcome {
            platform,
            masternode_identity_balances,
            identities,
            end_epoch_index,
        } = run_chain_for_strategy(2000, hour_in_ms, strategy, config, 15);
        let mut drive_cache = platform.drive.cache.borrow_mut();
        let counter = drive_cache
            .versions_counter
            .as_ref()
            .expect("expected a version counter");
        platform
            .drive
            .fetch_versions_with_counter(None)
            .expect("expected to get versions");
        assert_eq!(counter.get(&1), Some(&0)); //all nodes upgraded
        assert_eq!(counter.get(&2), Some(&448)); //most nodes were hit (12 were not)
        assert_eq!(platform.state.last_block_info.unwrap().epoch.index, 4);
        assert_eq!(platform.state.current_protocol_version_in_consensus, 2);
    }
}
