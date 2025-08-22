use bincode::{Decode, Encode};

pub mod v1;
pub mod v2;

/// These are the fee costs for various actions excluding validation
/// Re-Validation will happen on data contract update which is why we don't bundle them
/// together.
#[derive(Clone, Debug, Default, Encode, Decode, PartialEq, Eq)]
pub struct FeeDataContractRegistrationVersion {
    pub base_contract_registration_fee: u64,
    pub document_type_registration_fee: u64,
    pub document_type_base_non_unique_index_registration_fee: u64,
    pub document_type_base_unique_index_registration_fee: u64,
    /// All contested indexes are unique, but if the index is contested you only apply this fee
    pub document_type_base_contested_index_registration_fee: u64,
    pub token_registration_fee: u64,
    pub token_uses_perpetual_distribution_fee: u64,
    pub token_uses_pre_programmed_distribution_fee: u64,
    pub search_keyword_fee: u64,
}
