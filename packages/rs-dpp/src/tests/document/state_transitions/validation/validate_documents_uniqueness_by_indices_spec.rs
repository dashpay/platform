use mockall::predicate;
use serde_json::json;

use crate::{
    consensus::ConsensusError,
    data_contract::DataContract,
    document::{
        Document,
        document_transition::{Action, DocumentTransition},
        state_transition::documents_batch_transition::validation::state::validate_documents_uniqueness_by_indices::*,
    },
    prelude::Identifier,
    state_repository::MockStateRepositoryLike,
    StateError,
    tests::{
        fixtures::{
            get_data_contract_fixture, get_document_transitions_fixture, get_documents_fixture,
        },
        utils::generate_random_identifier_struct,
    },
    util::string_encoding::Encoding,
    validation::ValidationResult,
};

struct TestData {
    owner_id: Identifier,
    data_contract: DataContract,
    documents: Vec<Document>,
    document_transitions: Vec<DocumentTransition>,
}

fn setup_test() -> TestData {
    let owner_id = generate_random_identifier_struct();
    let data_contract = get_data_contract_fixture(Some(owner_id.clone()));
    let documents = get_documents_fixture(data_contract.clone()).unwrap();

    TestData {
        owner_id,
        data_contract,
        document_transitions: get_document_transitions_fixture([(
            Action::Create,
            documents.clone(),
        )]),
        documents,
    }
}

#[tokio::test]
async fn should_return_valid_result_if_documents_have_no_unique_indices() {
    let TestData {
        owner_id,
        data_contract,
        documents,
        ..
    } = setup_test();
    let mut state_repository_mock = MockStateRepositoryLike::default();
    state_repository_mock
        .expect_fetch_documents::<Document>()
        .returning(|_, _, _, _| Ok(vec![]));

    let document_transitions =
        get_document_transitions_fixture([(Action::Create, vec![documents[0].clone()])]);
    let validation_result = validate_documents_uniqueness_by_indices(
        &state_repository_mock,
        &owner_id,
        &document_transitions,
        &data_contract,
        &Default::default(),
    )
    .await
    .expect("validation result should be returned");
    assert!(validation_result.is_valid())
}

#[tokio::test]
async fn should_return_valid_result_if_document_has_unique_indices_and_there_are_no_duplicates() {
    let TestData {
        owner_id,
        data_contract,
        documents,
        ..
    } = setup_test();
    let william_doc = documents[3].clone();
    let owner_id_base58 = owner_id.to_string(Encoding::Base58);
    let mut state_repository_mock = MockStateRepositoryLike::default();
    let document_transitions =
        get_document_transitions_fixture([(Action::Create, vec![william_doc.clone()])]);
    let expect_document = william_doc.to_owned();

    state_repository_mock
        .expect_fetch_documents::<Document>()
        .with(
            predicate::eq(data_contract.id.clone()),
            predicate::eq("indexedDocument"),
            predicate::eq(json!({
               "where" : [
                ["$ownerId", "==", owner_id_base58 ],
                ["firstName", "==", william_doc.get("firstName").unwrap()],
               ],
            })),
            predicate::always(),
        )
        .returning(move |_, _, _, _| Ok(vec![expect_document.clone()]));

    let expect_document = william_doc.to_owned();
    state_repository_mock
        .expect_fetch_documents::<Document>()
        .with(
            predicate::eq(data_contract.id.clone()),
            predicate::eq("indexedDocument"),
            predicate::eq(json!({
               "where" : [
                ["$ownerId", "==", owner_id_base58 ],
                ["lastName", "==", william_doc.get("lastName").unwrap()],
               ],
            })),
            predicate::always(),
        )
        .returning(move |_, _, _, _| Ok(vec![expect_document.clone()]));

    let validation_result = validate_documents_uniqueness_by_indices(
        &state_repository_mock,
        &owner_id,
        &document_transitions,
        &data_contract,
        &Default::default(),
    )
    .await
    .expect("validation result should be returned");
    assert!(validation_result.is_valid())
}

