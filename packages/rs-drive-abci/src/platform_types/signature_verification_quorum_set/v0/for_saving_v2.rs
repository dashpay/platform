use crate::platform_types::signature_verification_quorum_set::v0::for_saving_v1::{
    PreviousPastQuorumsForSavingV1, QuorumForSavingV1,
};
use crate::platform_types::signature_verification_quorum_set::v0::quorum_config_for_saving_v0::QuorumConfigForSavingV0;
use crate::platform_types::signature_verification_quorum_set::{
    SignatureVerificationQuorumSetForSaving, SignatureVerificationQuorumSetV0,
};
use bincode::{Decode, Encode};

// We needed to introduce this V2 because the PreviousPastQuorumsForSavingV1 were using a QuorumForSavingV0
// This was an oversight in SignatureVerificationQuorumSetForSavingV1.
// This would cause the previous quorums to still use the old BLS sigs library for serialization/deserialization.

#[derive(Debug, Clone, Encode, Decode)]
pub struct SignatureVerificationQuorumSetForSavingV2 {
    config: QuorumConfigForSavingV0,
    current_quorums: Vec<QuorumForSavingV1>,
    previous_quorums: Option<PreviousPastQuorumsForSavingV1>,
}

impl From<SignatureVerificationQuorumSetForSavingV2> for SignatureVerificationQuorumSetForSaving {
    fn from(value: SignatureVerificationQuorumSetForSavingV2) -> Self {
        SignatureVerificationQuorumSetForSaving::V2(value)
    }
}

impl From<SignatureVerificationQuorumSetV0> for SignatureVerificationQuorumSetForSavingV2 {
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

impl From<SignatureVerificationQuorumSetForSavingV2> for SignatureVerificationQuorumSetV0 {
    fn from(value: SignatureVerificationQuorumSetForSavingV2) -> Self {
        let SignatureVerificationQuorumSetForSavingV2 {
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
