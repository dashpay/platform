use crate::execution::engine::consensus_params_update::v1::consensus_params_update_v1;
use crate::platform_types::epoch_info::EpochInfo;
use dpp::dashcore::Network;
use dpp::version::v15::PROTOCOL_VERSION_15;
use dpp::version::PlatformVersion;
use tenderdash_abci::proto::google::protobuf::Duration;
use tenderdash_abci::proto::types::{ConsensusParams, EvidenceParams};

/// Maximum evidence age in blocks, applied when the network crosses to protocol
/// version 15 (state sync). Value proposed in issue #2512 for nodes that bootstrap
/// from snapshots and do not hold full history.
///
/// REVIEW BEFORE RELEASE: at ~6s blocks, 15_000 blocks is roughly one day, while
/// [`V15_EVIDENCE_MAX_AGE_DURATION_SECONDS`] below is 20 days. Evidence expires when
/// EITHER bound is exceeded, so the effective window is the smaller (~1 day) — the two
/// values from #2512 look inconsistent and need to be confirmed before this ships.
const V15_EVIDENCE_MAX_AGE_NUM_BLOCKS: i64 = 15_000;

/// Maximum evidence age in time: 20 days, per issue #2512. See the review note on
/// [`V15_EVIDENCE_MAX_AGE_NUM_BLOCKS`].
const V15_EVIDENCE_MAX_AGE_DURATION_SECONDS: i64 = 20 * 24 * 60 * 60;

/// Maximum total evidence per block in bytes. Tenderdash's default (1 MiB); #2512 does
/// not change it, but the whole evidence section must be populated when it is emitted.
const V15_EVIDENCE_MAX_BYTES: i64 = 1_048_576;

/// Same as v1, but the first block of protocol version 15 additionally emits evidence
/// params sized for a network whose nodes may have bootstrapped via state sync
/// (issue #2512).
#[inline(always)]
pub(super) fn consensus_params_update_v2(
    network: Network,
    original_platform_version: &PlatformVersion,
    new_platform_version: &PlatformVersion,
    epoch_info: &EpochInfo,
) -> Option<ConsensusParams> {
    let mut consensus_params = consensus_params_update_v1(
        network,
        original_platform_version,
        new_platform_version,
        epoch_info,
    )?;

    // Crossing to v15 implies a protocol version change, so v1 always emits params on
    // the activation block and we only need to attach the evidence section.
    let is_crossing_to_v15 = original_platform_version.protocol_version < PROTOCOL_VERSION_15
        && new_platform_version.protocol_version >= PROTOCOL_VERSION_15;
    if is_crossing_to_v15 {
        consensus_params.evidence = Some(EvidenceParams {
            max_age_num_blocks: V15_EVIDENCE_MAX_AGE_NUM_BLOCKS,
            max_age_duration: Some(Duration {
                seconds: V15_EVIDENCE_MAX_AGE_DURATION_SECONDS,
                nanos: 0,
            }),
            max_bytes: V15_EVIDENCE_MAX_BYTES,
        });
    }

    Some(consensus_params)
}
