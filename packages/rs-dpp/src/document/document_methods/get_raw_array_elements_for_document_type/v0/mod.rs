use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
use crate::data_contract::document_type::DocumentPropertyType;
use crate::data_contract::document_type::DocumentTypeRef;
use crate::document::DocumentV0Getters;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use platform_value::btreemap_extensions::BTreeValueMapPathHelper;
use platform_value::Value;
use std::collections::HashSet;

pub trait DocumentGetRawArrayElementsForDocumentTypeV0: DocumentV0Getters {
    /// Return array element values for an indexed array property.
    /// Each element is encoded for use as an index tree key.
    /// Returns an empty Vec if the field is not an array, is missing, or is empty.
    /// Duplicate elements are deduplicated.
    fn get_raw_array_elements_for_document_type_v0(
        &self,
        key_path: &str,
        document_type: DocumentTypeRef,
        _platform_version: &PlatformVersion,
    ) -> Result<Vec<Vec<u8>>, ProtocolError> {
        // Get the property definition to determine the array item type
        let property = document_type.flattened_properties().get(key_path);

        let array_item_type = match property {
            Some(prop) => match &prop.property_type {
                DocumentPropertyType::Array(item_type) => item_type,
                _ => {
                    // Not an array property - return empty
                    return Ok(vec![]);
                }
            },
            None => {
                // Property not found - return empty
                return Ok(vec![]);
            }
        };

        // Get the array value from the document
        let array_value = self.properties().get_optional_at_path(key_path)?;

        match array_value {
            None => Ok(vec![]), // Field not present
            Some(Value::Array(elements)) => {
                // Encode each element and deduplicate
                let mut seen: HashSet<Vec<u8>> = HashSet::new();
                let mut result: Vec<Vec<u8>> = Vec::with_capacity(elements.len());

                for element in elements {
                    // Skip null elements
                    if element.is_null() {
                        continue;
                    }

                    let encoded = array_item_type.encode_element_for_tree_keys(element)?;

                    // Deduplicate
                    if seen.insert(encoded.clone()) {
                        result.push(encoded);
                    }
                }

                Ok(result)
            }
            Some(_) => {
                // Field exists but is not an array - this shouldn't happen if schema is valid
                Ok(vec![])
            }
        }
    }
}
