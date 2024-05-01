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
