use platform_value::Value;

use crate::{
    consensus::{basic::BasicError, ConsensusError},
    data_contract::DataContract,
    document::{
        document_transition::{Action, DocumentTransitionObjectLike},
        state_transition::documents_batch_transition::validation::basic::validate_partial_compound_indices::*,
    },
    tests::fixtures::{
        get_data_contract_fixture, get_document_transitions_fixture,
    },
    validation::ValidationResult,
};
use crate::document::ExtendedDocument;
use crate::tests::fixtures::get_extended_documents_fixture;

struct TestData {
    data_contract: DataContract,
    documents: Vec<ExtendedDocument>,
}

fn setup_test() -> TestData {
    let data_contract = get_data_contract_fixture(None);
    let documents =
        get_extended_documents_fixture(data_contract.clone()).expect("documents should be created");

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
        .properties_as_mut()
        .remove("lastName")
        .expect("lastName property should exist and be removed");

    let documents_for_transition = vec![document];
    let raw_document_transitions: Vec<Value> =
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
    assert_eq!(1021, result.consensus_errors[0].code());
    match basic_error {
        BasicError::InconsistentCompoundIndexDataError(err) => {
            assert_eq!(
                err.document_type(),
                "optionalUniqueIndexedDocument".to_string()
            );
            assert_eq!(
                err.index_properties(),
                vec![
                    "$ownerId".to_string(),
                    "firstName".to_string(),
                    "lastName".to_string()
                ]
            );
        }
        _ => panic!(
            "Expected InconsistentCompoundIndexDataError, got {}",
            basic_error
        ),
    }
}

#[test]
fn should_return_valid_result_if_compound_index_contains_nof_fields() {
    let TestData {
        data_contract,
        mut documents,
    } = setup_test();
    let mut document = documents.remove(8);
    document.properties_as_mut().clear();

    let documents_for_transition = vec![document];
    let raw_document_transitions: Vec<Value> =
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
    let raw_document_transitions: Vec<Value> =
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

fn get_basic_error(result: &ValidationResult<()>, error_number: usize) -> &BasicError {
    match result
        .consensus_errors
        .get(error_number)
        .expect("error should be found")
    {
        ConsensusError::BasicError(basic_error) => basic_error,
        _ => panic!(
            "error '{:?}' isn't a basic error",
            result.consensus_errors[error_number]
        ),
    }
}
