use crate::platform_types::signature_verification_quorum_set::v0::quorum_set::PreviousPastQuorumsV0;
use crate::platform_types::signature_verification_quorum_set::{
    Quorums, ThresholdBlsPublicKey, VerificationQuorum,
};
use bincode::{Decode, Encode};

use dpp::dashcore::hashes::Hash;
use dpp::dashcore::QuorumHash;
use dpp::platform_value::Bytes32;

#[derive(Debug, Clone, Encode, Decode)]
pub struct QuorumForSavingV1 {
    hash: Bytes32,
    #[bincode(with_serde)]
    public_key: ThresholdBlsPublicKey,
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
impl From<Quorums<VerificationQuorum>> for Vec<QuorumForSavingV1> {
    fn from(quorums: Quorums<VerificationQuorum>) -> Self {
        quorums
            .into_iter()
            .map(|(hash, quorum)| QuorumForSavingV1 {
                hash: Bytes32::from(hash.as_byte_array()),
                public_key: quorum.public_key,
                index: quorum.index,
            })
            .collect()
    }
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct PreviousPastQuorumsForSavingV1 {
    quorums: Vec<QuorumForSavingV1>,
    last_active_core_height: u32,
    updated_at_core_height: u32,
    previous_change_height: Option<u32>,
}

impl From<PreviousPastQuorumsV0> for PreviousPastQuorumsForSavingV1 {
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

impl From<PreviousPastQuorumsForSavingV1> for PreviousPastQuorumsV0 {
    fn from(value: PreviousPastQuorumsForSavingV1) -> Self {
        let PreviousPastQuorumsForSavingV1 {
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
