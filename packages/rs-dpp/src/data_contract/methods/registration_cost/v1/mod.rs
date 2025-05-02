use crate::data_contract::accessors::v0::DataContractV0Getters;
use crate::data_contract::accessors::v1::DataContractV1Getters;
use crate::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use crate::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::Index;
use crate::data_contract::serialized_version::DataContractInSerializationFormat;
use crate::fee::Credits;
use crate::prelude::DataContract;
use platform_value::Value;
use platform_version::version::PlatformVersion;

impl DataContract {
    /// Computes the registration cost of a data contract based on its document types,
    /// indexes, token configurations, and keyword count.
    ///
    /// # Parameters
    /// - `platform_version`: A reference to the current platform version providing fee parameters.
    ///
    /// # Returns
    /// - `Ok(Credits)`: The total registration cost in credits if all additions are overflow-safe.
    /// - `Err(ProtocolError)`: If any arithmetic operation results in overflow.
    ///
    /// # Fee Components
    /// - Base contract registration fee.
    /// - Per document type registration fee.
    /// - Per index registration fee (unique and non-unique).
    /// - Token registration fee per token.
    /// - Additional fees for tokens using perpetual or pre-programmed distribution.
    /// - Search keyword fees (`keyword_count * search_keyword_fee`).
    pub(super) fn registration_cost_v1(&self, platform_version: &PlatformVersion) -> Credits {
        let fee_version = &platform_version.fee_version.data_contract_registration;
        let mut cost = fee_version.base_contract_registration_fee;

        for document_type in self.document_types().values() {
            cost = cost.saturating_add(fee_version.document_type_registration_fee);

            for index in document_type.indexes().values() {
                let base_index_fee = if index.contested_index.is_some() {
                    fee_version.document_type_base_contested_index_registration_fee
                } else if index.unique {
                    fee_version.document_type_base_unique_index_registration_fee
                } else {
                    fee_version.document_type_base_non_unique_index_registration_fee
                };

                cost = cost.saturating_add(base_index_fee);
            }
        }

        for token_config in self.tokens().values() {
            cost = cost.saturating_add(fee_version.token_registration_fee);

            if token_config
                .distribution_rules()
                .perpetual_distribution()
                .is_some()
            {
                cost = cost.saturating_add(fee_version.token_uses_perpetual_distribution_fee);
            }

            if token_config
                .distribution_rules()
                .pre_programmed_distribution()
                .is_some()
            {
                cost = cost.saturating_add(fee_version.token_uses_pre_programmed_distribution_fee);
            }
        }

        let keyword_cost = fee_version
            .search_keyword_fee
            .saturating_mul(self.keywords().len() as u64);

        cost = cost.saturating_add(keyword_cost);

        cost
    }
}

impl DataContractInSerializationFormat {
    /// Computes the registration cost of a data contract based on its document types,
    /// indexes, token configurations, and keyword count.
    ///
    /// # Parameters
    /// - `platform_version`: A reference to the current platform version providing fee parameters.
    ///
    /// # Returns
    /// - `Ok(Credits)`: The total registration cost in credits if all additions are overflow-safe.
    /// - `Err(ProtocolError)`: If any arithmetic operation results in overflow.
    ///
    /// # Fee Components
    /// - Base contract registration fee.
    /// - Per document type registration fee.
    /// - Per index registration fee (unique and non-unique).
    /// - Token registration fee per token.
    /// - Additional fees for tokens using perpetual or pre-programmed distribution.
    /// - Search keyword fees (`keyword_count * search_keyword_fee`).
    pub(super) fn registration_cost_v1(&self, platform_version: &PlatformVersion) -> Credits {
        let fee_version = &platform_version.fee_version.data_contract_registration;
        let mut cost = fee_version.base_contract_registration_fee;

        for document_type_schema in self.document_schemas().values() {
            cost = cost.saturating_add(fee_version.document_type_registration_fee);

            // If this is not okay the registration will fail on basic validation
            if let Ok(schema_map) = document_type_schema.to_map() {
                // Initialize indices
                if let Ok(Some(index_values)) = Value::inner_optional_array_slice_value(
                    schema_map,
                    crate::data_contract::document_type::property_names::INDICES,
                ) {
                    for index_value in index_values {
                        if let Ok(index_value_map) = index_value.to_map() {
                            if let Ok(index) = Index::try_from(index_value_map.as_slice()) {
                                let base_index_fee = if index.contested_index.is_some() {
                                    fee_version.document_type_base_contested_index_registration_fee
                                } else if index.unique {
                                    fee_version.document_type_base_unique_index_registration_fee
                                } else {
                                    fee_version.document_type_base_non_unique_index_registration_fee
                                };
                                cost = cost.saturating_add(base_index_fee);
                            }
                        }
                    }
                }
            };
        }

        for token_config in self.tokens().values() {
            cost = cost.saturating_add(fee_version.token_registration_fee);

            if token_config
                .distribution_rules()
                .perpetual_distribution()
                .is_some()
            {
                cost = cost.saturating_add(fee_version.token_uses_perpetual_distribution_fee);
            }

            if token_config
                .distribution_rules()
                .pre_programmed_distribution()
                .is_some()
            {
                cost = cost.saturating_add(fee_version.token_uses_pre_programmed_distribution_fee);
            }
        }

        let keyword_cost = fee_version
            .search_keyword_fee
            .saturating_mul(self.keywords().len() as u64);

        cost = cost.saturating_add(keyword_cost);

        cost
    }
}
