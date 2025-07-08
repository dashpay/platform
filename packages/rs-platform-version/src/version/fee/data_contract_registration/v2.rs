use crate::version::fee::data_contract_registration::FeeDataContractRegistrationVersion;

/// Introduced in protocol version 9 (version 2.0)
pub const FEE_DATA_CONTRACT_REGISTRATION_VERSION2: FeeDataContractRegistrationVersion =
    FeeDataContractRegistrationVersion {
        base_contract_registration_fee: 10_000_000_000, // 0.1 Dash
        document_type_registration_fee: 2_000_000_000,  // 0.02 Dash
        document_type_base_non_unique_index_registration_fee: 1_000_000_000, // 0.01 Dash
        document_type_base_unique_index_registration_fee: 1_000_000_000, // 0.01 Dash
        document_type_base_contested_index_registration_fee: 100_000_000_000, // 1 Dash
        token_registration_fee: 10_000_000_000,         // 0.1 Dash
        token_uses_perpetual_distribution_fee: 10_000_000_000, // 0.1 Dash
        token_uses_pre_programmed_distribution_fee: 10_000_000_000, // 0.1 Dash
        search_keyword_fee: 10_000_000_000,             // 0.1 Dash
    };
