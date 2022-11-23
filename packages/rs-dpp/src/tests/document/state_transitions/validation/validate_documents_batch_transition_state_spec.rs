use std::time::Duration;

use anyhow::anyhow;
use chrono::Utc;
use dashcore::BlockHeader;
use serde_json::{json, Value as JsonValue};

use crate::{
    codes::ErrorWithCode,
    consensus::ConsensusError,
    data_contract::DataContract,
    document::{
        Document,
        document_transition::{Action, DocumentTransition, DocumentTransitionObjectLike},
        DocumentsBatchTransition, state_transition::documents_batch_transition::validation::state::validate_documents_batch_transition_state::*,
    },
    prelude::Identifier,
    prelude::ProtocolError,
    state_repository::MockStateRepositoryLike,
    StateError,
    tests::{
        fixtures::{
            get_data_contract_fixture, get_document_transitions_fixture, get_documents_fixture,
        },
        utils::{generate_random_identifier_struct, new_block_header},
    },
    validation::ValidationResult, state_transition::StateTransitionLike,
};

struct TestData {
    owner_id: Identifier,
    data_contract: DataContract,
    documents: Vec<Document>,
    document_transitions: Vec<DocumentTransition>,
    state_transition: DocumentsBatchTransition,
    state_repository_mock: MockStateRepositoryLike,
}

fn init() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .try_init();
}

