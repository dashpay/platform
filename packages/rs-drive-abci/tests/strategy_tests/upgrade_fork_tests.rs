#[cfg(test)]
mod tests {
    use super::*;
    use crate::{run_chain_for_strategy, Frequency, Strategy, UpgradingInfo};
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
            quorum_switch_block_count: 25,
        };
        run_chain_for_strategy(1000, 3000, strategy, config, 15);
    }
}
