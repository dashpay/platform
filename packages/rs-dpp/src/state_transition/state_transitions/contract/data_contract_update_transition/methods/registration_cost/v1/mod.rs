use crate::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use crate::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use crate::data_contract::document_type::Index;
use crate::fee::Credits;
use crate::state_transition::data_contract_update_transition::DataContractUpdateTransitionV1;
use platform_value::Value;
use platform_version::version::PlatformVersion;

impl DataContractUpdateTransitionV1 {
    /// Computes the registration cost of a data contract update transition based on
    /// new items being added (new document schemas, new tokens, added keywords).
    ///
    /// # Parameters
    /// - `platform_version`: A reference to the current platform version providing fee parameters.
    ///
    /// # Returns
    /// - `Credits`: The total registration cost in credits for new items.
    ///
    /// # Fee Components
    /// - Per new document type registration fee.
    /// - Per index registration fee for new document types (unique and non-unique).
    /// - Token registration fee per new token.
    /// - Additional fees for new tokens using perpetual or pre-programmed distribution.
    /// - Search keyword fees for added keywords (`added_keyword_count * search_keyword_fee`).
    pub(in crate::state_transition::state_transitions::contract::data_contract_update_transition::methods) fn registration_cost_v1(
        &self,
        platform_version: &PlatformVersion,
    ) -> Credits {
        let fee_version = &platform_version.fee_version.data_contract_registration;
        let mut cost: Credits = 0;

        // Calculate cost for new document schemas
        for document_type_schema in self.new_document_schemas.values() {
            cost = cost.saturating_add(fee_version.document_type_registration_fee);

            // Parse indexes from the schema if present
            if let Ok(schema_map) = document_type_schema.to_map() {
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
            }
        }

        // Calculate cost for new tokens
        for token_config in self.new_tokens.values() {
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

        // Calculate cost for added keywords
        let keyword_cost = fee_version
            .search_keyword_fee
            .saturating_mul(self.add_keywords.len() as u64);

        cost = cost.saturating_add(keyword_cost);

        cost
    }
}