#[tokio::test]
async fn should_return_invalid_result_if_document_has_unique_indices_and_there_are_duplicates() {
    let TestData {
        owner_id,
        data_contract,
        documents,
        ..
    } = setup_test();
    let william_doc = documents[3].clone();
    let leon_doc = documents[4].clone();
    let owner_id_base58 = owner_id.to_string(Encoding::Base58);
    let mut state_repository_mock = MockStateRepositoryLike::default();
    let document_transitions = get_document_transitions_fixture([(
        Action::Create,
        vec![william_doc.clone(), leon_doc.clone()],
    )]);

    let expect_document = leon_doc.to_owned();
    state_repository_mock
        .expect_fetch_documents::<Document>()
        .with(
            predicate::eq(data_contract.id.clone()),
            predicate::eq("indexedDocument"),
            predicate::eq(json!({
               "where" : [
                ["$ownerId", "==", owner_id_base58 ],
                ["firstName", "==", william_doc.get("firstName").unwrap()],
               ],
            })),
            predicate::always(),
        )
        .returning(move |_, _, _, _| Ok(vec![expect_document.clone()]));

    let expect_document = leon_doc.to_owned();
    state_repository_mock
        .expect_fetch_documents::<Document>()
        .with(
            predicate::eq(data_contract.id.clone()),
            predicate::eq("indexedDocument"),
            predicate::eq(json!({
               "where" : [
                ["$ownerId", "==", owner_id_base58 ],
                ["lastName", "==", william_doc.get("lastName").unwrap()],
               ],
            })),
            predicate::always(),
        )
        .returning(move |_, _, _, _| Ok(vec![expect_document.clone()]));

    let expect_document = william_doc.to_owned();
    state_repository_mock
        .expect_fetch_documents::<Document>()
        .with(
            predicate::eq(data_contract.id.clone()),
            predicate::eq("indexedDocument"),
            predicate::eq(json!({
               "where" : [
                ["$ownerId", "==", owner_id_base58 ],
                ["firstName", "==", leon_doc.get("firstName").unwrap()],
               ],
            })),
            predicate::always(),
        )
        .returning(move |_, _, _, _| Ok(vec![expect_document.clone()]));

    let expect_document = william_doc.to_owned();
    state_repository_mock
        .expect_fetch_documents::<Document>()
        .with(
            predicate::eq(data_contract.id.clone()),
            predicate::eq("indexedDocument"),
            predicate::eq(json!({
               "where" : [
                ["$ownerId", "==", owner_id_base58 ],
                ["lastName", "==", leon_doc.get("lastName").unwrap()],
               ],
            })),
            predicate::always(),
        )
        .returning(move |_, _, _, _| Ok(vec![expect_document.clone()]));

    let validation_result = validate_documents_uniqueness_by_indices(
        &state_repository_mock,
        &owner_id,
        &document_transitions,
        &data_contract,
        &Default::default(),
    )
    .await
    .expect("validation result should be returned");
    assert!(!validation_result.is_valid());

    assert_eq!(4, validation_result.errors.len());
    assert_eq!(4009, validation_result.errors[0].code());

    let state_error_1 = get_state_error(&validation_result, 0);
    assert!(matches!(
        state_error_1,
        StateError::DuplicateUniqueIndexError { document_id, .. } if  document_id == &document_transitions[0].base().id
    ));
    let state_error_3 = get_state_error(&validation_result, 2);
    assert!(matches!(
        state_error_3 ,
        StateError::DuplicateUniqueIndexError { document_id, .. } if  document_id == &document_transitions[1].base().id
    ));
}

#[tokio::test]
async fn should_return_valid_result_if_document_has_undefined_field_from_index() {
    let TestData {
        owner_id,
        data_contract,
        documents,
        ..
    } = setup_test();
    let indexed_document = documents[7].clone();
    let document_transitions =
        get_document_transitions_fixture([(Action::Create, vec![indexed_document.clone()])]);
    let owner_id_base58 = owner_id.to_string(Encoding::Base58);
    let mut state_repository_mock = MockStateRepositoryLike::default();

    let expect_document = indexed_document.to_owned();
    state_repository_mock
        .expect_fetch_documents::<Document>()
        .with(
            predicate::eq(data_contract.id.clone()),
            predicate::eq("indexedDocument"),
            predicate::eq(json!({
               "where" : [
                ["$ownerId", "==", owner_id_base58 ],
                ["firstName", "==", indexed_document.get("firstName").unwrap()],
               ],
            })),
            predicate::always(),
        )
        .returning(move |_, _, _, _| Ok(vec![expect_document.clone()]));

    let expect_document = indexed_document.to_owned();
    state_repository_mock
        .expect_fetch_documents::<Document>()
        .with(
            predicate::eq(data_contract.id.clone()),
            predicate::eq("indexedDocument"),
            predicate::eq(json!({
               "where" : [
                ["$ownerId", "==", owner_id_base58 ],
                ["lastName", "==", indexed_document.get("lastName").unwrap()],
               ],
            })),
            predicate::always(),
        )
        .returning(move |_, _, _, _| Ok(vec![expect_document.clone()]));

    let validation_result = validate_documents_uniqueness_by_indices(
        &state_repository_mock,
        &owner_id,
        &document_transitions,
        &data_contract,
        &Default::default(),
    )
    .await
    .expect("validation result should be returned");
    assert!(validation_result.is_valid());
}

#[tokio::test]
async fn should_return_valid_result_if_document_being_created_and_has_created_at_and_updated_at_indices(
) {
    let TestData {
        owner_id,
        data_contract,
        documents,
        ..
    } = setup_test();
    let unique_dates_doc = documents[6].clone();
    let document_transitions =
        get_document_transitions_fixture([(Action::Create, vec![unique_dates_doc.clone()])]);
    let mut state_repository_mock = MockStateRepositoryLike::default();

    let expect_document = unique_dates_doc.to_owned();
    state_repository_mock
            .expect_fetch_documents::<Document>()
            .with(
                predicate::eq(data_contract.id.clone()),
                predicate::eq("uniqueDates"),
                predicate::eq(json!({
                   "where" : [
                    ["$createdAt", "==", unique_dates_doc.created_at.expect("createdAt should be present") ],
                    ["$updatedAt", "==", unique_dates_doc.created_at.expect("createdAt should be present") ],
                   ],
                })),
               predicate::always(),
            )
            .returning(move |_, _, _, _| Ok(vec![expect_document.clone()]));

    let validation_result = validate_documents_uniqueness_by_indices(
        &state_repository_mock,
        &owner_id,
        &document_transitions,
        &data_contract,
        &Default::default(),
    )
    .await
    .expect("validation result should be returned");
    assert!(validation_result.is_valid());
}

fn get_state_error(result: &ValidationResult<()>, error_number: usize) -> &StateError {
    match result
        .errors
        .get(error_number)
        .expect("error should be found")
    {
        ConsensusError::StateError(state_error) => &*state_error,
        _ => panic!(
            "error '{:?}' isn't a basic error",
            result.errors[error_number]
        ),
    }
}
