use bincode::{Decode, Encode};

pub mod v1;

#[derive(Clone, Debug, Default, Encode, Decode, PartialEq, Eq)]
pub struct FeeDataContractValidationVersion {
    pub document_type_base_fee: u64,
    pub document_type_size_fee: u64,
    pub document_type_per_property_fee: u64,
    pub document_type_base_non_unique_index_fee: u64,
    pub document_type_non_unique_index_per_property_fee: u64,
    pub document_type_base_unique_index_fee: u64,
    pub document_type_unique_index_per_property_fee: u64,
}

#[cfg(test)]
mod tests {
    use super::FeeDataContractValidationVersion;

    #[test]
    // If this test failed, then a new field was added in FeeDataContractValidationVersion. And the corresponding eq needs to be updated as well
    fn test_fee_data_contract_validation_fees_version_equality() {
        let version1 = FeeDataContractValidationVersion {
            document_type_base_fee: 1,
            document_type_size_fee: 2,
            document_type_per_property_fee: 3,
            document_type_base_non_unique_index_fee: 4,
            document_type_non_unique_index_per_property_fee: 5,
            document_type_base_unique_index_fee: 6,
            document_type_unique_index_per_property_fee: 7,
        };

        let version2 = FeeDataContractValidationVersion {
            document_type_base_fee: 1,
            document_type_size_fee: 2,
            document_type_per_property_fee: 3,
            document_type_base_non_unique_index_fee: 4,
            document_type_non_unique_index_per_property_fee: 5,
            document_type_base_unique_index_fee: 6,
            document_type_unique_index_per_property_fee: 7,
        };

        // This assertion will check if all fields are considered in the equality comparison
        assert_eq!(version1, version2, "FeeDataContractValidationVersion equality test failed. If a field was added or removed, update the Eq implementation.");
    }
}
