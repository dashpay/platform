use crate::version::fee::data_contract::FeeDataContractValidationVersion;
use sha2::{Digest, Sha256};

pub mod v1;
#[derive(Clone, Debug, Default)]
pub struct StateTransitionMinFees {
    pub credit_transfer: u64,
    pub credit_withdrawal: u64,
    pub identity_update: u64,
    pub document_batch_sub_transition: u64,
    pub contract_create: u64,
    pub contract_update: u64,
}

impl StateTransitionMinFees {
    pub(crate) fn to_hash(&self) -> u64 {
        let mut hasher = Sha256::new();
        Digest::update(&mut hasher, &self.credit_transfer.to_be_bytes());
        Digest::update(&mut hasher, &self.credit_withdrawal.to_be_bytes());
        Digest::update(&mut hasher, &self.identity_update.to_be_bytes());
        Digest::update(
            &mut hasher,
            &self.document_batch_sub_transition.to_be_bytes(),
        );
        Digest::update(&mut hasher, &self.contract_create.to_be_bytes());
        Digest::update(&mut hasher, &self.contract_update.to_be_bytes());

        let result = hasher.finalize();
        // Use the first 8 bytes of the hash as the u64 representation
        let hash_bytes: [u8; 8] = result[0..8].try_into().unwrap();
        u64::from_be_bytes(hash_bytes)
    }
}
