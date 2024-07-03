use bincode::{Decode, Encode};
use sha2::{Digest, Sha256};

pub mod v1;

#[derive(Clone, Debug, Encode, Decode, Default)]
pub struct FeeProcessingVersion {
    pub fetch_identity_balance_processing_cost: u64,
    pub fetch_identity_revision_processing_cost: u64,
    pub fetch_identity_balance_and_revision_processing_cost: u64,
    pub fetch_identity_cost_per_look_up_key_by_id: u64,
    pub fetch_single_identity_key_processing_cost: u64,
    pub validate_key_structure: u64,
}

impl FeeProcessingVersion {
    pub(crate) fn to_hash(&self) -> u64 {
        let mut hasher = Sha256::new();
        Digest::update(
            &mut hasher,
            &self.fetch_identity_balance_processing_cost.to_be_bytes(),
        );
        Digest::update(
            &mut hasher,
            &self.fetch_identity_revision_processing_cost.to_be_bytes(),
        );
        Digest::update(
            &mut hasher,
            &self
                .fetch_identity_balance_and_revision_processing_cost
                .to_be_bytes(),
        );
        Digest::update(
            &mut hasher,
            &self.fetch_identity_cost_per_look_up_key_by_id.to_be_bytes(),
        );
        Digest::update(
            &mut hasher,
            &self.fetch_single_identity_key_processing_cost.to_be_bytes(),
        );
        Digest::update(&mut hasher, &self.validate_key_structure.to_be_bytes());

        let result = hasher.finalize();
        // Use the first 8 bytes of the hash as the u64 representation
        let hash_bytes: [u8; 8] = result[0..8].try_into().unwrap();
        u64::from_be_bytes(hash_bytes)
    }
}
