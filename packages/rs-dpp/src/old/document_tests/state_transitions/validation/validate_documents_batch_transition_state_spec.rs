use std::collections::BTreeMap;
use std::time::Duration;

use chrono::Utc;
use platform_value::Value;

use crate::{
    consensus::ConsensusError,
    data_contract::DataContract,
    document::{
        document_transition::{Action, DocumentTransition, DocumentTransitionObjectLike},
        DocumentsBatchTransition, state_transition::documents_batch_transition::validation::state::validate_documents_batch_transition_state::*,
    },
    prelude::Identifier,
    prelude::ProtocolError,
    state_repository::MockStateRepositoryLike,
    tests::{
        fixtures::{
            get_data_contract_fixture, get_document_transitions_fixture,
        },
        utils::generate_random_identifier_struct,
    },
};
use crate::consensus::state::state_error::StateError;
use crate::document::{Document, ExtendedDocument};
use crate::errors::consensus::codes::ErrorWithCode;
use crate::identity::TimestampMillis;
use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use crate::tests::fixtures::get_extended_documents_fixture;
use crate::validation::ConsensusValidationResult;

struct TestData {
    owner_id: Identifier,
    data_contract: DataContract,
    extended_documents: Vec<ExtendedDocument>,
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
    let created_data_contract = get_data_contract_fixture(Some(owner_id));
    let documents =
        get_extended_documents_fixture(created_data_contract.data_contract.clone()).unwrap();

    let document_transitions = get_document_transitions_fixture([(
        DocumentTransitionActionType::Create,
        documents.clone(),
    )]);
    let raw_document_transitions: Vec<Value> = document_transitions
        .iter()
        .map(|dt| dt.to_object().unwrap())
        .collect();
    let owner_id_bytes = owner_id.to_buffer();
    let mut map = BTreeMap::new();
    map.insert("ownerId".to_string(), Value::Identifier(owner_id_bytes));
    map.insert(
        "transitions".to_string(),
        Value::Array(raw_document_transitions),
    );
    let state_transition = DocumentsBatchTransition::from_value_map(
        map,
        vec![created_data_contract.data_contract.clone()],
    )
    .expect("documents batch state transition should be created");

    let mut state_repository_mock = MockStateRepositoryLike::default();
    let data_contract_to_return = created_data_contract.data_contract.clone();
    state_repository_mock
        .expect_fetch_data_contract()
        .returning(move |_, _| Ok(Some(data_contract_to_return.clone())));

    state_repository_mock
        .expect_fetch_latest_platform_block_time()
        .returning(|| Ok(Utc::now().timestamp_millis() as u64));

    TestData {
        owner_id,
        data_contract: created_data_contract.data_contract,
        document_transitions,
        extended_documents: documents,
        state_transition,
        state_repository_mock,
    }
}

