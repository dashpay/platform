use bincode::{Decode, Encode};

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

impl PartialEq for FeeProcessingVersion {
    fn eq(&self, other: &Self) -> bool {
        self.fetch_identity_balance_processing_cost == other.fetch_identity_balance_processing_cost
            && self.fetch_identity_revision_processing_cost
                == other.fetch_identity_revision_processing_cost
            && self.fetch_identity_balance_and_revision_processing_cost
                == other.fetch_identity_balance_and_revision_processing_cost
            && self.fetch_identity_cost_per_look_up_key_by_id
                == other.fetch_identity_cost_per_look_up_key_by_id
            && self.fetch_single_identity_key_processing_cost
                == other.fetch_single_identity_key_processing_cost
            && self.validate_key_structure == other.validate_key_structure
    }
}

#[cfg(test)]
mod tests {
    use super::FeeProcessingVersion;

    #[test]
    // If this test failed, then a new field was added in FeeProcessingVersion. And the corresponding eq needs to be updated as well
    fn test_fee_processing_version_equality() {
        let version1 = FeeProcessingVersion {
            fetch_identity_balance_processing_cost: 1,
            fetch_identity_revision_processing_cost: 2,
            fetch_identity_balance_and_revision_processing_cost: 3,
            fetch_identity_cost_per_look_up_key_by_id: 4,
            fetch_single_identity_key_processing_cost: 5,
            validate_key_structure: 6,
        };

        let version2 = FeeProcessingVersion {
            fetch_identity_balance_processing_cost: 1,
            fetch_identity_revision_processing_cost: 2,
            fetch_identity_balance_and_revision_processing_cost: 3,
            fetch_identity_cost_per_look_up_key_by_id: 4,
            fetch_single_identity_key_processing_cost: 5,
            validate_key_structure: 6,
        };

        // This assertion will check if all fields are considered in the equality comparison
        assert_eq!(version1, version2, "FeeProcessingVersion equality test failed. If a field was added or removed, update the Eq implementation.");
    }
}
