use dpp::version::PlatformVersion;
use tenderdash_abci::proto::types::{ConsensusParams, VersionParams};
#[inline(always)]
pub(super) fn consensus_params_update_v0(
    original_platform_version: &PlatformVersion,
    new_platform_version: &PlatformVersion,
) -> Option<ConsensusParams> {
    if original_platform_version
        .consensus
        .tenderdash_consensus_version
        == new_platform_version.consensus.tenderdash_consensus_version
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
