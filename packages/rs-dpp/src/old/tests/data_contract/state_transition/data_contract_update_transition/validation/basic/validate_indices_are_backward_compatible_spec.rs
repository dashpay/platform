use crate::{
    consensus::basic::BasicError,
    data_contract::state_transition::data_contract_update_transition::validation::basic::validate_indices_are_backward_compatible,
};
use std::collections::BTreeMap;

use crate::consensus::codes::ErrorWithCode;
use crate::validation::SimpleConsensusValidationResult;
use crate::{
    consensus::ConsensusError, data_contract::DataContract,
    tests::fixtures::get_data_contract_fixture, util::json_value::JsonValueExt,
};
use serde_json::json;

type JsonSchema = serde_json::Value;

struct TestData {
    old_data_contract: DataContract,
    new_data_contract: DataContract,
    old_documents_schema: BTreeMap<String, JsonSchema>,
    new_documents_schema: BTreeMap<String, JsonSchema>,
}

fn setup_test() -> TestData {
    let old_data_contract = get_data_contract_fixture(None).data_contract;
    let mut new_data_contract = old_data_contract.clone();

    let mut indexed_document = new_data_contract
        .get_document_schema("indexedDocument")
        .expect("document should exist")
        .to_owned();

    indexed_document["properties"]["otherName"] = json!({
        "type" : "string"
    });
    indexed_document["indices"]
        .push(json!({
            "name" : "index42",
            "unique": false,
            "properties" : [
                { "otherName" : "asc"}
            ]
        }))
        .expect("the unique index should be added to the document");
    indexed_document["indices"]
        .push(json!({
            "name" : "index42",
            "properties" : [
                { "otherName" : "asc"}
            ]
        }))
        .expect("the non-unique index should be added to the document");
    new_data_contract.set_document_schema(String::from("indexedDocument"), indexed_document);
    let old_documents_schema = old_data_contract.documents.to_owned();
    let new_documents_schema = new_data_contract.documents.to_owned();

    TestData {
        old_data_contract,
        new_data_contract,
        old_documents_schema,
        new_documents_schema,
    }
}

fn get_basic_error(result: &SimpleConsensusValidationResult, error_number: usize) -> &BasicError {
    match result
        .errors
        .get(error_number)
        .expect("error should be found")
    {
        ConsensusError::BasicError(basic_error) => basic_error,
        _ => panic!(
            "error '{:?}' isn't a basic error",
            result.errors[error_number]
        ),
    }
}

#[test]
fn should_return_invalid_result_if_some_of_unique_indices_have_changed() {
    let TestData {
        old_documents_schema,
        mut new_documents_schema,
        ..
    } = setup_test();

    new_documents_schema.get_mut("indexedDocument").unwrap()["indices"][0]["properties"][0] =
        json!({ "lastName": "asc" });

    let result = validate_indices_are_backward_compatible(
        old_documents_schema.iter(),
        new_documents_schema.iter(),
    )
    .expect("validation result should be returned");

    assert_eq!(1, result.errors.len());
    assert_eq!(1053, result.errors[0].code());

    let basic_error = get_basic_error(&result, 0);

    match basic_error {
        BasicError::DataContractUniqueIndicesChangedError(err) => {
            assert_eq!(err.document_type().clone(), "indexedDocument".to_string());
            assert_eq!(err.index_name().clone(), "index1".to_string());
        }
        _ => panic!(
            "Expected DataContractUniqueIndicesChangedError, got {}",
            basic_error
        ),
    }
}

#[test]
fn should_return_invalid_result_if_non_unique_index_update_failed_due_to_changed_old_properties() {
    let TestData {
        old_documents_schema,
        mut new_documents_schema,
        ..
    } = setup_test();
    new_documents_schema.get_mut("indexedDocument").unwrap()["indices"][2]["properties"][0] =
        json!({ "$id": "asc" });

    let result = validate_indices_are_backward_compatible(
        old_documents_schema.iter(),
        new_documents_schema.iter(),
    )
    .expect("validation result should be returned");

    assert_eq!(1, result.errors.len());
    assert_eq!(1054, result.errors[0].code());

    let basic_error = get_basic_error(&result, 0);

    match basic_error {
        BasicError::DataContractInvalidIndexDefinitionUpdateError(err) => {
            assert_eq!(err.document_type().clone(), "indexedDocument".to_string());
            assert_eq!(err.index_path().clone(), "index3".to_string());
        }
        _ => panic!(
            "Expected DataContractInvalidIndexDefinitionUpdateError, got {}",
            basic_error
        ),
    }
}

