use crate::platform_types::epoch_info::v0::EpochInfoV0Methods;
use crate::platform_types::epoch_info::EpochInfo;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use tenderdash_abci::proto::types::{ConsensusParams, VersionParams};

#[inline(always)]
pub(super) fn consensus_params_update_v1(
    network: Network,
    original_platform_version: &PlatformVersion,
    new_platform_version: &PlatformVersion,
    epoch_info: &EpochInfo,
) -> Option<ConsensusParams> {
    // These are emergency consensus updates
    match network {
        Network::Dash => {
            if epoch_info.is_first_block_of_epoch(3) {
                return Some(ConsensusParams {
                    block: None,
                    evidence: None,
                    validator: None,
                    version: Some(VersionParams {
                        app_version: new_platform_version.protocol_version as u64,
                        consensus_version: 1,
                    }),
                    synchrony: None,
                    timeout: None,
                    abci: None,
                });
            }
        }
        Network::Testnet => {
            if epoch_info.is_first_block_of_epoch(1480) {
                return Some(ConsensusParams {
                    block: None,
                    evidence: None,
                    validator: None,
                    version: Some(VersionParams {
                        app_version: new_platform_version.protocol_version as u64,
                        consensus_version: 1,
                    }),
                    synchrony: None,
                    timeout: None,
                    abci: None,
                });
            }
        }
        _ => {}
    }

    // Update versions if any of them changed
    if original_platform_version
        .consensus
        .tenderdash_consensus_version
        == new_platform_version.consensus.tenderdash_consensus_version
        && original_platform_version.protocol_version == new_platform_version.protocol_version
    {
        None
    } else {
        Some(ConsensusParams {
            block: None,
            evidence: None,
            validator: None,
            version: Some(VersionParams {
                app_version: new_platform_version.protocol_version as u64,
                consensus_version: new_platform_version.consensus.tenderdash_consensus_version
                    as i32,
            }),
            synchrony: None,
            timeout: None,
            abci: None,
        })
    }
}
