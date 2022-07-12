use std::borrow::Borrow;

use crate::{
    consensus::basic::BasicError,
    data_contract::DataContract,
    util::{
        json_schema::{Index, JsonSchemaExt},
        json_value::JsonValueExt,
    },
    validation::ValidationResult,
    ProtocolError,
};
use serde_json::Value as JsonValue;

pub fn validate_partial_compound_indices(
    raw_document_transitions: impl IntoIterator<Item = impl Borrow<JsonValue>>,
    data_contract: &DataContract,
) -> Result<ValidationResult, ProtocolError> {
    let mut result = ValidationResult::default();

    for transition in raw_document_transitions {
        let raw_transition = transition.borrow();
        let document_type = raw_transition.get_string("$type")?;
        let document_schema = data_contract.get_document_schema(document_type)?;
        let indices = document_schema.get_indices().unwrap_or_default();

        if indices.is_empty() {
            continue;
        }
        result.merge(validate_indices(&indices, document_type, raw_transition));
    }

    Ok(result)
}

pub fn validate_indices(
    indices: &[Index],
    document_type: &str,
    raw_transition: &JsonValue,
) -> ValidationResult {
    let mut validation_result = ValidationResult::default();

    for index in indices
        .iter()
        .filter(|i| i.unique && i.properties.len() > 1)
    {
        let properties = index
            .properties
            .iter()
            .map(|property| property.keys().next().unwrap());

        if !are_all_properties_defined_or_undefined(properties.clone(), raw_transition) {
            validation_result.add_error(BasicError::InconsistentCompoundIndexDataError {
                index_properties: properties.map(ToOwned::to_owned).collect(),
                document_type: document_type.to_string(),
            })
        }
    }

    validation_result
}

fn are_all_properties_defined_or_undefined(
    properties: impl IntoIterator<Item = impl AsRef<str>>,
    json_value: &JsonValue,
) -> bool {
    let mut defined_property_counter = 0;
    let mut properties_len = 0;

    for property in properties {
        properties_len += 1;
        let property_name = property.as_ref();

        if property_name == "$ownerId" {
            defined_property_counter += 1;
            continue;
        }
        if property.as_ref().starts_with('$') && json_value.get(property_name).is_some() {
            defined_property_counter += 1;
            continue;
        }
        if json_value.get_value(property_name).is_ok() {
            defined_property_counter += 1
        }
    }

    defined_property_counter == 0 || defined_property_counter == properties_len
}

#[cfg(test)]
mod test {
    use super::are_all_properties_defined_or_undefined;
    use serde_json::json;

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
            &input
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
            &input
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
            &input
        ));
    }
}
