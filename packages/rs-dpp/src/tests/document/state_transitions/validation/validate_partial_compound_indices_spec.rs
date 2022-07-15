use crate::{
		validation::ValidationResult,
    consensus::{basic::BasicError, ConsensusError},
    data_contract::DataContract,
    document::{
					state_transition::documents_batch_transition::validation::basic::validate_partial_compound_indices::*,
        document_transition::{Action, DocumentTransitionObjectLike},
        Document,
    },
    tests::fixtures::{
        get_data_contract_fixture, get_document_transitions_fixture, get_documents_fixture,
    },
    util::json_value::JsonValueExt,
};
use serde_json::{json, Value as JsonValue};

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
    let raw_document_transitions: Vec<JsonValue> =
        get_document_transitions_fixture([(Action::Create, documents_for_transition)])
            .into_iter()
            .map(|dt| {
                dt.to_object()
                    .expect("the transition should be converted to object")
            })
            .collect();
    let result = validate_partial_compound_indices(raw_document_transitions.iter(), &data_contract)
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
    let raw_document_transitions: Vec<JsonValue> =
        get_document_transitions_fixture([(Action::Create, documents_for_transition)])
            .into_iter()
            .map(|dt| {
                dt.to_object()
                    .expect("the transition should be converted to object")
            })
            .collect();
    let result = validate_partial_compound_indices(raw_document_transitions.iter(), &data_contract)
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
    let raw_document_transitions: Vec<JsonValue> =
        get_document_transitions_fixture([(Action::Create, documents_for_transition)])
            .into_iter()
            .map(|dt| {
                dt.to_object()
                    .expect("the transition should be converted to object")
            })
            .collect();
    let result = validate_partial_compound_indices(raw_document_transitions.iter(), &data_contract)
        .expect("should return validation result");
    assert!(result.is_valid());
}

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
