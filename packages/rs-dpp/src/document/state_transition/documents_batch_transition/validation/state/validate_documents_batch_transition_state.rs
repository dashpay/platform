use super::{
    execute_data_triggers::execute_data_triggers, fetch_documents_factory::fetch_documents,
    validate_documents_uniqueness_by_indices::validate_documents_uniqueness_by_indices,
};
use crate::{
    consensus::ConsensusError,
    data_contract::DataContract,
    data_trigger::DataTriggerExecutionContext,
    document::{
        document_transition::{Action, DocumentTransition, DocumentTransitionExt},
        Document, DocumentsBatchTransition,
    },
    prelude::Identifier,
    state_repository::StateRepositoryLike,
    validation::ValidationResult,
    ProtocolError, StateError,
};
use futures::future::join_all;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

const BLOCK_TIME_WINDOW_MINUTES: usize = 5;
const BLOCK_TIME_WINDOW_MS: usize = BLOCK_TIME_WINDOW_MINUTES * BLOCK_TIME_WINDOW_MINUTES * 1000;

type StartTimeMs = usize;
type EndTimeMs = usize;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BlockHeader {
    time: HeaderTime,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HeaderTime {
    seconds: usize,
}

pub async fn validate_document_batch_transition_state(
    state_repository: &impl StateRepositoryLike,
    state_transition: &DocumentsBatchTransition,
) -> Result<ValidationResult, ProtocolError> {
    let mut result = ValidationResult::default();
    let owner_id = state_transition.get_owner_id();

    let transitions_by_data_contract_id = state_transition
        .get_transitions()
        .iter()
        .group_by(|t| &t.base().data_contract_id);

    let mut futures = vec![];
    for (data_contract_id, transitions) in transitions_by_data_contract_id.into_iter() {
        futures.push(validate_document_transitions(
            state_repository,
            data_contract_id,
            owner_id,
            transitions,
        ))
    }
    for execution_result in join_all(futures).await {
        result.merge(execution_result?);
    }

    Ok(result)
}

// pub fn validate_documents_batch_transition_state(state_repository: impl StateRepositoryLike) {}
async fn validate_document_transitions(
    state_repository: &impl StateRepositoryLike,
    data_contract_id: &Identifier,
    owner_id: &Identifier,
    document_transitions: impl IntoIterator<Item = impl AsRef<DocumentTransition>>,
) -> Result<ValidationResult, ProtocolError> {
    let mut result = ValidationResult::default();
    let transitions: Vec<_> = document_transitions.into_iter().collect();

    let data_contract = state_repository
        .fetch_data_contract::<DataContract>(data_contract_id)
        .await
        .map_err(|_| ProtocolError::DataContractNotPresentError {
            data_contract_id: data_contract_id.clone(),
        })?;

    let fetched_documents = fetch_documents(state_repository, &transitions).await?;
    let header: BlockHeader = state_repository
        .fetch_latest_platform_block_header()
        .await?;

    let (time_window_start, time_window_end) = calculate_time_window(header.time.seconds * 1000);

    for transition in transitions.iter() {
        let validation_result = validate_transition(
            transition.as_ref(),
            &fetched_documents,
            time_window_start,
            time_window_end,
            owner_id,
        );
        result.merge(validation_result);
    }
    if !result.is_valid() {
        return Ok(result);
    }

    let validation_result = validate_documents_uniqueness_by_indices(
        state_repository,
        owner_id,
        transitions
            .iter()
            .filter(|d| d.as_ref().as_transition_delete().is_none()),
        &data_contract,
    )
    .await?;
    if !result.is_valid() {
        result.merge(validation_result);
        return Ok(result);
    }

    let data_trigger_execution_context = DataTriggerExecutionContext {
        state_repository: state_repository.to_owned(),
        owner_id,
        data_contract: &data_contract,
    };

    let data_trigger_execution_results =
        execute_data_triggers(transitions.iter(), &data_trigger_execution_context).await?;

    for execution_result in data_trigger_execution_results.into_iter() {
        if !execution_result.is_ok() {
            result.add_errors(
                execution_result
                    .errors
                    .into_iter()
                    .map(ConsensusError::from)
                    .collect(),
            )
        }
    }

    Ok(result)
}

fn calculate_time_window(timestamp_ms: usize) -> (StartTimeMs, EndTimeMs) {
    let start_time_ms = timestamp_ms - BLOCK_TIME_WINDOW_MS;
    let end_time_ms = timestamp_ms + BLOCK_TIME_WINDOW_MS;
    (start_time_ms, end_time_ms)
}

fn validate_transition(
    transition: &DocumentTransition,
    fetched_documents: &[Document],
    time_window_start: usize,
    time_window_end: usize,
    owner_id: &Identifier,
) -> ValidationResult {
    let mut result = ValidationResult::default();
    match transition.base().action {
        Action::Create => {
            let validation_result = check_if_timestamps_are_equal(transition);
            result.merge(validation_result);

            let validation_result =
                check_created_inside_time_window(transition, time_window_start, time_window_end);
            result.merge(validation_result);

            let validation_result =
                check_updated_inside_time_window(transition, time_window_start, time_window_end);
            result.merge(validation_result);

            let validation_result =
                check_if_document_is_already_present(transition, fetched_documents);
            result.merge(validation_result);
        }
        Action::Replace => {
            let validation_result =
                check_updated_inside_time_window(transition, time_window_start, time_window_end);
            result.merge(validation_result);

            let validation_result = check_revision(transition, fetched_documents);
            result.merge(validation_result);

            let validation_result = check_if_document_can_be_found(transition, fetched_documents);
            if !validation_result.is_valid() {
                result.merge(validation_result);
                return result;
            }

            let validation_result = check_ownership(transition, fetched_documents, owner_id);
            result.merge(validation_result);
        }
        Action::Delete => {
            let validation_result = check_if_document_can_be_found(transition, fetched_documents);
            if !validation_result.is_valid() {
                result.merge(validation_result);
                return result;
            }

            let validation_result = check_ownership(transition, fetched_documents, owner_id);
            result.merge(validation_result);
        }
    }
    result
}

fn check_ownership(
    document_transition: &DocumentTransition,
    fetched_documents: &[Document],
    owner_id: &Identifier,
) -> ValidationResult {
    let mut result = ValidationResult::default();
    let fetched_document = match fetched_documents
        .iter()
        .find(|d| d.id == document_transition.base().id)
    {
        Some(d) => d,
        None => return result,
    };
    if &fetched_document.owner_id != owner_id {
        result.add_error(ConsensusError::StateError(Box::new(
            StateError::DocumentOwnerMismatchError {
                document_id: document_transition.base().id.clone(),
                document_owner_id: owner_id.to_owned(),
                existing_document_owner_id: fetched_document.owner_id.clone(),
            },
        )));
    }
    result
}

fn check_revision(
    document_transition: &DocumentTransition,
    fetched_documents: &[Document],
) -> ValidationResult {
    let mut result = ValidationResult::default();
    let fetched_document = match fetched_documents
        .iter()
        .find(|d| d.id == document_transition.base().id)
    {
        Some(d) => d,
        None => return result,
    };
    let revision = match document_transition.as_transition_replace() {
        Some(d) => d.revision,
        None => return result,
    };
    let expected_revision = fetched_document.revision + 1;
    if revision != expected_revision {
        result.add_error(ConsensusError::StateError(Box::new(
            StateError::InvalidDocumentRevisionError {
                document_id: document_transition.base().id.clone(),
                current_revision: fetched_document.revision,
            },
        )))
    }
    result
}

fn check_if_document_is_already_present(
    document_transition: &DocumentTransition,
    fetched_documents: &[Document],
) -> ValidationResult {
    let mut result = ValidationResult::default();
    let maybe_fetched_document = fetched_documents
        .iter()
        .find(|d| d.id == document_transition.base().id);

    if maybe_fetched_document.is_some() {
        result.add_error(ConsensusError::StateError(Box::new(
            StateError::DocumentAlreadyPresentError {
                document_id: document_transition.base().id.clone(),
            },
        )))
    }
    result
}

fn check_if_document_can_be_found(
    document_transition: &DocumentTransition,
    fetched_documents: &[Document],
) -> ValidationResult {
    let mut result = ValidationResult::default();
    let maybe_fetched_document = fetched_documents
        .iter()
        .find(|d| d.id == document_transition.base().id);

    if maybe_fetched_document.is_none() {
        result.add_error(ConsensusError::StateError(Box::new(
            StateError::DocumentNotFoundError {
                document_id: document_transition.base().id.clone(),
            },
        )))
    }
    result
}

fn check_if_timestamps_are_equal(document_transition: &DocumentTransition) -> ValidationResult {
    let mut result = ValidationResult::default();
    let created_at = document_transition.get_created_at();
    let updated_at = document_transition.get_updated_at();

    if created_at.is_some() && updated_at.is_some() && updated_at.unwrap() != created_at.unwrap() {
        result.add_error(ConsensusError::StateError(Box::new(
            StateError::DocumentTimestampMismatchError {
                document_id: document_transition.base().id.clone(),
            },
        )));
    }

    result
}

fn check_created_inside_time_window(
    document_transition: &DocumentTransition,
    start_window_ms: usize,
    end_window_ms: usize,
) -> ValidationResult {
    let mut result = ValidationResult::default();
    let created_at = match document_transition.get_created_at() {
        Some(t) => t,
        None => return result,
    };

    if created_at < start_window_ms as i64 || created_at > end_window_ms as i64 {
        result.add_error(ConsensusError::StateError(Box::new(
            StateError::DocumentTimestampWindowViolationError {
                timestamp_name: String::from("createdAt"),
                document_id: document_transition.base().id.clone(),
                timestamp: created_at,
                time_window_start: start_window_ms as i64,
                time_window_end: end_window_ms as i64,
            },
        )));
    }
    result
}

fn check_updated_inside_time_window(
    document_transition: &DocumentTransition,
    start_window_ms: usize,
    end_window_ms: usize,
) -> ValidationResult {
    let mut result = ValidationResult::default();
    let updated_at = match document_transition.get_updated_at() {
        Some(t) => t,
        None => return result,
    };

    if updated_at < start_window_ms as i64 || updated_at > end_window_ms as i64 {
        result.add_error(ConsensusError::StateError(Box::new(
            StateError::DocumentTimestampWindowViolationError {
                timestamp_name: String::from("updatedAt"),
                document_id: document_transition.base().id.clone(),
                timestamp: updated_at,
                time_window_start: start_window_ms as i64,
                time_window_end: end_window_ms as i64,
            },
        )));
    }
    result
}

//
// let owner_id =

#[cfg(test)]
mod test {
    use std::time::Duration;

    use super::*;
    use crate::{
        codes::ErrorWithCode,
        consensus::ConsensusError,
        data_contract::DataContract,
        document::{
            document_transition::{Action, DocumentTransition, DocumentTransitionObjectLike},
            Document, DocumentsBatchTransition,
        },
        prelude::Identifier,
        state_repository::MockStateRepositoryLike,
        tests::{
            fixtures::{
                get_data_contract_fixture, get_document_transitions_fixture, get_documents_fixture,
            },
            utils::generate_random_identifier_struct,
        },
        validation::ValidationResult,
        StateError,
    };
    use anyhow::anyhow;
    use chrono::Utc;
    use serde_json::{json, Value as JsonValue};

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
            .returning(move |_| Ok(data_contract_to_return.clone()));
        state_repository_mock
            .expect_fetch_latest_platform_block_header::<BlockHeader>()
            .returning(|| {
                Ok(BlockHeader {
                    time: HeaderTime {
                        seconds: Utc::now().timestamp() as usize,
                    },
                })
            });

        TestData {
            owner_id,
            data_contract,
            document_transitions,
            documents,
            state_transition,
            state_repository_mock,
        }
    }

    fn get_state_error(result: &ValidationResult, error_number: usize) -> &StateError {
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
            DocumentTransition::Delete(ref mut t) => {}
        }
    }

    fn set_created_at(dt: &mut DocumentTransition, ts: Option<i64>) {
        match dt {
            DocumentTransition::Create(ref mut t) => t.created_at = ts,
            DocumentTransition::Replace(ref mut t) => {}
            DocumentTransition::Delete(ref mut t) => {}
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
            .returning(|_| Err(anyhow!("no document found")));

        let error = validate_document_transitions(
            &state_repository_mock,
            &data_contract.id,
            &owner_id,
            document_transitions,
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
    async fn should_return_invalid_result_if_document_transition_with_action_delete_is_not_present()
    {
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
            .returning(move |_, _, _| Ok(vec![]));

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
            .returning(move |_, _, _| Ok(vec![documents[0].clone()]));

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
            .returning(move |_, _, _| Ok(vec![fetched_document.clone()]));

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
            .returning(move |_, _, _| Ok(vec![]));

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
            .returning(move |_, _, _| Ok(vec![]));

        let validation_result =
            validate_document_batch_transition_state(&state_repository_mock, &state_transition)
                .await
                .expect("validation result should be returned");

        println!("the validation result is {:#?}", validation_result);
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
            .returning(move |_, _, _| Ok(vec![]));

        let validation_result =
            validate_document_batch_transition_state(&state_repository_mock, &state_transition)
                .await
                .expect("validation result should be returned");

        println!("the validation result is {:#?}", validation_result);
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
            .returning(move |_, _, _| {
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
}
