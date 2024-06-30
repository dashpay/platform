extern crate sha2;

use sha2::digest::Update;
use sha2::{Digest, Sha256};

pub mod v1;

#[derive(Clone, Debug, Default)]
pub struct FeeStorageVersion {
    pub storage_disk_usage_credit_per_byte: u64,
    pub storage_processing_credit_per_byte: u64,
    pub storage_load_credit_per_byte: u64,
    pub non_storage_load_credit_per_byte: u64,
    pub storage_seek_cost: u64,
}

impl FeeStorageVersion {
    pub(crate) fn to_hash(&self) -> u64 {
        let mut hasher = Sha256::new();
        Digest::update(
            &mut hasher,
            &self.storage_disk_usage_credit_per_byte.to_be_bytes(),
        );
        Digest::update(
            &mut hasher,
            &self.storage_processing_credit_per_byte.to_be_bytes(),
        );
        Digest::update(
            &mut hasher,
            &self.storage_load_credit_per_byte.to_be_bytes(),
        );
        Digest::update(
            &mut hasher,
            &self.non_storage_load_credit_per_byte.to_be_bytes(),
        );
        Digest::update(&mut hasher, &self.storage_seek_cost.to_be_bytes());

        let result = hasher.finalize();
        // Use the first 8 bytes of the hash as the u64 representation
        let hash_bytes: [u8; 8] = result[0..8].try_into().unwrap();
        u64::from_be_bytes(hash_bytes)
    }
}
