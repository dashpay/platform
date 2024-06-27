use crate::version::fee::data_contract::FeeDataContractValidationVersion;
use crate::version::fee::hashing::FeeHashingVersion;
use crate::version::fee::processing::FeeProcessingVersion;
use crate::version::fee::signature::FeeSignatureVersion;
use crate::version::fee::state_transition_min_fees::StateTransitionMinFees;
use crate::version::fee::storage::FeeStorageVersion;

mod data_contract;
mod hashing;
mod processing;
pub mod signature;
pub mod state_transition_min_fees;
pub mod storage;
pub mod v1;

#[derive(Clone, Debug, Default)]
pub struct FeeVersion {
    pub storage: FeeStorageVersion,
    pub signature: FeeSignatureVersion,
    pub hashing: FeeHashingVersion,
    pub processing: FeeProcessingVersion,
    pub data_contract: FeeDataContractValidationVersion,
    pub state_transition_min_fees: StateTransitionMinFees,
}

impl PartialEq for FeeStorageVersion {
    fn eq(&self, other: &Self) -> bool {
        self.storage_disk_usage_credit_per_byte == other.storage_disk_usage_credit_per_byte
            && self.storage_processing_credit_per_byte == other.storage_processing_credit_per_byte
            && self.storage_load_credit_per_byte == other.storage_load_credit_per_byte
            && self.non_storage_load_credit_per_byte == other.non_storage_load_credit_per_byte
            && self.storage_seek_cost == other.storage_seek_cost
    }
}
