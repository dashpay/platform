use crate::fee::default_costs::EpochIndexFeeVersionsForStorage;
use crate::util::deserializer::ProtocolVersion;
use bincode::{Decode, Encode};
use platform_value::Bytes32;

/// Reduced Platform State for Saving V0
/// This minimal version of Platform state is written in GroveDB under the root hash.
/// This allows a freshly new synced node to reconstruct Platform state.
#[derive(Clone, Debug, Encode, Decode)]
pub struct ReducedPlatformStateForSavingV0 {
    /// Current Version
    pub current_protocol_version_in_consensus: ProtocolVersion,
    /// upcoming protocol version
    pub next_epoch_protocol_version: ProtocolVersion,
    /// current quorum
    pub current_validator_set_quorum_hash: Bytes32,
    /// next quorum
    pub next_validator_set_quorum_hash: Option<Bytes32>,
    /// previous Fee Versions
    pub previous_fee_versions: EpochIndexFeeVersionsForStorage,
}
