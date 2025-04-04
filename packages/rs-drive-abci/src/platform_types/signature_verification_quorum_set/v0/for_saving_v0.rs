use crate::platform_types::signature_verification_quorum_set::v0::quorum_config_for_saving_v0::QuorumConfigForSavingV0;
use crate::platform_types::signature_verification_quorum_set::v0::quorum_set::PreviousPastQuorumsV0;
use crate::platform_types::signature_verification_quorum_set::{
    Quorums, SignatureVerificationQuorumSetForSaving, SignatureVerificationQuorumSetV0,
    VerificationQuorum,
};
use bincode::{Decode, Encode};
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::QuorumHash;
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
    #[cfg(feature = "bls-signatures")]
    #[bincode(with_serde)]
    public_key: bls_signatures::PublicKey,
    index: Option<u32>,
}

impl From<Vec<QuorumForSavingV0>> for Quorums<VerificationQuorum> {
    fn from(value: Vec<QuorumForSavingV0>) -> Self {
        Quorums::from_iter(value.into_iter().map(|quorum| {
            (
                QuorumHash::from_byte_array(quorum.hash.to_buffer()),
                VerificationQuorum {
                    #[cfg(feature = "bls-signatures")]
                    public_key: dpp::bls_signatures::PublicKey::try_from(
                        quorum.public_key.to_bytes().as_slice(),
                    )
                    .expect("expected to convert between BLS key libraries (from chia)"),
                    #[cfg(not(feature = "bls-signatures"))]
                    public_key: Default::default(),
                    index: quorum.index,
                },
            )
        }))
    }
}

impl From<Quorums<VerificationQuorum>> for Vec<QuorumForSavingV0> {
    fn from(quorums: Quorums<VerificationQuorum>) -> Self {
        quorums
            .into_iter()
            .map(|(hash, quorum)| QuorumForSavingV0 {
                hash: Bytes32::from(hash.as_byte_array()),
                #[cfg(feature = "bls-signatures")]
                public_key: bls_signatures::PublicKey::from_bytes(
                    &quorum.public_key.0.to_compressed(),
                )
                .expect("expected to convert between BLS key libraries (to chia)"),
                index: quorum.index,
            })
            .collect()
    }
}
