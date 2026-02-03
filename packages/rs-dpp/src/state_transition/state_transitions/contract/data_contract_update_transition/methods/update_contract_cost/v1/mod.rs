use std::collections::BTreeMap;

use crate::data_contract::document_type::Index;
use crate::fee::Credits;
use platform_value::Value;
use platform_version::version::PlatformVersion;

/// Computes the update contract cost based on updated document schemas.
///
/// # Parameters
/// - `updated_document_schemas`: A map of document names to their updated JSON schema values.
/// - `platform_version`: A reference to the current platform version providing fee parameters.
///
/// # Returns
/// - `Credits`: The total update cost in credits.
///
/// # Fee Components
/// - Per updated document type fee (same as registration fee for document types).
/// - Per index fee for indexes in updated schemas.
///
/// Note: This charges for the full schema content of updated documents, as we cannot
/// determine what specifically changed within a schema without the original.
pub(in crate::state_transition::state_transitions::contract) fn update_contract_cost_from_fields(
    updated_document_schemas: &BTreeMap<String, Value>,
    platform_version: &PlatformVersion,
) -> Credits {
    let fee_version = &platform_version.fee_version.data_contract_registration;
    let mut cost: Credits = 0;

    // Calculate cost for updated document schemas
    for document_type_schema in updated_document_schemas.values() {
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

    cost
}
