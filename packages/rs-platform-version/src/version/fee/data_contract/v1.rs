use crate::version::fee::data_contract::FeeDataContractValidationVersion;

pub const FEE_DATA_CONTRACT_VALIDATION_VERSION1: FeeDataContractValidationVersion =
    FeeDataContractValidationVersion {
        document_type_base_fee: 500,
        document_type_size_fee: 10,
        document_type_per_property_fee: 40,
        document_type_base_non_unique_index_fee: 50,
        document_type_non_unique_index_per_property_fee: 30,
        document_type_base_unique_index_fee: 100,
        document_type_unique_index_per_property_fee: 60,
    };
