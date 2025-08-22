use crate::version::fee::v1::FEE_VERSION1;
use bincode::{Decode, Encode};

pub mod v1;

#[derive(Clone, Debug, Encode, Decode, Default, PartialEq, Eq)]
pub struct FeeProcessingVersion {
    pub fetch_identity_balance_processing_cost: u64,
    pub fetch_identity_revision_processing_cost: u64,
    pub fetch_identity_balance_and_revision_processing_cost: u64,
    pub fetch_identity_cost_per_look_up_key_by_id: u64,
    pub fetch_identity_token_balance_processing_cost: u64,
    pub fetch_prefunded_specialized_balance_processing_cost: u64,
    pub fetch_single_identity_key_processing_cost: u64,
    pub validate_key_structure: u64,
    pub perform_network_threshold_signing: u64,
}

// This is type only meant for deserialization because of an issue
// The issue was that the platform state was stored with FeeVersions in it before version 1.4
// When we would add new fields we would be unable to deserialize
// This FeeProcessingVersionFieldsBeforeVersion4 is how things were before version 1.4 was released
#[derive(Clone, Debug, Encode, Decode, Default, PartialEq, Eq)]
pub struct FeeProcessingVersionFieldsBeforeVersion1Point4 {
    pub fetch_identity_balance_processing_cost: u64,
    pub fetch_identity_revision_processing_cost: u64,
    pub fetch_identity_balance_and_revision_processing_cost: u64,
    pub fetch_identity_cost_per_look_up_key_by_id: u64,
    pub fetch_prefunded_specialized_balance_processing_cost: u64,
    pub fetch_single_identity_key_processing_cost: u64,
    pub validate_key_structure: u64,
}

impl From<FeeProcessingVersionFieldsBeforeVersion1Point4> for FeeProcessingVersion {
    fn from(old: FeeProcessingVersionFieldsBeforeVersion1Point4) -> Self {
        FeeProcessingVersion {
            fetch_identity_balance_processing_cost: old.fetch_identity_balance_processing_cost,
            fetch_identity_revision_processing_cost: old.fetch_identity_revision_processing_cost,
            fetch_identity_balance_and_revision_processing_cost: old
                .fetch_identity_balance_and_revision_processing_cost,
            fetch_identity_cost_per_look_up_key_by_id: old
                .fetch_identity_cost_per_look_up_key_by_id,
            fetch_identity_token_balance_processing_cost: FEE_VERSION1
                .processing
                .fetch_identity_token_balance_processing_cost,
            fetch_prefunded_specialized_balance_processing_cost: old
                .fetch_prefunded_specialized_balance_processing_cost,
            fetch_single_identity_key_processing_cost: old
                .fetch_single_identity_key_processing_cost,
            validate_key_structure: old.validate_key_structure,
            perform_network_threshold_signing: FEE_VERSION1
                .processing
                .perform_network_threshold_signing,
        }
    }
}
