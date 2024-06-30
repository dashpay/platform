use crate::version::fee::processing::FeeProcessingVersion;
use sha2::{Digest, Sha256};

pub mod v1;

#[derive(Clone, Debug, Default)]
pub struct FeeDataContractValidationVersion {
    pub document_type_base_fee: u64,
    pub document_type_size_fee: u64,
    pub document_type_per_property_fee: u64,
    pub document_type_base_non_unique_index_fee: u64,
    pub document_type_non_unique_index_per_property_fee: u64,
    pub document_type_base_unique_index_fee: u64,
    pub document_type_unique_index_per_property_fee: u64,
}

impl FeeDataContractValidationVersion {
    pub(crate) fn to_hash(&self) -> u64 {
        let mut hasher = Sha256::new();
        Digest::update(&mut hasher, &self.document_type_base_fee.to_be_bytes());
        Digest::update(&mut hasher, &self.document_type_size_fee.to_be_bytes());
        Digest::update(
            &mut hasher,
            &self.document_type_per_property_fee.to_be_bytes(),
        );
        Digest::update(
            &mut hasher,
            &self.document_type_base_non_unique_index_fee.to_be_bytes(),
        );
        Digest::update(
            &mut hasher,
            &self
                .document_type_non_unique_index_per_property_fee
                .to_be_bytes(),
        );
        Digest::update(
            &mut hasher,
            &self.document_type_base_unique_index_fee.to_be_bytes(),
        );
        Digest::update(
            &mut hasher,
            &self
                .document_type_unique_index_per_property_fee
                .to_be_bytes(),
        );

        let result = hasher.finalize();
        // Use the first 8 bytes of the hash as the u64 representation
        let hash_bytes: [u8; 8] = result[0..8].try_into().unwrap();
        u64::from_be_bytes(hash_bytes)
    }
}
