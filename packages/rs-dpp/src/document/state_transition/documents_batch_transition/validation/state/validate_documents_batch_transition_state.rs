use futures::future::join_all;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

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

use super::{
    execute_data_triggers::execute_data_triggers, fetch_documents::fetch_documents,
    validate_documents_uniqueness_by_indices::validate_documents_uniqueness_by_indices,
};

const BLOCK_TIME_WINDOW_MINUTES: usize = 5;
const BLOCK_TIME_WINDOW_MS: usize = BLOCK_TIME_WINDOW_MINUTES * BLOCK_TIME_WINDOW_MINUTES * 1000;

type StartTimeMs = usize;
type EndTimeMs = usize;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BlockHeader {
    pub time: HeaderTime,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HeaderTime {
    pub seconds: usize,
}

pub async fn validate_document_batch_transition_state(
    state_repository: &impl StateRepositoryLike,
    state_transition: &DocumentsBatchTransition,
) -> Result<ValidationResult<()>, ProtocolError> {
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

pub async fn validate_document_transitions(
    state_repository: &impl StateRepositoryLike,
    data_contract_id: &Identifier,
    owner_id: &Identifier,
    document_transitions: impl IntoIterator<Item = impl AsRef<DocumentTransition>>,
) -> Result<ValidationResult<()>, ProtocolError> {
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
) -> ValidationResult<()> {
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
) -> ValidationResult<()> {
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
) -> ValidationResult<()> {
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
) -> ValidationResult<()> {
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
) -> ValidationResult<()> {
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

fn check_if_timestamps_are_equal(document_transition: &DocumentTransition) -> ValidationResult<()> {
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
) -> ValidationResult<()> {
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
) -> ValidationResult<()> {
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
