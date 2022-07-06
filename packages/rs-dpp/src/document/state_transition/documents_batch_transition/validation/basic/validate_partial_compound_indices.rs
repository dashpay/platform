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
    use super::{are_all_properties_defined_or_undefined, validate_partial_compound_indices};
    use serde_json::json;

    fn get_basic_error(result: &ValidationResult, error_number: usize) -> &BasicError {
        match result
            .errors
            .get(error_number)
            .expect("error should be found")
        {
            ConsensusError::BasicError(basic_error) => &**basic_error,
            _ => panic!(
                "error '{:?}' isn't a basic error",
                result.errors[error_number]
            ),
        }
    }

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

    use crate::{
        consensus::{basic::BasicError, ConsensusError},
        data_contract::DataContract,
        document::{
            document_transition::{Action, DocumentTransitionObjectLike},
            Document,
        },
        tests::fixtures::{
            get_data_contract_fixture, get_document_transitions_fixture, get_documents_fixture,
        },
        util::json_value::JsonValueExt,
        validation::ValidationResult,
    };

    struct TestData {
        data_contract: DataContract,
        documents: Vec<Document>,
    }

    fn setup_test() -> TestData {
        let data_contract = get_data_contract_fixture(None);
        let documents =
            get_documents_fixture(data_contract.clone()).expect("documents should be created");

        TestData {
            data_contract,
            documents,
        }
    }

    #[test]
    fn should_return_invalid_result_if_compound_index_contains_not_all_fields() {
        let TestData {
            data_contract,
            mut documents,
        } = setup_test();
        let mut document = documents.remove(9);
        document
            .data
            .remove("lastName")
            .expect("lastName property should exist and be removed");

        let documents_for_transition = vec![document];
        let raw_document_transitions =
            get_document_transitions_fixture([(Action::Create, documents_for_transition)])
                .into_iter()
                .map(|dt| {
                    dt.to_object()
                        .expect("the transition should be converted to object")
                });
        let result = validate_partial_compound_indices(raw_document_transitions, &data_contract)
            .expect("should return validation result");

        let basic_error = get_basic_error(&result, 0);

        assert!(!result.is_valid());
        assert_eq!(1021, result.errors[0].code());
        assert!(
            matches!(basic_error, BasicError::InconsistentCompoundIndexDataError { index_properties, document_type } if {
                document_type  == "optionalUniqueIndexedDocument" &&
                index_properties ==  &vec!["$ownerId".to_string(), "firstName".to_string(), "lastName".to_string()]
            })
        )
    }

    #[test]
    fn should_return_valid_result_if_compound_index_contains_nof_fields() {
        let TestData {
            data_contract,
            mut documents,
        } = setup_test();
        let mut document = documents.remove(8);
        document.data = json!({});

        let documents_for_transition = vec![document];
        let raw_document_transitions =
            get_document_transitions_fixture([(Action::Create, documents_for_transition)])
                .into_iter()
                .map(|dt| {
                    dt.to_object()
                        .expect("the transition should be converted to object")
                });
        let result = validate_partial_compound_indices(raw_document_transitions, &data_contract)
            .expect("should return validation result");
        assert!(result.is_valid());
    }

    #[test]
    fn should_return_valid_result_if_compound_index_contains_all_fields() {
        let TestData {
            data_contract,
            mut documents,
        } = setup_test();
        let document = documents.remove(8);
        let documents_for_transition = vec![document];
        let raw_document_transitions =
            get_document_transitions_fixture([(Action::Create, documents_for_transition)])
                .into_iter()
                .map(|dt| {
                    dt.to_object()
                        .expect("the transition should be converted to object")
                });
        let result = validate_partial_compound_indices(raw_document_transitions, &data_contract)
            .expect("should return validation result");
        assert!(result.is_valid());
    }
}