fn get_state_error<TData: Clone>(
    result: &ConsensusValidationResult<TData>,
    error_number: usize,
) -> &StateError {
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

fn set_updated_at(dt: &mut DocumentTransition, ts: Option<TimestampMillis>) {
    match dt {
        DocumentTransition::Create(ref mut t) => t.updated_at = ts,
        DocumentTransition::Replace(ref mut t) => t.updated_at = ts,
        DocumentTransition::Delete(ref mut _t) => {}
    }
}

fn set_created_at(dt: &mut DocumentTransition, ts: Option<TimestampMillis>) {
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
        .expect_fetch_data_contract()
        .returning(|_, _| Ok(None));

    let error = validate_document_transitions(
        &state_repository_mock,
        &data_contract.id(),
        owner_id,
        document_transitions.iter().collect::<Vec<_>>().as_slice(),
        &Default::default(),
    )
    .await
    .expect_err("protocol error expected");

    match error {
        ProtocolError::DataContractNotPresentError(err) => {
            assert_eq!(err.data_contract_id(), data_contract.id);
        }
        _ => panic!("Expected DataContractNotPresentError, got {}", error),
    }
}

#[tokio::test]
async fn should_return_invalid_result_if_document_transition_with_action_delete_is_not_present() {
    let TestData {
        data_contract,
        owner_id,
        extended_documents: documents,
        mut state_repository_mock,
        ..
    } = setup_test();
    let document_transitions = get_document_transitions_fixture([
        (DocumentTransitionActionType::Create, vec![]),
        (
            DocumentTransitionActionType::Delete,
            vec![documents[0].clone()],
        ),
    ]);
    let transition_id = document_transitions[0].base().id;

    let owner_id_bytes = owner_id.to_buffer();
    let raw_document_transitions: Vec<Value> = document_transitions
        .into_iter()
        .map(|dt| dt.to_object().unwrap())
        .collect();

    let mut map = BTreeMap::new();
    map.insert("ownerId".to_string(), Value::Identifier(owner_id_bytes));
    map.insert(
        "transitions".to_string(),
        Value::Array(raw_document_transitions),
    );
    let state_transition =
        DocumentsBatchTransition::from_value_map(map, vec![data_contract.clone()])
            .expect("documents batch state transition should be created");

    state_repository_mock
        .expect_fetch_documents()
        .returning(move |_, _, _, _| Ok(vec![]));

    let execution_context = StateTransitionExecutionContext::default();

    let validation_result = validate_document_batch_transition_state(
        &state_repository_mock,
        &state_transition,
        &execution_context,
    )
    .await
    .expect("validation result should be returned");

    let state_error = get_state_error(&validation_result, 0);
    assert_eq!(4005, state_error.code());
    assert!(matches!(
        state_error,
        StateError::DocumentNotFoundError(e) if e.document_id() == &transition_id
    ));
}

#[tokio::test]
async fn should_return_invalid_result_if_document_transition_with_action_replace_has_wrong_revision(
) {
    let TestData {
        data_contract,
        owner_id,
        extended_documents,
        mut state_repository_mock,
        ..
    } = setup_test();

    let mut documents = extended_documents
        .clone()
        .into_iter()
        .map(|extended_document| extended_document.document)
        .collect::<Vec<Document>>();

    let mut replace_document = ExtendedDocument::from_raw_json_document(
        extended_documents[0]
            .to_json_object_for_validation()
            .unwrap(),
        data_contract.clone(),
    )
    .expect("document should be created");
    replace_document.document.revision = Some(3);

    documents[0].created_at = replace_document.created_at().copied();

    let document_transitions = get_document_transitions_fixture([
        (DocumentTransitionActionType::Create, vec![]),
        (
            DocumentTransitionActionType::Replace,
            vec![replace_document],
        ),
    ]);
    let transition_id = document_transitions[0].base().id;

    let raw_document_transitions: Vec<Value> = document_transitions
        .into_iter()
        .map(|dt| dt.to_object().unwrap())
        .collect();

    let mut map = BTreeMap::new();
    map.insert(
        "ownerId".to_string(),
        Value::Identifier(owner_id.to_buffer()),
    );
    map.insert(
        "contractId".to_string(),
        Value::Identifier(data_contract.id.to_buffer()),
    );
    map.insert(
        "transitions".to_string(),
        Value::Array(raw_document_transitions),
    );
    let state_transition =
        DocumentsBatchTransition::from_value_map(map, vec![data_contract.clone()])
            .expect("documents batch state transition should be created");

    state_repository_mock
        .expect_fetch_documents()
        .returning(move |_, _, _, _| Ok(vec![documents[0].clone()]));

    let execution_context = StateTransitionExecutionContext::default();

    let validation_result = validate_document_batch_transition_state(
        &state_repository_mock,
        &state_transition,
        &execution_context,
    )
    .await
    .expect("validation result should be returned");
    let state_error = get_state_error(&validation_result, 0);

    assert_eq!(4010, state_error.code());
    assert!(matches!(
        state_error,
        StateError::InvalidDocumentRevisionError(e)  if  {
            e.document_id() == &transition_id &&
            e.current_revision() == &Some(1)
        }
    ));
}

#[tokio::test]
async fn should_return_invalid_result_if_document_transition_with_action_replace_has_mismatch_of_owner_id(
) {
    let TestData {
        data_contract,
        owner_id,
        extended_documents,
        mut state_repository_mock,
        ..
    } = setup_test();
    let mut replace_document = ExtendedDocument::from_raw_json_document(
        extended_documents[0]
            .to_json_object_for_validation()
            .unwrap(),
        data_contract.clone(),
    )
    .expect("document should be created");
    replace_document.document.revision = Some(1);

    let mut fetched_document = ExtendedDocument::from_raw_json_document(
        extended_documents[0]
            .to_json_object_for_validation()
            .unwrap(),
        data_contract.clone(),
    )
    .expect("document should be created");
    let another_owner_id = generate_random_identifier_struct();
    fetched_document.document.owner_id = another_owner_id;

    let document_transitions = get_document_transitions_fixture([
        (DocumentTransitionActionType::Create, vec![]),
        (
            DocumentTransitionActionType::Replace,
            vec![replace_document],
        ),
    ]);
    let transition_id = document_transitions[0].base().id;

    let raw_document_transitions: Vec<Value> = document_transitions
        .into_iter()
        .map(|dt| dt.to_object().unwrap())
        .collect();

    let mut map = BTreeMap::new();
    map.insert(
        "ownerId".to_string(),
        Value::Identifier(owner_id.to_buffer()),
    );
    map.insert(
        "contractId".to_string(),
        Value::Identifier(data_contract.id.to_buffer()),
    );
    map.insert(
        "transitions".to_string(),
        Value::Array(raw_document_transitions),
    );
    let state_transition =
        DocumentsBatchTransition::from_value_map(map, vec![data_contract.clone()])
            .expect("documents batch state transition should be created");

    state_repository_mock
        .expect_fetch_documents()
        .returning(move |_, _, _, _| Ok(vec![fetched_document.document.clone()]));

    let execution_context = StateTransitionExecutionContext::default();

    let validation_result = validate_document_batch_transition_state(
        &state_repository_mock,
        &state_transition,
        &execution_context,
    )
    .await
    .expect("validation result should be returned");
    let state_error = get_state_error(&validation_result, 0);
    assert_eq!(4006, state_error.code());
    assert!(matches!(
        state_error,
        StateError::DocumentOwnerIdMismatchError(e) if  {
            e.document_id() == &transition_id &&
            e.existing_document_owner_id() == &another_owner_id &&
            e.document_owner_id() ==  &owner_id
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
        extended_documents: documents,
        mut state_repository_mock,
        ..
    } = setup_test();

    let document_transitions = get_document_transitions_fixture([(
        DocumentTransitionActionType::Create,
        vec![documents[0].clone()],
    )]);
    let transition_id = document_transitions[0].base().id;
    let raw_document_transitions: Vec<Value> = document_transitions
        .into_iter()
        .map(|dt| dt.to_object().unwrap())
        .collect();
    let mut map = BTreeMap::new();
    map.insert(
        "ownerId".to_string(),
        Value::Identifier(owner_id.to_buffer()),
    );
    map.insert(
        "contractId".to_string(),
        Value::Identifier(data_contract.id.to_buffer()),
    );
    map.insert(
        "transitions".to_string(),
        Value::Array(raw_document_transitions),
    );
    let mut state_transition =
        DocumentsBatchTransition::from_value_map(map, vec![data_contract.clone()])
            .expect("documents batch state transition should be created");

    let now_ts = Utc::now().timestamp_millis() as u64;
    state_transition
        .transitions
        .iter_mut()
        .for_each(|t| set_updated_at(t, Some(now_ts)));

    state_repository_mock
        .expect_fetch_documents()
        .returning(move |_, _, _, _| Ok(vec![]));

    let execution_context = StateTransitionExecutionContext::default();

    let validation_result = validate_document_batch_transition_state(
        &state_repository_mock,
        &state_transition,
        &execution_context,
    )
    .await
    .expect("validation result should be returned");

    let state_error = get_state_error(&validation_result, 0);
    assert_eq!(4007, state_error.code());
    assert!(matches!(
        state_error,
        StateError::DocumentTimestampsMismatchError(e)  if  {
            e.document_id() == &transition_id
        }
    ));
}

#[tokio::test]
async fn should_return_invalid_result_if_crated_at_has_violated_time_window() {
    let TestData {
        data_contract,
        owner_id,
        extended_documents: documents,
        mut state_repository_mock,
        ..
    } = setup_test();

    let document_transitions = get_document_transitions_fixture([(
        DocumentTransitionActionType::Create,
        vec![documents[0].clone()],
    )]);
    let transition_id = document_transitions[0].base().id;
    let raw_document_transitions: Vec<Value> = document_transitions
        .into_iter()
        .map(|dt| dt.to_object().unwrap())
        .collect();
    let mut map = BTreeMap::new();
    map.insert(
        "ownerId".to_string(),
        Value::Identifier(owner_id.to_buffer()),
    );
    map.insert(
        "contractId".to_string(),
        Value::Identifier(data_contract.id.to_buffer()),
    );
    map.insert(
        "transitions".to_string(),
        Value::Array(raw_document_transitions),
    );
    let mut state_transition =
        DocumentsBatchTransition::from_value_map(map, vec![data_contract.clone()])
            .expect("documents batch state transition should be created");

    let now_ts_minus_6_mins =
        Utc::now().timestamp_millis() as u64 - Duration::from_secs(60 * 6).as_millis() as u64;
    state_transition
        .transitions
        .iter_mut()
        .for_each(|t| set_created_at(t, Some(now_ts_minus_6_mins)));

    state_repository_mock
        .expect_fetch_documents()
        .returning(move |_, _, _, _| Ok(vec![]));

    let execution_context = StateTransitionExecutionContext::default();

    let validation_result = validate_document_batch_transition_state(
        &state_repository_mock,
        &state_transition,
        &execution_context,
    )
    .await
    .expect("validation result should be returned");

    let state_error = get_state_error(&validation_result, 0);
    assert_eq!(4008, state_error.code());
    assert!(matches!(
        state_error,
        StateError::DocumentTimestampWindowViolationError(e)   if  {
            e.document_id() == &transition_id &&
            e.timestamp_name() == "createdAt"
        }
    ));
}

#[tokio::test]
async fn should_return_invalid_result_if_created_at_and_updated_at_are_equal_for_replace_transition(
) {
    let TestData {
        data_contract,
        owner_id,
        extended_documents: documents,
        mut state_repository_mock,
        ..
    } = setup_test();

    let document_transitions = get_document_transitions_fixture([
        (DocumentTransitionActionType::Create, vec![]),
        (
            DocumentTransitionActionType::Replace,
            vec![documents[0].clone()],
        ),
    ]);
    let transition_id = document_transitions[0].base().id;
    let raw_document_transitions: Vec<Value> = document_transitions
        .into_iter()
        .map(|dt| dt.to_object().unwrap())
        .collect();
    let mut map = BTreeMap::new();
    map.insert(
        "ownerId".to_string(),
        Value::Identifier(owner_id.to_buffer()),
    );
    map.insert(
        "contractId".to_string(),
        Value::Identifier(data_contract.id.to_buffer()),
    );
    map.insert(
        "transitions".to_string(),
        Value::Array(raw_document_transitions),
    );
    let mut state_transition =
        DocumentsBatchTransition::from_value_map(map, vec![data_contract.clone()])
            .expect("documents batch state transition should be created");

    state_transition
        .transitions
        .iter_mut()
        .for_each(|t| set_updated_at(t, documents[0].created_at().copied()));

    state_repository_mock
        .expect_fetch_documents()
        .returning(move |_, _, _, _| Ok(vec![documents[0].document.clone()]));

    let execution_context = StateTransitionExecutionContext::default();

    let validation_result = validate_document_batch_transition_state(
        &state_repository_mock,
        &state_transition,
        &execution_context,
    )
    .await
    .expect("validation result should be returned");

    let state_error = get_state_error(&validation_result, 0);
    assert_eq!(4025, state_error.code());
    assert!(matches!(
        state_error,
        StateError::DocumentTimestampsAreEqualError(e) if  {
            e.document_id() == &transition_id
        }
    ));
}

#[tokio::test]
async fn should_not_validate_time_in_block_window_on_dry_run() {
    let TestData {
        data_contract,
        owner_id,
        extended_documents: documents,
        mut state_repository_mock,
        ..
    } = setup_test();

    let document_transitions = get_document_transitions_fixture([(
        DocumentTransitionActionType::Create,
        vec![documents[0].clone()],
    )]);
    let raw_document_transitions: Vec<Value> = document_transitions
        .into_iter()
        .map(|dt| dt.to_object().unwrap())
        .collect();
    let mut map = BTreeMap::new();
    map.insert(
        "ownerId".to_string(),
        Value::Identifier(owner_id.to_buffer()),
    );
    map.insert(
        "contractId".to_string(),
        Value::Identifier(data_contract.id.to_buffer()),
    );
    map.insert(
        "transitions".to_string(),
        Value::Array(raw_document_transitions),
    );
    let mut state_transition =
        DocumentsBatchTransition::from_value_map(map, vec![data_contract.clone()])
            .expect("documents batch state transition should be created");

    let execution_context = StateTransitionExecutionContext::default().with_dry_run();
    let now_ts_minus_6_mins =
        Utc::now().timestamp_millis() as u64 - Duration::from_secs(60 * 6).as_millis() as u64;
    state_transition
        .transitions
        .iter_mut()
        .for_each(|t| set_created_at(t, Some(now_ts_minus_6_mins)));

    state_repository_mock
        .expect_fetch_documents()
        .returning(move |_, _, _, _| Ok(vec![]));

    let result = validate_document_batch_transition_state(
        &state_repository_mock,
        &state_transition,
        &execution_context,
    )
    .await
    .expect("validation result should be returned");

    assert!(result.is_valid());
}

#[tokio::test]
async fn should_return_invalid_result_if_updated_at_has_violated_time_window() {
    let TestData {
        data_contract,
        owner_id,
        extended_documents: documents,
        mut state_repository_mock,
        ..
    } = setup_test();

    let document_transitions = get_document_transitions_fixture([(
        DocumentTransitionActionType::Create,
        vec![documents[1].clone()],
    )]);
    let transition_id = document_transitions[0].base().id;
    let raw_document_transitions: Vec<Value> = document_transitions
        .into_iter()
        .map(|dt| dt.to_object().unwrap())
        .collect();
    let mut map = BTreeMap::new();
    map.insert(
        "ownerId".to_string(),
        Value::Identifier(owner_id.to_buffer()),
    );
    map.insert(
        "contractId".to_string(),
        Value::Identifier(data_contract.id.to_buffer()),
    );
    map.insert(
        "transitions".to_string(),
        Value::Array(raw_document_transitions),
    );
    let mut state_transition =
        DocumentsBatchTransition::from_value_map(map, vec![data_contract.clone()])
            .expect("documents batch state transition should be created");

    let now_ts_minus_6_mins =
        Utc::now().timestamp_millis() as u64 - Duration::from_secs(60 * 6).as_millis() as u64;
    state_transition.transitions.iter_mut().for_each(|t| {
        set_updated_at(t, Some(now_ts_minus_6_mins));
        set_created_at(t, None);
    });

    state_repository_mock
        .expect_fetch_documents()
        .returning(move |_, _, _, _| Ok(vec![]));

    let execution_context = StateTransitionExecutionContext::default();

    let validation_result = validate_document_batch_transition_state(
        &state_repository_mock,
        &state_transition,
        &execution_context,
    )
    .await
    .expect("validation result should be returned");

    let state_error = get_state_error(&validation_result, 0);
    assert_eq!(4008, state_error.code());
    assert!(matches!(
        state_error,
        StateError::DocumentTimestampWindowViolationError(e)   if  {
            e.document_id() == &transition_id &&
            e.timestamp_name() == "updatedAt"
        }
    ));
}

#[tokio::test]
async fn should_return_valid_result_if_document_transitions_are_valid() {
    let TestData {
        data_contract,
        owner_id,
        extended_documents,
        mut state_repository_mock,
        ..
    } = setup_test();
    let mut fetched_document_1 = extended_documents[1].clone();
    let mut fetched_document_2 = extended_documents[2].clone();
    fetched_document_1.document.revision = Some(1);
    fetched_document_2.document.revision = Some(1);
    fetched_document_1.document.owner_id = owner_id;
    fetched_document_2.document.owner_id = owner_id;

    state_repository_mock
        .expect_fetch_documents()
        .returning(move |_, _, _, _| {
            Ok(vec![
                fetched_document_1.document.clone(),
                fetched_document_2.document.clone(),
            ])
        });
    let document_transitions = get_document_transitions_fixture([
        (DocumentTransitionActionType::Create, vec![]),
        (
            DocumentTransitionActionType::Replace,
            vec![extended_documents[1].clone()],
        ),
        (
            DocumentTransitionActionType::Delete,
            vec![extended_documents[2].clone()],
        ),
    ]);
    let raw_document_transitions: Vec<Value> = document_transitions
        .into_iter()
        .map(|dt| dt.to_object().unwrap())
        .collect();
    let mut map = BTreeMap::new();
    map.insert(
        "ownerId".to_string(),
        Value::Identifier(owner_id.to_buffer()),
    );
    map.insert(
        "contractId".to_string(),
        Value::Identifier(data_contract.id.to_buffer()),
    );
    map.insert(
        "transitions".to_string(),
        Value::Array(raw_document_transitions),
    );
    let state_transition =
        DocumentsBatchTransition::from_value_map(map, vec![data_contract.clone()])
            .expect("documents batch state transition should be created");

    let execution_context = StateTransitionExecutionContext::default();

    let validation_result = validate_document_batch_transition_state(
        &state_repository_mock,
        &state_transition,
        &execution_context,
    )
    .await
    .expect("validation result should be returned");
    println!("result is {:#?}", validation_result);
    assert!(validation_result.is_valid());
}