#[test]
fn should_return_invalid_result_if_already_indexed_properties_are_added_to_existing_index() {
    let TestData {
        old_documents_schema,
        mut new_documents_schema,
        ..
    } = setup_test();
    new_documents_schema.get_mut("indexedDocument").unwrap()["indices"][2]["properties"]
        .push(json!({ "firstName": "asc" }))
        .expect("the new index property should be added");

    let result = validate_indices_are_backward_compatible(
        old_documents_schema.iter(),
        new_documents_schema.iter(),
    )
    .expect("validation result should be returned");

    assert_eq!(1, result.errors.len());
    assert_eq!(1054, result.errors[0].code());

    let basic_error = get_basic_error(&result, 0);
    match basic_error {
        BasicError::DataContractInvalidIndexDefinitionUpdateError(err) => {
            assert_eq!(err.document_type(), "indexedDocument".to_string());
            assert_eq!(err.index_path(), "index3".to_string());
        }
        _ => panic!(
            "Expected DataContractInvalidIndexDefinitionUpdateError, got {}",
            basic_error
        ),
    }
}

#[test]
fn should_return_invalid_result_if_one_of_new_indices_contains_old_properties_in_the_wrong_order() {
    let TestData {
        old_documents_schema,
        mut new_documents_schema,
        ..
    } = setup_test();
    new_documents_schema.get_mut("indexedDocument").unwrap()["indices"]
        .push(json!({
        "name": "index_other",
        "properties": [
          { "firstName": "asc" },
          { "$ownerId": "asc" },
        ],

          }))
        .expect("the new index should be added");

    let result = validate_indices_are_backward_compatible(
        old_documents_schema.iter(),
        new_documents_schema.iter(),
    )
    .expect("validation result should be returned");

    assert_eq!(1, result.errors.len());
    assert_eq!(1054, result.errors[0].code());

    let basic_error = get_basic_error(&result, 0);

    // the JS-version imports DataContractInvalidIndexDefinitionUpdateError as DataContractHaveNewIndexWithOldPropertiesError as
    // we should decide if we want to have a separate error called DataContractInvalidIndexDefinitionUpdateError
    match basic_error {
        BasicError::DataContractInvalidIndexDefinitionUpdateError(err) => {
            assert_eq!(err.document_type(), "indexedDocument".to_string());
            assert_eq!(err.index_path(), "index_other".to_string());
        }
        _ => panic!(
            "Expected DataContractInvalidIndexDefinitionUpdateError, got {}",
            basic_error
        ),
    }
}

#[test]
fn should_return_invalid_result_if_one_of_new_indices_is_unique() {
    let TestData {
        old_documents_schema,
        mut new_documents_schema,
        ..
    } = setup_test();
    new_documents_schema.get_mut("indexedDocument").unwrap()["indices"]
        .push(json!({
            "name": "index_other",
            "properties": [
                { "otherName": "asc" },
            ],
            "unique": true,
        }))
        .expect("the new index should be added");

    let result = validate_indices_are_backward_compatible(
        old_documents_schema.iter(),
        new_documents_schema.iter(),
    )
    .expect("validation result should be returned");

    assert_eq!(1, result.errors.len());
    assert_eq!(1055, result.errors[0].code());

    let basic_error = get_basic_error(&result, 0);

    match basic_error {
        BasicError::DataContractHaveNewUniqueIndexError(err) => {
            assert_eq!(err.document_type(), "indexedDocument".to_string());
            assert_eq!(err.index_name(), "index_other".to_string());
        }
        _ => panic!(
            "Expected DataContractHaveNewUniqueIndexError, got {}",
            basic_error
        ),
    }
}

#[test]
fn should_return_valid_result_if_indices_are_not_changed() {
    let TestData {
        old_documents_schema,
        new_documents_schema,
        ..
    } = setup_test();

    let result = validate_indices_are_backward_compatible(
        old_documents_schema.iter(),
        new_documents_schema.iter(),
    )
    .expect("validation result should be returned");

    assert!(result.is_valid());
}

#[test]
fn should_return_invalid_result_if_non_unique_index_added_for_non_indexed_property() {
    let TestData {
        mut old_documents_schema,
        mut new_documents_schema,
        ..
    } = setup_test();

    // Here we add another property to old schema that certainly was not indexed by any means.
    old_documents_schema.get_mut("indexedDocument").unwrap()["properties"]
        ["oldUnindexedProperty"] = json!({
        "type": "string",
        "maxLength": "420",
    });

    new_documents_schema.get_mut("indexedDocument").unwrap()["indices"]
        .push(json!(
            {
                "name": "index1337",
                "properties": [
                    {
                        "oldUnindexedProperty": "asc",
                    },
                ],
                "unique": false,
            }
        ))
        .unwrap();
    let result = validate_indices_are_backward_compatible(
        old_documents_schema.iter(),
        new_documents_schema.iter(),
    )
    .expect("validation result should be returned");

    assert_eq!(result.errors.len(), 1);
    assert_eq!(result.errors[0].code(), 1054);

    let basic_error = get_basic_error(&result, 0);

    match basic_error {
        BasicError::DataContractInvalidIndexDefinitionUpdateError(err) => {
            assert_eq!(err.document_type(), "indexedDocument".to_string());
            assert_eq!(err.index_path(), "index1337".to_string());
        }
        _ => panic!(
            "Expected DataContractInvalidIndexDefinitionUpdateError, got {}",
            basic_error
        ),
    }
}
