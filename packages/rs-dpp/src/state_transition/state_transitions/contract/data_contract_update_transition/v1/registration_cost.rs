use crate::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use crate::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use crate::data_contract::document_type::{property_names, Index, IndexGrammarAdmissions};
use crate::fee::Credits;
use crate::state_transition::data_contract_update_transition::DataContractUpdateTransitionV1;
use crate::ProtocolError;
use platform_value::Value;
use platform_version::version::PlatformVersion;

impl DataContractUpdateTransitionV1 {
    /// The registration cost of what this update adds to the contract.
    ///
    /// A full-contract (V0) update is charged the registration cost of the
    /// whole contract it re-sends. A delta only registers new and updated
    /// document types (with their indexes), new tokens and added keywords,
    /// so only those are charged; there is no base contract fee because no
    /// contract is being registered.
    ///
    /// Dispatches on the same `registration_cost` method version as
    /// [`DataContract::registration_cost`](crate::data_contract::DataContract::registration_cost).
    pub fn registration_cost(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .methods
            .registration_cost
        {
            0 => Ok(0),
            1 => Ok(self.registration_cost_v1(platform_version)),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "DataContractUpdateTransitionV1::registration_cost".to_string(),
                known_versions: vec![0, 1],
                received: version,
            }),
        }
    }

    fn registration_cost_v1(&self, platform_version: &PlatformVersion) -> Credits {
        let fee_version = &platform_version.fee_version.data_contract_registration;
        let mut cost: Credits = 0;

        for document_type_schema in self
            .new_document_schemas
            .values()
            .chain(self.updated_document_schemas.values())
        {
            cost = cost.saturating_add(fee_version.document_type_registration_fee);

            // A schema that does not parse fails validation; billing what
            // does parse mirrors the full-contract registration cost.
            let Ok(schema_map) = document_type_schema.to_map() else {
                continue;
            };
            let Ok(Some(index_values)) =
                Value::inner_optional_array_slice_value(schema_map, property_names::INDICES)
            else {
                continue;
            };
            let admissions = IndexGrammarAdmissions::for_schema_generation(
                platform_version
                    .dpp
                    .contract_versions
                    .document_type_versions
                    .schema
                    .document_type_schema,
            );
            for index_value in index_values {
                let Ok(index_value_map) = index_value.to_map() else {
                    continue;
                };
                let Ok(index) = Index::try_from_value_map(index_value_map.as_slice(), admissions)
                else {
                    continue;
                };
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

        let keyword_cost = fee_version
            .search_keyword_fee
            .saturating_mul(self.add_keywords.len() as u64);

        cost.saturating_add(keyword_cost)
    }
}
