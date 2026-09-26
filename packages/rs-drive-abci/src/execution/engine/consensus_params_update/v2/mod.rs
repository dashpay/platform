use crate::platform_types::epoch_info::v0::EpochInfoV0Methods;
use crate::platform_types::epoch_info::EpochInfo;
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use tenderdash_abci::proto::types::{BlockParams, ConsensusParams, VersionParams};

/// Consensus parameter update, version 2: the version parameters v1 pushes, plus the block
/// parameters when the new protocol version sets them and the previous one did not carry the
/// same values.
///
/// The Tenderdash block byte cap is a consensus parameter, so raising it for the contract-code
/// envelopes has to happen at the same height on every validator; returning it from
/// `prepare_proposal` and `process_proposal` at the protocol boundary (and from `init_chain`
/// for a fresh network) is the mechanism. Tenderdash copies both `max_bytes` and `max_gas`
/// whenever a `Block` update is present, so the gas cap travels with the byte cap.
///
/// The emergency updates of v1 are unchanged and keep their priority; a block parameter push
/// never coincides with them because they fire at fixed epochs on the two live networks.
#[inline(always)]
pub(super) fn consensus_params_update_v2(
    network: Network,
    original_platform_version: &PlatformVersion,
    new_platform_version: &PlatformVersion,
    epoch_info: &EpochInfo,
) -> Option<ConsensusParams> {
    // These are emergency consensus updates
    match network {
        Network::Mainnet => {
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
        return None;
    }

    Some(ConsensusParams {
        block: block_params_update(original_platform_version, new_platform_version),
        evidence: None,
        validator: None,
        version: Some(VersionParams {
            app_version: new_platform_version.protocol_version as u64,
            consensus_version: new_platform_version.consensus.tenderdash_consensus_version as i32,
        }),
        synchrony: None,
        timeout: None,
        abci: None,
    })
}

/// The block parameters to push: `Some` only when the new protocol version sets both and the
/// previous version did not already carry the same pair. A version that sets neither keeps
/// whatever Tenderdash holds (the genesis value, or an earlier push).
fn block_params_update(
    original_platform_version: &PlatformVersion,
    new_platform_version: &PlatformVersion,
) -> Option<BlockParams> {
    let new_consensus = &new_platform_version.consensus;
    let (Some(max_bytes), Some(max_gas)) =
        (new_consensus.block_max_bytes, new_consensus.block_max_gas)
    else {
        return None;
    };
    let original_consensus = &original_platform_version.consensus;
    if original_consensus.block_max_bytes == Some(max_bytes)
        && original_consensus.block_max_gas == Some(max_gas)
    {
        return None;
    }
    // `BlockParams.max_bytes` is an `i64` in the proto; the table value is bounded by
    // Tenderdash's own 100 MB hard cap, far below `i64::MAX`, so a value that does not fit is
    // a table error and the push is skipped rather than truncated.
    let max_bytes = i64::try_from(max_bytes).ok()?;
    Some(BlockParams { max_bytes, max_gas })
}
