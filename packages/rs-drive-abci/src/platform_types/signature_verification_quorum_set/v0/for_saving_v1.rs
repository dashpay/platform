#[cfg(feature = "bls-signatures")]
use crate::platform_types::signature_verification_quorum_set::v0::for_saving_v0::PreviousPastQuorumsForSavingV0;
use crate::platform_types::signature_verification_quorum_set::v0::quorum_config_for_saving_v0::QuorumConfigForSavingV0;
use crate::platform_types::signature_verification_quorum_set::v0::quorum_set::PreviousPastQuorumsV0;
use crate::platform_types::signature_verification_quorum_set::{
    Quorums, SignatureVerificationQuorumSetForSaving, SignatureVerificationQuorumSetV0,
    ThresholdBlsPublicKey, VerificationQuorum,
};
use bincode::{Decode, Encode};
use dpp::bls_signatures::Bls12381G2Impl;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::QuorumHash;
use dpp::platform_value::Bytes32;

#[derive(Debug, Clone, Encode, Decode)]
pub struct SignatureVerificationQuorumSetForSavingV1 {
    config: QuorumConfigForSavingV0,
    current_quorums: Vec<QuorumForSavingV1>,
    #[cfg(feature = "bls-signatures")]
    previous_quorums: Option<PreviousPastQuorumsForSavingV0>,
}

impl From<SignatureVerificationQuorumSetForSavingV1> for SignatureVerificationQuorumSetForSaving {
    fn from(value: SignatureVerificationQuorumSetForSavingV1) -> Self {
        SignatureVerificationQuorumSetForSaving::V1(value)
    }
}

impl From<SignatureVerificationQuorumSetV0> for SignatureVerificationQuorumSetForSavingV1 {
    fn from(value: SignatureVerificationQuorumSetV0) -> Self {
        #[allow(unused_variables)]
        let SignatureVerificationQuorumSetV0 {
            config,
            current_quorums,
            previous,
        } = value;

        Self {
            config: config.into(),
            current_quorums: current_quorums.into(),
            #[cfg(feature = "bls-signatures")]
            previous_quorums: previous.map(|previous| previous.into()),
        }
    }
}

impl From<SignatureVerificationQuorumSetForSavingV1> for SignatureVerificationQuorumSetV0 {
    fn from(value: SignatureVerificationQuorumSetForSavingV1) -> Self {
        let SignatureVerificationQuorumSetForSavingV1 {
            config,
            current_quorums,
            #[cfg(feature = "bls-signatures")]
            previous_quorums,
        } = value;

        Self {
            config: config.into(),
            current_quorums: current_quorums.into(),
            #[cfg(feature = "bls-signatures")]
            previous: previous_quorums.map(|previous| previous.into()),
            #[cfg(not(feature = "bls-signatures"))]
            previous: None,
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
