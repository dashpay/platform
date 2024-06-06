use crate::platform_types::core_quorum_set::v0::quorum_set::{PreviousQuorumsV0, QuorumConfig};
use crate::platform_types::core_quorum_set::{
    CoreQuorumSetForSaving, CoreQuorumSetV0, Quorums, ThresholdBlsPublicKey, VerificationQuorum,
};
use dashcore_rpc::dashcore::hashes::Hash;
use dashcore_rpc::dashcore::QuorumHash;
use dashcore_rpc::json::QuorumType;
use dpp::identity::state_transition::asset_lock_proof::Encode;
use dpp::platform_serialization::de::Decode;
use dpp::platform_value::Bytes32;

#[derive(Debug, Clone, Encode, Decode)]
pub struct CoreQuorumSetForSavingV0 {
    config: QuorumConfigForSavingV0,

    current_quorums: Vec<QuorumForSavingV0>,

    previous_quorums: Option<PreviousQuorumsForSavingV0>,
}

impl From<CoreQuorumSetForSavingV0> for CoreQuorumSetForSaving {
    fn from(value: CoreQuorumSetForSavingV0) -> Self {
        CoreQuorumSetForSaving::V0(value)
    }
}

impl From<CoreQuorumSetV0> for CoreQuorumSetForSavingV0 {
    fn from(value: CoreQuorumSetV0) -> Self {
        let CoreQuorumSetV0 {
            config,
            current_quorums,
            previous,
        } = value;

        Self {
            config: config.into(),
            current_quorums: current_quorums.into(),
            previous_quorums: previous.map(|previous| previous.into()),
        }
    }
}

impl From<CoreQuorumSetForSavingV0> for CoreQuorumSetV0 {
    fn from(value: CoreQuorumSetForSavingV0) -> Self {
        let CoreQuorumSetForSavingV0 {
            config,
            current_quorums,
            previous_quorums,
        } = value;

        Self {
            config: config.into(),
            current_quorums: current_quorums.into(),
            previous: previous_quorums.map(|previous| previous.into()),
        }
    }
}

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

#[derive(Debug, Clone, Encode, Decode)]
pub struct PreviousQuorumsForSavingV0 {
    quorums: Vec<QuorumForSavingV0>,
    active_core_height: u32,
    updated_at_core_height: u32,
    previous_change_height: Option<u32>,
}

impl From<PreviousQuorumsV0> for PreviousQuorumsForSavingV0 {
    fn from(value: PreviousQuorumsV0) -> Self {
        let PreviousQuorumsV0 {
            quorums,
            active_core_height,
            updated_at_core_height,
            previous_change_height,
        } = value;

        Self {
            quorums: quorums.into(),
            active_core_height,
            updated_at_core_height,
            previous_change_height,
        }
    }
}

impl From<PreviousQuorumsForSavingV0> for PreviousQuorumsV0 {
    fn from(value: PreviousQuorumsForSavingV0) -> Self {
        let PreviousQuorumsForSavingV0 {
            quorums,
            active_core_height,
            updated_at_core_height,
            previous_change_height,
        } = value;

        Self {
            quorums: quorums.into(),
            active_core_height,
            updated_at_core_height,
            previous_change_height,
        }
    }
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct QuorumForSavingV0 {
    hash: Bytes32,
    #[bincode(with_serde)]
    public_key: ThresholdBlsPublicKey,
    index: Option<u32>,
}

impl From<Vec<QuorumForSavingV0>> for Quorums<VerificationQuorum> {
    fn from(value: Vec<QuorumForSavingV0>) -> Self {
        Quorums::from_iter(value.into_iter().map(|quorum| {
            (
                QuorumHash::from_byte_array(quorum.hash.to_buffer()),
                VerificationQuorum {
                    public_key: quorum.public_key,
                    index: quorum.index,
                },
            )
        }))
    }
}

impl Into<Vec<QuorumForSavingV0>> for Quorums<VerificationQuorum> {
    fn into(self) -> Vec<QuorumForSavingV0> {
        self.into_iter()
            .map(|(hash, quorum)| QuorumForSavingV0 {
                hash: Bytes32::from(hash.as_byte_array()),
                public_key: quorum.public_key,
                index: quorum.index,
            })
            .collect()
    }
}
