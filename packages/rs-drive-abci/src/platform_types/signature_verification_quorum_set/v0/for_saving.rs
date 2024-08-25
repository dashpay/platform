use crate::platform_types::signature_verification_quorum_set::v0::quorum_set::{
    PreviousPastQuorumsV0, QuorumConfig,
};
use crate::platform_types::signature_verification_quorum_set::{
    Quorums, SignatureVerificationQuorumSetForSaving, SignatureVerificationQuorumSetV0,
    ThresholdBlsPublicKey, VerificationQuorum,
};
use dashcore_rpc::dashcore::hashes::Hash;
use dashcore_rpc::dashcore::QuorumHash;
use dashcore_rpc::json::QuorumType;
use dpp::identity::state_transition::asset_lock_proof::Encode;
use dpp::platform_serialization::de::Decode;
use dpp::platform_value::Bytes32;

#[derive(Debug, Clone, Encode, Decode)]
pub struct SignatureVerificationQuorumSetForSavingV0 {
    config: QuorumConfigForSavingV0,
    current_quorums: Vec<QuorumForSavingV0>,
    previous_quorums: Option<PreviousPastQuorumsForSavingV0>,
}

impl From<SignatureVerificationQuorumSetForSavingV0> for SignatureVerificationQuorumSetForSaving {
    fn from(value: SignatureVerificationQuorumSetForSavingV0) -> Self {
        SignatureVerificationQuorumSetForSaving::V0(value)
    }
}

impl From<SignatureVerificationQuorumSetV0> for SignatureVerificationQuorumSetForSavingV0 {
    fn from(value: SignatureVerificationQuorumSetV0) -> Self {
        let SignatureVerificationQuorumSetV0 {
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

impl From<SignatureVerificationQuorumSetForSavingV0> for SignatureVerificationQuorumSetV0 {
    fn from(value: SignatureVerificationQuorumSetForSavingV0) -> Self {
        let SignatureVerificationQuorumSetForSavingV0 {
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
pub struct PreviousPastQuorumsForSavingV0 {
    quorums: Vec<QuorumForSavingV0>,
    last_active_core_height: u32,
    updated_at_core_height: u32,
    previous_change_height: Option<u32>,
}

impl From<PreviousPastQuorumsV0> for PreviousPastQuorumsForSavingV0 {
    fn from(value: PreviousPastQuorumsV0) -> Self {
        let PreviousPastQuorumsV0 {
            quorums,
            last_active_core_height,
            updated_at_core_height,
            previous_change_height,
        } = value;

        Self {
            quorums: quorums.into(),
            last_active_core_height,
            updated_at_core_height,
            previous_change_height,
        }
    }
}

impl From<PreviousPastQuorumsForSavingV0> for PreviousPastQuorumsV0 {
    fn from(value: PreviousPastQuorumsForSavingV0) -> Self {
        let PreviousPastQuorumsForSavingV0 {
            quorums,
            last_active_core_height: active_core_height,
            updated_at_core_height,
            previous_change_height,
        } = value;

        Self {
            quorums: quorums.into(),
            last_active_core_height: active_core_height,
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

#[allow(clippy::from_over_into)]
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
