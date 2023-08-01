use std::collections::BTreeMap;
use platform_value::Value;
use crate::data_contract::document_type::Index;
use crate::ProtocolError;

impl Index {

    pub(super) fn get_wrongly_updated_non_unique_index_v0<'a>(
        existing_schema_indices: &'a [Index],
        new_indices: &'a BTreeMap<String, Index>,
        existing_schema: &'a Value,
    ) -> Result<Option<&'a Index>, ProtocolError> {
        // Checking every existing non-unique index, and it's respective new index
        // if they are changed per spec
        for index_definition in existing_schema_indices.iter().filter(|i| !i.unique) {
            let maybe_new_index_definition = new_indices.get(&index_definition.name);
            if let Some(new_index_definition) = maybe_new_index_definition {
                // Non-unique index can be ONLY updated by appending. The 'old' properties in the new
                // index must remain intact.
                let index_properties_len = index_definition.properties.len();
                if new_index_definition.properties[0..index_properties_len]
                    != index_definition.properties
                {
                    return Ok(Some(index_definition));
                }

                // Check if the rest of new indexes are defined in the existing schema
                for property in
                new_index_definition.properties[index_definition.properties.len()..].iter()
                {
                    if let Ok(indices) = existing_schema.get_value("indices") {
                        let indices_array = indices.as_array().ok_or_else(|| {
                            ProtocolError::ParsingError(
                                "Error parsing schema: indices is not an array".to_string(),
                            )
                        })?;

                        for index in indices_array {
                            let properties_value = index.get_value("properties")?;
                            let properties_array = properties_value.as_array().ok_or_else(|| {
                                ProtocolError::ParsingError(
                                    "Error parsing schema: properties is not an array".to_string(),
                                )
                            })?;

                            for property_to_check in properties_array {
                                if property_to_check.has(&property.name) {
                                    return Ok(Some(index_definition));
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(None)
    }
}