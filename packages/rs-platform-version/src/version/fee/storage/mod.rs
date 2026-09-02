use bincode::{Decode, Encode};

pub mod v1;
pub mod v2;

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

/// Frozen pre-protocol-version-14 layout of `FeeStorageVersion`, kept only so
/// that `FeeVersionFieldsBeforeVersion4` (persisted inside platform state
/// before 1.4) keeps decoding after new fields are added to the live struct.
#[derive(Clone, Debug, Encode, Decode, Default, PartialEq, Eq)]
pub struct FeeStorageVersionBeforeVersion14 {
    pub storage_disk_usage_credit_per_byte: u64,
    pub storage_processing_credit_per_byte: u64,
    pub storage_load_credit_per_byte: u64,
    pub non_storage_load_credit_per_byte: u64,
    pub storage_seek_cost: u64,
}

impl From<FeeStorageVersionBeforeVersion14> for FeeStorageVersion {
    fn from(value: FeeStorageVersionBeforeVersion14) -> Self {
        FeeStorageVersion {
            storage_disk_usage_credit_per_byte: value.storage_disk_usage_credit_per_byte,
            storage_processing_credit_per_byte: value.storage_processing_credit_per_byte,
            storage_load_credit_per_byte: value.storage_load_credit_per_byte,
            non_storage_load_credit_per_byte: value.non_storage_load_credit_per_byte,
            storage_seek_cost: value.storage_seek_cost,
            // Unreachable before protocol version 14: the `ttl` grammar did not exist.
            ttl_ephemeral_disk_usage_credit_per_byte: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FeeStorageVersion, FeeStorageVersionBeforeVersion14};

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
    }

    #[test]
    // Guards the wire layout FeeVersionFieldsBeforeVersion4 persists: five u64s, nothing more.
    fn test_fee_storage_version_before_version_14_decodes_pre_existing_layout() {
        let frozen = FeeStorageVersionBeforeVersion14 {
            storage_disk_usage_credit_per_byte: 1,
            storage_processing_credit_per_byte: 2,
            storage_load_credit_per_byte: 3,
            non_storage_load_credit_per_byte: 4,
            storage_seek_cost: 5,
        };

        let bytes = bincode::encode_to_vec(&frozen, bincode::config::standard())
            .expect("expected to encode frozen storage fee version");
        assert_eq!(bytes, vec![1, 2, 3, 4, 5]);

        let (decoded, _): (FeeStorageVersionBeforeVersion14, _) =
            bincode::decode_from_slice(&bytes, bincode::config::standard())
                .expect("expected to decode frozen storage fee version");
        assert_eq!(decoded, frozen);

        let live = FeeStorageVersion::from(decoded);
        assert_eq!(live.storage_seek_cost, 5);
        assert_eq!(live.ttl_ephemeral_disk_usage_credit_per_byte, 0);
    }
}
