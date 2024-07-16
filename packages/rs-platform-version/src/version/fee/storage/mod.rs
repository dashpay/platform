use bincode::{Decode, Encode};

pub mod v1;

#[derive(Clone, Debug, Encode, Decode, Default, PartialEq, Eq)]
pub struct FeeStorageVersion {
    pub storage_disk_usage_credit_per_byte: u64,
    pub storage_processing_credit_per_byte: u64,
    pub storage_load_credit_per_byte: u64,
    pub non_storage_load_credit_per_byte: u64,
    pub storage_seek_cost: u64,
}

#[cfg(test)]
mod tests {
    use super::FeeStorageVersion;

    #[test]
    // If this test failed, then a new field was added in FeeProcessingVersion. And the corresponding eq needs to be updated as well
    fn test_fee_storage_version_equality() {
        let version1 = FeeStorageVersion {
            storage_disk_usage_credit_per_byte: 1,
            storage_processing_credit_per_byte: 2,
            storage_load_credit_per_byte: 3,
            non_storage_load_credit_per_byte: 4,
            storage_seek_cost: 5,
        };

        let version2 = FeeStorageVersion {
            storage_disk_usage_credit_per_byte: 1,
            storage_processing_credit_per_byte: 2,
            storage_load_credit_per_byte: 3,
            non_storage_load_credit_per_byte: 4,
            storage_seek_cost: 5,
        };

        // This assertion will check if all fields are considered in the equality comparison
        assert_eq!(version1, version2, "FeeStorageVersion equality test failed. If a field was added or removed, update the Eq implementation.");
    }
}
