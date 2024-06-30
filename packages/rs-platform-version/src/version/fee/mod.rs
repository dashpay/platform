use crate::version::fee::data_contract::FeeDataContractValidationVersion;
use crate::version::fee::hashing::FeeHashingVersion;
use crate::version::fee::processing::FeeProcessingVersion;
use crate::version::fee::signature::FeeSignatureVersion;
use crate::version::fee::state_transition_min_fees::StateTransitionMinFees;
use crate::version::fee::storage::FeeStorageVersion;
use sha2::{Digest, Sha256};

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

impl PartialEq for FeeVersion {
    fn eq(&self, other: &Self) -> bool {
        self.to_hash() == other.to_hash()
    }
}

impl FeeVersion {
    fn to_hash(&self) -> u64 {
        let mut hasher = Sha256::new();
        Digest::update(&mut hasher, &self.storage.to_hash().to_be_bytes());
        Digest::update(&mut hasher, &self.signature.to_hash().to_be_bytes());
        Digest::update(&mut hasher, &self.hashing.to_hash().to_be_bytes());
        Digest::update(&mut hasher, &self.processing.to_hash().to_be_bytes());
        Digest::update(&mut hasher, &self.data_contract.to_hash().to_be_bytes());
        Digest::update(
            &mut hasher,
            &self.state_transition_min_fees.to_hash().to_be_bytes(),
        );

        let result = hasher.finalize();
        // Use the first 8 bytes of the hash as the u64 representation
        let hash_bytes: [u8; 8] = result[0..8].try_into().unwrap();
        u64::from_be_bytes(hash_bytes)
    }
}
