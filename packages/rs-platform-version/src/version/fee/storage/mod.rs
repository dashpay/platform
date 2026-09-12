use bincode::{Decode, Encode};

pub mod v1;

#[derive(Clone, Debug, Encode, Decode, Default, PartialEq, Eq)]
pub struct FeeStorageVersion {
    pub storage_disk_usage_credit_per_byte: u64,
    pub storage_processing_credit_per_byte: u64,
    pub storage_load_credit_per_byte: u64,
    pub non_storage_load_credit_per_byte: u64,
    pub storage_seek_cost: u64,
    /// Credits charged per byte written under a TTL'd `timeRange` index
    /// subtree, billed to PROCESSING in place of the storage fee: the
    /// bytes provably live at most one week (the `ttl` cap) plus a
    /// bounded drainage lag, so charging them the perpetual-retention
    /// storage price would overprice them by orders of magnitude. Zero
    /// until protocol version 14 — the `ttl` grammar does not parse
    /// before it, so no ephemeral-classified operation can exist.
    pub ttl_ephemeral_disk_usage_credit_per_byte: u64,
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
            ttl_ephemeral_disk_usage_credit_per_byte: 6,
        };

        let version2 = FeeStorageVersion {
            storage_disk_usage_credit_per_byte: 1,
            storage_processing_credit_per_byte: 2,
            storage_load_credit_per_byte: 3,
            non_storage_load_credit_per_byte: 4,
            storage_seek_cost: 5,
            ttl_ephemeral_disk_usage_credit_per_byte: 6,
        };

        // This assertion will check if all fields are considered in the equality comparison
        assert_eq!(version1, version2, "FeeStorageVersion equality test failed. If a field was added or removed, update the Eq implementation.");

        // And the inequality direction: a difference in the newest field
        // alone must be visible to Eq.
        let version3 = FeeStorageVersion {
            ttl_ephemeral_disk_usage_credit_per_byte: 7,
            ..version2.clone()
        };
        assert_ne!(
            version1, version3,
            "FeeStorageVersion equality must distinguish ttl_ephemeral_disk_usage_credit_per_byte"
        );
    }
}
