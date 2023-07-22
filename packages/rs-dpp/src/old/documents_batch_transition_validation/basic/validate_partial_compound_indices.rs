use std::borrow::Borrow;
use std::collections::BTreeMap;

use platform_value::btreemap_extensions::BTreeValueMapPathHelper;
use platform_value::Value;

use crate::consensus::basic::document::InconsistentCompoundIndexDataError;
use crate::{
    consensus::basic::BasicError,
    data_contract::DataContract,
    util::json_schema::{JsonSchemaExt},
    validation::SimpleConsensusValidationResult,
    ProtocolError,
};
use crate::data_contract::document_schema::DataContractDocumentSchemaMethodsV0;
use crate::data_contract::document_type::Index;

pub fn validate_partial_compound_indices<'a>(
    raw_document_transitions: impl IntoIterator<Item = &'a Value>,
    data_contract: &DataContract,
) -> Result<SimpleConsensusValidationResult, ProtocolError> {
    let mut result = SimpleConsensusValidationResult::default();

    for transition in raw_document_transitions {
        let document_type = transition.get_str("$type")?;
        let document_schema = data_contract.get_document_schema(document_type)?;
        let indices = document_schema.get_indices::<Vec<_>>().unwrap_or_default();

        if indices.is_empty() {
            continue;
        }
        result.merge(validate_indices(
            &indices,
            document_type,
            &transition.to_btree_ref_string_map()?,
        ));
    }

    Ok(result)
}

pub fn validate_indices<V>(
    indices: &[Index],
    document_type: &str,
    raw_transition_map: &BTreeMap<String, V>,
) -> SimpleConsensusValidationResult
where
    V: Borrow<Value>,
{
    let mut validation_result = SimpleConsensusValidationResult::default();

    for index in indices
        .iter()
        .filter(|i| i.unique && i.properties.len() > 1)
    {
        let properties = index.properties.iter().map(|property| &property.name);

        if !are_all_properties_defined_or_undefined(properties.clone(), raw_transition_map) {
            validation_result.add_error(BasicError::InconsistentCompoundIndexDataError(
                InconsistentCompoundIndexDataError::new(
                    document_type.to_string(),
                    properties.map(ToOwned::to_owned).collect(),
                ),
            ))
        }
    }

    validation_result
}

fn are_all_properties_defined_or_undefined<V>(
    properties: impl IntoIterator<Item = impl AsRef<str>>,
    map: &BTreeMap<String, V>,
) -> bool
where
    V: Borrow<Value>,
{
    let mut defined_property_counter = 0;
    let mut properties_len = 0;

    for property in properties {
        properties_len += 1;
        let property_name = property.as_ref();

        if property_name == "$ownerId" {
            defined_property_counter += 1;
            continue;
        }
        //todo: this seems weird
        if property.as_ref().starts_with('$')
            && map
                .get_optional_at_path(property_name)
                .ok()
                .flatten()
                .is_some()
        {
            defined_property_counter += 1;
            continue;
        }
        if map
            .get_optional_at_path(property_name)
            .ok()
            .flatten()
            .is_some()
        {
            defined_property_counter += 1
        }
    }

    defined_property_counter == 0 || defined_property_counter == properties_len
}

#[cfg(test)]
mod test {
    use platform_value::converter::serde_json::BTreeValueJsonConverter;
    use serde_json::json;
    use std::collections::BTreeMap;

    use super::are_all_properties_defined_or_undefined;

    #[test]
    fn should_return_true_if_all_properties_are_defined() {
        let input = json!({
            "alpha" : { },
            "bravo" : {
                "charlie" : {},
            },
            "delta" : {},
        });

        let property_names = ["alpha", "bravo.charlie"];

        assert!(are_all_properties_defined_or_undefined(
            property_names,
            &BTreeMap::from_json_value(input).unwrap()
        ));
    }

    #[test]
    fn should_return_false_if_some_properties_are_undefined() {
        let input = json!({
            "alpha" : { },
            "bravo" : {
                "charlie" : {},
            },
            "delta" : {},
        });
        let property_names = ["alpha", "bravo.charlie", "echo"];

        assert!(!are_all_properties_defined_or_undefined(
            property_names,
            &BTreeMap::from_json_value(input).unwrap()
        ));
    }

    #[test]
    fn should_return_true_if_all_properties_are_undefined() {
        let input = json!({
            "alpha" : { },
            "bravo" : {
                "charlie" : {},
            },
            "delta" : {},
        });
        let property_names = ["echo", "foxtrot", "golf"];

        assert!(are_all_properties_defined_or_undefined(
            property_names,
            &BTreeMap::from_json_value(input).unwrap()
        ));
    }
}
