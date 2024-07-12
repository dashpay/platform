use bincode::{Decode, Encode};

pub mod v1;

#[derive(Clone, Debug, Encode, Decode, Default, PartialEq, Eq)]
pub struct FeeHashingVersion {
    pub blake3_base: u64,
    pub blake3_per_block: u64,
    pub sha256_per_block: u64,
    pub sha256_ripe_md160_base: u64,
    pub single_sha256_base: u64,
}

#[cfg(test)]
mod tests {
    use super::FeeHashingVersion;

    #[test]
    // If this test failed, then a new field was added in FeeHashingVersion. And the corresponding eq needs to be updated as well
    fn test_fee_hashing_version_equality() {
        let version1 = FeeHashingVersion {
            single_sha256_base: 1,
            blake3_base: 2,
            sha256_ripe_md160_base: 3,
            sha256_per_block: 4,
            blake3_per_block: 5,
        };

        let version2 = FeeHashingVersion {
            single_sha256_base: 1,
            blake3_base: 2,
            sha256_ripe_md160_base: 3,
            sha256_per_block: 4,
            blake3_per_block: 5,
        };

        // This assertion will check if all fields are considered in the equality comparison
        assert_eq!(version1, version2, "FeeHashingVersion equality test failed. If a field was added or removed, update the Eq implementation.");
    }
}
