use crate::platform_types::signature_verification_quorum_set::QuorumConfig;
use bincode::Encode;
use dashcore_rpc::dashcore_rpc_json::QuorumType;
use dpp::platform_serialization::de::Decode;

#[derive(Debug, Clone, Encode, Decode)]
pub struct QuorumConfigForSavingV0 {
    quorum_type: QuorumType,
    active_signers: u16,
    rotation: bool,
    window: u32,
}

impl From<QuorumConfig> for QuorumConfigForSavingV0 {
    fn from(config: QuorumConfig) -> Self {
        Self {
            quorum_type: config.quorum_type,
            active_signers: config.active_signers,
            rotation: config.rotation,
            window: config.window,
        }
    }
}

impl From<QuorumConfigForSavingV0> for QuorumConfig {
    fn from(config: QuorumConfigForSavingV0) -> Self {
        Self {
            quorum_type: config.quorum_type,
            active_signers: config.active_signers,
            rotation: config.rotation,
            window: config.window,
        }
    }
}
