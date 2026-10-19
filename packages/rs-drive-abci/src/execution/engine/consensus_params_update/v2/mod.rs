use crate::platform_types::epoch_info::v0::EpochInfoV0Methods;
use crate::platform_types::epoch_info::EpochInfo;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use tenderdash_abci::proto::google::protobuf::Duration;
use tenderdash_abci::proto::types::{ConsensusParams, EvidenceParams, VersionParams};

#[inline(always)]
pub(super) fn consensus_params_update_v2(
    network: Network,
    original_platform_version: &PlatformVersion,
    epoch_info: &EpochInfo,
    new_platform_version: &PlatformVersion,
) -> Option<ConsensusParams> {
    let evidence = if original_platform_version.consensus.evidence
        != new_platform_version.consensus.evidence
    {
        new_platform_version
            .consensus
            .evidence
            .map(|policy| EvidenceParams {
                max_age_num_blocks: policy.max_age_num_blocks,
                max_age_duration: Some(Duration {
                    seconds: policy.max_age_duration_seconds,
                    nanos: 0,
                }),
                max_bytes: policy.max_bytes,
            })
    } else {
        None
    };

    // These are emergency consensus updates
    match network {
        Network::Mainnet => {
            if epoch_info.is_first_block_of_epoch(3) {
                return Some(ConsensusParams {
                    block: None,
                    evidence,
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
        Network::Testnet if epoch_info.is_first_block_of_epoch(1480) => {
            return Some(ConsensusParams {
                block: None,
                evidence,
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
        _ => {}
    }

    // Update versions or the evidence policy when they change
    if original_platform_version
        .consensus
        .tenderdash_consensus_version
        == new_platform_version.consensus.tenderdash_consensus_version
        && original_platform_version.protocol_version == new_platform_version.protocol_version
        && evidence.is_none()
    {
        None
    } else {
        Some(ConsensusParams {
            block: None,
            evidence,
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