fn setup_test() -> TestData {
    init();
    let owner_id = generate_random_identifier_struct();
    let data_contract = get_data_contract_fixture(Some(owner_id.clone()));
    let documents = get_documents_fixture(data_contract.clone()).unwrap();

    let document_transitions =
        get_document_transitions_fixture([(Action::Create, documents.clone())]);
    let raw_document_transitions: Vec<JsonValue> = document_transitions
        .iter()
        .map(|dt| dt.to_object().unwrap())
        .collect();
    let owner_id_bytes = owner_id.to_buffer();
    let state_transition = DocumentsBatchTransition::from_raw_object(
        json!({
            "ownerId" : owner_id_bytes,
            "transitions" : raw_document_transitions,
        }),
        vec![data_contract.clone()],
    )
    .expect("documents batch state transition should be created");

    let mut state_repository_mock = MockStateRepositoryLike::default();
    let data_contract_to_return = data_contract.clone();
    state_repository_mock
        .expect_fetch_data_contract::<DataContract>()
        .returning(move |_, _| Ok(data_contract_to_return.clone()));

    state_repository_mock
        .expect_fetch_latest_platform_block_header::<BlockHeader>()
        .returning(|| Ok(new_block_header(Some(Utc::now().timestamp() as u32))));

    TestData {
        owner_id,
        data_contract,
        document_transitions,
        documents,
        state_transition,
        state_repository_mock,
    }
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

fn set_updated_at(dt: &mut DocumentTransition, ts: Option<i64>) {
    match dt {
        DocumentTransition::Create(ref mut t) => t.updated_at = ts,
        DocumentTransition::Replace(ref mut t) => t.updated_at = ts,
        DocumentTransition::Delete(ref mut _t) => {}
    }
}

fn set_created_at(dt: &mut DocumentTransition, ts: Option<i64>) {
    match dt {
        DocumentTransition::Create(ref mut t) => t.created_at = ts,
        DocumentTransition::Replace(ref mut _t) => {}
        DocumentTransition::Delete(ref mut _t) => {}
    }
}
#[tokio::test]
async fn should_throw_error_if_data_contract_was_not_found() {
    let TestData {
        data_contract,
        owner_id,
        document_transitions,
        ..
    } = setup_test();
    let mut state_repository_mock = MockStateRepositoryLike::default();
    state_repository_mock
        .expect_fetch_data_contract::<DataContract>()
        .returning(|_, _| Err(anyhow!("no document found")));

    let error = validate_document_transitions(
        &state_repository_mock,
        &data_contract.id,
        &owner_id,
        document_transitions,
        &Default::default(),
    )
    .await
    .expect_err("protocol error expected");

    assert!(matches!(
        error,
        ProtocolError::DataContractNotPresentError { data_contract_id } if
        data_contract_id == data_contract.id
    ));
}

#[tokio::test]
async fn should_return_invalid_result_if_document_transition_with_action_delete_is_not_present() {
    let TestData {
        data_contract,
        owner_id,
        documents,
        mut state_repository_mock,
        ..
    } = setup_test();
    let document_transitions = get_document_transitions_fixture([
        (Action::Create, vec![]),
        (Action::Delete, vec![documents[0].clone()]),
    ]);
    let transition_id = document_transitions[0].base().id.clone();

    let owner_id_bytes = owner_id.to_buffer();
    let raw_document_transitions: Vec<JsonValue> = document_transitions
        .into_iter()
        .map(|dt| dt.to_object().unwrap())
        .collect();

    let state_transition = DocumentsBatchTransition::from_raw_object(
        json!({
            "ownerId" : owner_id_bytes,
            "transitions": raw_document_transitions}),
        vec![data_contract.clone()],
    )
    .expect("documents batch state transition should be created");

    state_repository_mock
        .expect_fetch_documents::<Document>()
        .returning(move |_, _, _, _| Ok(vec![]));

    let validation_result =
        validate_document_batch_transition_state(&state_repository_mock, &state_transition)
            .await
            .expect("validation result should be returned");

    let state_error = get_state_error(&validation_result, 0);
    assert_eq!(4005, state_error.get_code());
    assert!(matches!(
        state_error,
        StateError::DocumentNotFoundError { document_id } if document_id == &transition_id
    ));
}

#[tokio::test]
async fn should_return_invalid_result_if_document_transition_with_action_replace_has_wrong_revision(
) {
    let TestData {
        data_contract,
        owner_id,
        mut documents,
        mut state_repository_mock,
        ..
    } = setup_test();
    let mut replace_document = Document::from_raw_document(
        documents[0].to_object(false).unwrap(),
        data_contract.clone(),
    )
    .expect("document should be created");
    replace_document.revision = 3;

    documents[0].created_at = replace_document.created_at;

    let document_transitions = get_document_transitions_fixture([
        (Action::Create, vec![]),
        (Action::Replace, vec![replace_document]),
    ]);
    let transition_id = document_transitions[0].base().id.clone();

    let raw_document_transitions: Vec<JsonValue> = document_transitions
        .into_iter()
        .map(|dt| dt.to_object().unwrap())
        .collect();

    let state_transition = DocumentsBatchTransition::from_raw_object(
        json!({
            "ownerId" : owner_id.to_buffer(),
            "contractId" : data_contract.id.to_buffer(),
            "transitions": raw_document_transitions}),
        vec![data_contract.clone()],
    )
    .expect("documents batch state transition should be created");

    state_repository_mock
        .expect_fetch_documents()
        .returning(move |_, _, _, _| Ok(vec![documents[0].clone()]));

    let validation_result =
        validate_document_batch_transition_state(&state_repository_mock, &state_transition)
            .await
            .expect("validation result should be returned");
    let state_error = get_state_error(&validation_result, 0);

    assert_eq!(4010, state_error.get_code());
    assert!(matches!(
        state_error,
        StateError::InvalidDocumentRevisionError { document_id, current_revision }  if  {
            document_id == &transition_id &&
            current_revision == &1
        }
    ));
}

#[tokio::test]
async fn should_return_invalid_result_if_document_transition_with_action_replace_has_mismatch_of_owner_id(
) {
    let TestData {
        data_contract,
        owner_id,
        documents,
        mut state_repository_mock,
        ..
    } = setup_test();
    let mut replace_document = Document::from_raw_document(
        documents[0].to_object(false).unwrap(),
        data_contract.clone(),
    )
    .expect("document should be created");
    replace_document.revision = 1;

    let mut fetched_document = Document::from_raw_document(
        documents[0].to_object(false).unwrap(),
        data_contract.clone(),
    )
    .expect("document should be created");
    let another_owner_id = generate_random_identifier_struct();
    fetched_document.owner_id = another_owner_id.clone();

    let document_transitions = get_document_transitions_fixture([
        (Action::Create, vec![]),
        (Action::Replace, vec![replace_document]),
    ]);
    let transition_id = document_transitions[0].base().id.clone();

    let raw_document_transitions: Vec<JsonValue> = document_transitions
        .into_iter()
        .map(|dt| dt.to_object().unwrap())
        .collect();

    let state_transition = DocumentsBatchTransition::from_raw_object(
        json!({
            "ownerId" : owner_id.to_buffer(),
            "contractId" : data_contract.id.to_buffer(),
            "transitions": raw_document_transitions}),
        vec![data_contract.clone()],
    )
    .expect("documents batch state transition should be created");

    state_repository_mock
        .expect_fetch_documents()
        .returning(move |_, _, _, _| Ok(vec![fetched_document.clone()]));

    let validation_result =
        validate_document_batch_transition_state(&state_repository_mock, &state_transition)
            .await
            .expect("validation result should be returned");
    let state_error = get_state_error(&validation_result, 0);
    assert_eq!(4006, state_error.get_code());
    assert!(matches!(
        state_error,
        StateError::DocumentOwnerMismatchError { document_id, document_owner_id, existing_document_owner_id } if  {
            document_id == &transition_id &&
            existing_document_owner_id == &another_owner_id &&
            document_owner_id ==  &owner_id

        }
    ));
}

#[tokio::test]
#[ignore = "the action is correct. It uses enums"]
async fn should_throw_an_error_if_document_transition_has_invalid_actions() {
    unimplemented!()
}

#[tokio::test]
#[ignore = "unable to mock unique indices validator"]
async fn should_return_invalid_result_if_there_are_duplicate_document_transitions_according_to_unique_indices(
) {
    unimplemented!()
}

#[tokio::test]
async fn should_return_invalid_result_if_timestamps_mismatch() {
    let TestData {
        data_contract,
        owner_id,
        documents,
        mut state_repository_mock,
        ..
    } = setup_test();

    let document_transitions =
        get_document_transitions_fixture([(Action::Create, vec![documents[0].clone()])]);
    let transition_id = document_transitions[0].base().id.clone();
    let raw_document_transitions: Vec<JsonValue> = document_transitions
        .into_iter()
        .map(|dt| dt.to_object().unwrap())
        .collect();
    let mut state_transition = DocumentsBatchTransition::from_raw_object(
        json!({
            "ownerId" : owner_id.to_buffer(),
            "contractId" : data_contract.id.to_buffer(),
            "transitions": raw_document_transitions}),
        vec![data_contract.clone()],
    )
    .expect("documents batch state transition should be created");

    let now_ts = Utc::now().timestamp_millis();
    state_transition
        .transitions
        .iter_mut()
        .for_each(|t| set_updated_at(t, Some(now_ts)));

    state_repository_mock
        .expect_fetch_documents::<Document>()
        .returning(move |_, _, _, _| Ok(vec![]));

    let validation_result =
        validate_document_batch_transition_state(&state_repository_mock, &state_transition)
            .await
            .expect("validation result should be returned");

    let state_error = get_state_error(&validation_result, 0);
    assert_eq!(4007, state_error.get_code());
    assert!(matches!(
        state_error,
        StateError::DocumentTimestampMismatchError { document_id }  if  {
            document_id == &transition_id
        }
    ));
}

#[tokio::test]
async fn should_return_invalid_result_if_crated_at_has_violated_time_window() {
    let TestData {
        data_contract,
        owner_id,
        documents,
        mut state_repository_mock,
        ..
    } = setup_test();

    let document_transitions =
        get_document_transitions_fixture([(Action::Create, vec![documents[0].clone()])]);
    let transition_id = document_transitions[0].base().id.clone();
    let raw_document_transitions: Vec<JsonValue> = document_transitions
        .into_iter()
        .map(|dt| dt.to_object().unwrap())
        .collect();
    let mut state_transition = DocumentsBatchTransition::from_raw_object(
        json!({
            "ownerId" : owner_id.to_buffer(),
            "contractId" : data_contract.id.to_buffer(),
            "transitions": raw_document_transitions}),
        vec![data_contract.clone()],
    )
    .expect("documents batch state transition should be created");

    let now_ts_minus_6_mins =
        Utc::now().timestamp_millis() - Duration::from_secs(60 * 6).as_millis() as i64;
    state_transition
        .transitions
        .iter_mut()
        .for_each(|t| set_created_at(t, Some(now_ts_minus_6_mins)));

    state_repository_mock
        .expect_fetch_documents::<Document>()
        .returning(move |_, _, _, _| Ok(vec![]));

    let validation_result =
        validate_document_batch_transition_state(&state_repository_mock, &state_transition)
            .await
            .expect("validation result should be returned");

    let state_error = get_state_error(&validation_result, 0);
    assert_eq!(4008, state_error.get_code());
    assert!(matches!(
        state_error,
        StateError::DocumentTimestampWindowViolationError { timestamp_name, document_id, .. }   if  {
            document_id == &transition_id &&
            timestamp_name == "createdAt"
        }
    ));
}

#[tokio::test]
async fn should_not_validate_time_in_block_window_on_dry_run() {
    let TestData {
        data_contract,
        owner_id,
        documents,
        mut state_repository_mock,
        ..
    } = setup_test();

    let document_transitions =
        get_document_transitions_fixture([(Action::Create, vec![documents[0].clone()])]);
    let raw_document_transitions: Vec<JsonValue> = document_transitions
        .into_iter()
        .map(|dt| dt.to_object().unwrap())
        .collect();
    let mut state_transition = DocumentsBatchTransition::from_raw_object(
        json!({
            "ownerId" : owner_id.to_buffer(),
            "contractId" : data_contract.id.to_buffer(),
            "transitions": raw_document_transitions}),
        vec![data_contract.clone()],
    )
    .expect("documents batch state transition should be created");

    state_transition.get_execution_context().enable_dry_run();
    let now_ts_minus_6_mins =
        Utc::now().timestamp_millis() - Duration::from_secs(60 * 6).as_millis() as i64;
    state_transition
        .transitions
        .iter_mut()
        .for_each(|t| set_created_at(t, Some(now_ts_minus_6_mins)));

    state_repository_mock
        .expect_fetch_documents::<Document>()
        .returning(move |_, _, _, _| Ok(vec![]));

    let result =
        validate_document_batch_transition_state(&state_repository_mock, &state_transition)
            .await
            .expect("validation result should be returned");

    assert!(result.is_valid());
}

#[tokio::test]
async fn should_return_invalid_result_if_updated_at_has_violated_time_window() {
    let TestData {
        data_contract,
        owner_id,
        documents,
        mut state_repository_mock,
        ..
    } = setup_test();

    let document_transitions =
        get_document_transitions_fixture([(Action::Create, vec![documents[1].clone()])]);
    let transition_id = document_transitions[0].base().id.clone();
    let raw_document_transitions: Vec<JsonValue> = document_transitions
        .into_iter()
        .map(|dt| dt.to_object().unwrap())
        .collect();
    let mut state_transition = DocumentsBatchTransition::from_raw_object(
        json!({
            "ownerId" : owner_id.to_buffer(),
            "contractId" : data_contract.id.to_buffer(),
            "transitions": raw_document_transitions}),
        vec![data_contract.clone()],
    )
    .expect("documents batch state transition should be created");

    let now_ts_minus_6_mins =
        Utc::now().timestamp_millis() - Duration::from_secs(60 * 6).as_millis() as i64;
    state_transition.transitions.iter_mut().for_each(|t| {
        set_updated_at(t, Some(now_ts_minus_6_mins));
        set_created_at(t, None);
    });

    state_repository_mock
        .expect_fetch_documents::<Document>()
        .returning(move |_, _, _, _| Ok(vec![]));

    let validation_result =
        validate_document_batch_transition_state(&state_repository_mock, &state_transition)
            .await
            .expect("validation result should be returned");

    let state_error = get_state_error(&validation_result, 0);
    assert_eq!(4008, state_error.get_code());
    assert!(matches!(
        state_error,
        StateError::DocumentTimestampWindowViolationError { timestamp_name, document_id, .. }   if  {
            document_id == &transition_id &&
            timestamp_name == "updatedAt"
        }
    ));
}

#[tokio::test]
async fn should_return_valid_result_if_document_transitions_are_valid() {
    let TestData {
        data_contract,
        owner_id,
        documents,
        mut state_repository_mock,
        ..
    } = setup_test();
    let mut fetched_document_1 = Document::from_raw_document(
        documents[1].to_object(false).unwrap(),
        data_contract.clone(),
    )
    .unwrap();
    let mut fetched_document_2 = Document::from_raw_document(
        documents[2].to_object(false).unwrap(),
        data_contract.clone(),
    )
    .unwrap();
    fetched_document_1.revision = 1;
    fetched_document_2.revision = 1;
    fetched_document_1.owner_id = owner_id.clone();
    fetched_document_2.owner_id = owner_id.clone();

    state_repository_mock
        .expect_fetch_documents::<Document>()
        .returning(move |_, _, _, _| {
            Ok(vec![fetched_document_1.clone(), fetched_document_2.clone()])
        });
    let document_transitions = get_document_transitions_fixture([
        (Action::Create, vec![]),
        (Action::Replace, vec![documents[1].clone()]),
        (Action::Delete, vec![documents[2].clone()]),
    ]);
    let raw_document_transitions: Vec<JsonValue> = document_transitions
        .into_iter()
        .map(|dt| dt.to_object().unwrap())
        .collect();
    let state_transition = DocumentsBatchTransition::from_raw_object(
        json!({
            "ownerId" : owner_id.to_buffer(),
            "contractId" : data_contract.id.to_buffer(),
            "transitions": raw_document_transitions}),
        vec![data_contract.clone()],
    )
    .expect("documents batch state transition should be created");

    let validation_result =
        validate_document_batch_transition_state(&state_repository_mock, &state_transition)
            .await
            .expect("validation result should be returned");
    assert!(validation_result.is_valid());
}
