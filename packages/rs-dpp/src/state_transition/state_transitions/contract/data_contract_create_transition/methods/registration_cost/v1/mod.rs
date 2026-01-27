use std::collections::BTreeMap;

use crate::data_contract::associated_token::token_configuration::accessors::v0::TokenConfigurationV0Getters;
use crate::data_contract::associated_token::token_configuration::TokenConfiguration;
use crate::data_contract::associated_token::token_distribution_rules::accessors::v0::TokenDistributionRulesV0Getters;
use crate::data_contract::document_type::Index;
use crate::data_contract::TokenContractPosition;
use crate::fee::Credits;
use platform_value::Value;
use platform_version::version::PlatformVersion;

/// Computes the registration cost based on document schemas, indexes, token configurations,
/// and keyword count.
///
/// # Parameters
/// - `document_schemas`: A map of document names to their JSON schema values.
/// - `tokens`: A map of token positions to their configurations.
/// - `keywords_count`: The number of keywords to register.
/// - `base_fee`: The base fee to start with (use base_contract_registration_fee for create, 0 for update).
/// - `platform_version`: A reference to the current platform version providing fee parameters.
///
/// # Returns
/// - `Credits`: The total registration cost in credits.
///
/// # Fee Components
/// - Base fee (passed as parameter).
/// - Per document type registration fee.
/// - Per index registration fee (contested, unique, and non-unique).
/// - Token registration fee per token.
/// - Additional fees for tokens using perpetual or pre-programmed distribution.
/// - Search keyword fees (`keywords_count * search_keyword_fee`).
pub(in crate::state_transition::state_transitions::contract) fn registration_cost_from_fields(
    document_schemas: &BTreeMap<String, Value>,
    tokens: &BTreeMap<TokenContractPosition, TokenConfiguration>,
    keywords_count: usize,
    base_fee: Credits,
    platform_version: &PlatformVersion,
) -> Credits {
    let fee_version = &platform_version.fee_version.data_contract_registration;
    let mut cost = base_fee;

    // Calculate cost for document schemas
    for document_type_schema in document_schemas.values() {
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

    // Calculate cost for tokens
    for token_config in tokens.values() {
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

    // Calculate cost for keywords
    let keyword_cost = fee_version
        .search_keyword_fee
        .saturating_mul(keywords_count as u64);

    cost.saturating_add(keyword_cost)
}
