use crate::platform_types::signature_verification_quorum_set::v0::for_saving::{
    PreviousPastQuorumsForSavingV0, QuorumConfigForSavingV0,
};
use crate::platform_types::signature_verification_quorum_set::{
    Quorums, SignatureVerificationQuorumSetForSaving, SignatureVerificationQuorumSetV0,
    ThresholdBlsPublicKey, VerificationQuorum,
};
use dashcore_rpc::dashcore::hashes::Hash;
use dashcore_rpc::dashcore::QuorumHash;
use dpp::bls_signatures::Bls12381G2Impl;
use dpp::identity::state_transition::asset_lock_proof::Encode;
use dpp::platform_serialization::de::Decode;
use dpp::platform_value::Bytes32;

#[derive(Debug, Clone, Encode, Decode)]
pub struct SignatureVerificationQuorumSetForSavingV1 {
    config: QuorumConfigForSavingV0,
    current_quorums: Vec<QuorumForSavingV1>,
    previous_quorums: Option<PreviousPastQuorumsForSavingV0>,
}

impl From<SignatureVerificationQuorumSetForSavingV1> for SignatureVerificationQuorumSetForSaving {
    fn from(value: SignatureVerificationQuorumSetForSavingV1) -> Self {
        SignatureVerificationQuorumSetForSaving::V1(value)
    }
}

impl From<SignatureVerificationQuorumSetV0> for SignatureVerificationQuorumSetForSavingV1 {
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

impl From<SignatureVerificationQuorumSetForSavingV1> for SignatureVerificationQuorumSetV0 {
    fn from(value: SignatureVerificationQuorumSetForSavingV1) -> Self {
        let SignatureVerificationQuorumSetForSavingV1 {
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
pub struct QuorumForSavingV1 {
    hash: Bytes32,
    #[bincode(with_serde)]
    public_key: ThresholdBlsPublicKey<Bls12381G2Impl>,
    index: Option<u32>,
}

impl From<Vec<QuorumForSavingV1>> for Quorums<VerificationQuorum> {
    fn from(value: Vec<QuorumForSavingV1>) -> Self {
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
impl Into<Vec<QuorumForSavingV1>> for Quorums<VerificationQuorum> {
    fn into(self) -> Vec<QuorumForSavingV1> {
        self.into_iter()
            .map(|(hash, quorum)| QuorumForSavingV1 {
                hash: Bytes32::from(hash.as_byte_array()),
                public_key: quorum.public_key,
                index: quorum.index,
            })
            .collect()
    }
}
