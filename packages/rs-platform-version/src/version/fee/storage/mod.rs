use bincode::{Decode, Encode};

pub mod v1;

#[derive(Clone, Debug, Encode, Decode, Default)]
pub struct FeeStorageVersion {
    pub storage_disk_usage_credit_per_byte: u64,
    pub storage_processing_credit_per_byte: u64,
    pub storage_load_credit_per_byte: u64,
    pub non_storage_load_credit_per_byte: u64,
    pub storage_seek_cost: u64,
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
