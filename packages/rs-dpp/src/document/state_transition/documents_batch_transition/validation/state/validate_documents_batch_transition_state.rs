use dashcore::BlockHeader;
use futures::future::join_all;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{
    block_time_window::validate_time_in_block_time_window::validate_time_in_block_time_window,
    consensus::ConsensusError,
    data_contract::DataContract,
    data_trigger::DataTriggerExecutionContext,
    document::{
        document_transition::{Action, DocumentTransition, DocumentTransitionExt},
        Document, DocumentsBatchTransition,
    },
    prelude::{Identifier, TimestampMillis},
    state_repository::StateRepositoryLike,
    state_transition::{
        state_transition_execution_context::StateTransitionExecutionContext,
        StateTransitionIdentitySigned, StateTransitionLike,
    },
    validation::ValidationResult,
    ProtocolError, StateError,
};

use super::{
    execute_data_triggers::execute_data_triggers, fetch_documents::fetch_documents,
    validate_documents_uniqueness_by_indices::validate_documents_uniqueness_by_indices,
};

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
        .into_group_map_by(|t| &t.base().data_contract_id);

    let mut futures = vec![];
    for (data_contract_id, transitions) in transitions_by_data_contract_id.into_iter() {
        futures.push(validate_document_transitions(
            state_repository,
            data_contract_id,
            owner_id,
            transitions,
            state_transition.get_execution_context(),
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
    execution_context: &StateTransitionExecutionContext,
) -> Result<ValidationResult<()>, ProtocolError> {
    let mut result = ValidationResult::default();
    let transitions: Vec<_> = document_transitions.into_iter().collect();

    // We use temporary execution context without dry run,
    // because despite the dryRun, we need to get the
    // data contract to proceed with following logic
    let tmp_execution_context = StateTransitionExecutionContext::default();

    // Data Contract must exist
    let data_contract = state_repository
        .fetch_data_contract::<DataContract>(data_contract_id, &tmp_execution_context)
        .await
        .map_err(|_| ProtocolError::DataContractNotPresentError {
            data_contract_id: data_contract_id.clone(),
        })?;

    execution_context.add_operations(tmp_execution_context.get_operations());

    let fetched_documents =
        fetch_documents(state_repository, &transitions, execution_context).await?;

    // Calculate time window for timestamp
    let block_header: BlockHeader = state_repository
        .fetch_latest_platform_block_header()
        .await?;
    let last_header_time_millis = block_header.time as u64 * 1000;

    if !execution_context.is_dry_run() {
        for transition in transitions.iter() {
            let validation_result = validate_transition(
                transition.as_ref(),
                &fetched_documents,
                last_header_time_millis,
                owner_id,
            );
            result.merge(validation_result);
        }
        if !result.is_valid() {
            return Ok(result);
        }
    }

    let validation_result = validate_documents_uniqueness_by_indices(
        state_repository,
        owner_id,
        transitions
            .iter()
            .filter(|d| d.as_ref().as_transition_delete().is_none()),
        &data_contract,
        execution_context,
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
        state_transition_execution_context: execution_context,
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

fn validate_transition(
    transition: &DocumentTransition,
    fetched_documents: &[Document],
    last_header_block_time_millis: u64,
    owner_id: &Identifier,
) -> ValidationResult<()> {
    let mut result = ValidationResult::default();
    match transition.base().action {
        Action::Create => {
            let validation_result = check_if_timestamps_are_equal(transition);
            result.merge(validation_result);

            let validation_result =
                check_created_inside_time_window(transition, last_header_block_time_millis);
            result.merge(validation_result);

            let validation_result =
                check_updated_inside_time_window(transition, last_header_block_time_millis);
            result.merge(validation_result);

            let validation_result =
                check_if_document_is_already_present(transition, fetched_documents);
            result.merge(validation_result);
        }
        Action::Replace => {
            let validation_result =
                check_updated_inside_time_window(transition, last_header_block_time_millis);
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
    last_block_ts_millis: TimestampMillis,
) -> ValidationResult<()> {
    let mut result = ValidationResult::default();
    let created_at = match document_transition.get_created_at() {
        Some(t) => t,
        None => return result,
    } as u64;

    let window_validation = validate_time_in_block_time_window(last_block_ts_millis, created_at);
    if !window_validation.is_valid() {
        result.add_error(ConsensusError::StateError(Box::new(
            StateError::DocumentTimestampWindowViolationError {
                timestamp_name: String::from("createdAt"),
                document_id: document_transition.base().id.clone(),
                timestamp: created_at as i64,
                time_window_start: window_validation.time_window_start as i64,
                time_window_end: window_validation.time_window_end as i64,
            },
        )));
    }
    result
}

fn check_updated_inside_time_window(
    document_transition: &DocumentTransition,
    last_block_ts_millis: TimestampMillis,
) -> ValidationResult<()> {
    let mut result = ValidationResult::default();
    let updated_at = match document_transition.get_updated_at() {
        Some(t) => t,
        None => return result,
    } as u64;

    let window_validation = validate_time_in_block_time_window(last_block_ts_millis, updated_at);
    if !window_validation.is_valid() {
        result.add_error(ConsensusError::StateError(Box::new(
            StateError::DocumentTimestampWindowViolationError {
                timestamp_name: String::from("updatedAt"),
                document_id: document_transition.base().id.clone(),
                timestamp: updated_at as i64,
                time_window_start: window_validation.time_window_start as i64,
                time_window_end: window_validation.time_window_end as i64,
            },
        )));
    }
    result
}
