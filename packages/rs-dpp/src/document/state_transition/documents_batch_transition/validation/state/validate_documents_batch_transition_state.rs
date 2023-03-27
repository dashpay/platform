use std::convert::TryInto;

use async_trait::async_trait;
use futures::future::join_all;
use itertools::Itertools;

use crate::data_contract::errors::DataContractNotPresentError;
use crate::document::document_transition::{
    DocumentReplaceTransitionAction, DocumentTransitionAction,
};
use crate::document::state_transition::documents_batch_transition::{
    DocumentsBatchTransitionAction, DOCUMENTS_BATCH_TRANSITION_ACTION_VERSION,
};
use crate::document::Document;
use crate::validation::{AsyncStateTransitionDataValidator, SimpleValidationResult};
use crate::{
    block_time_window::validate_time_in_block_time_window::validate_time_in_block_time_window,
    consensus::ConsensusError,
    data_trigger::DataTriggerExecutionContext,
    document::{
        document_transition::{DocumentTransition, DocumentTransitionExt},
        DocumentsBatchTransition,
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

pub struct DocumentsBatchTransitionStateValidator<SR>
where
    SR: StateRepositoryLike,
{
    state_repository: SR,
}

#[async_trait(?Send)]
impl<SR> AsyncStateTransitionDataValidator for DocumentsBatchTransitionStateValidator<SR>
where
    SR: StateRepositoryLike,
{
    type StateTransition = DocumentsBatchTransition;
    type StateTransitionAction = DocumentsBatchTransitionAction;

    async fn validate(
        &self,
        data: &DocumentsBatchTransition,
    ) -> Result<DocumentsBatchTransitionAction, SimpleValidationResult> {
        validate_document_batch_transition_state(&self.state_repository, data).await
    }
}

impl<SR> DocumentsBatchTransitionStateValidator<SR>
where
    SR: StateRepositoryLike,
{
    pub fn new(state_repository: SR) -> DocumentsBatchTransitionStateValidator<SR>
    where
        SR: StateRepositoryLike,
    {
        DocumentsBatchTransitionStateValidator { state_repository }
    }
}

pub async fn validate_document_batch_transition_state(
    state_repository: &impl StateRepositoryLike,
    state_transition: &DocumentsBatchTransition,
) -> Result<DocumentsBatchTransitionAction, ValidationResult<()>> {
    let mut result = ValidationResult::default();
    let owner_id = *state_transition.get_owner_id();

    let transitions_by_data_contract_id = state_transition
        .get_transitions_slice()
        .iter()
        .into_group_map_by(|t| &t.base().data_contract_id);

    let mut futures = vec![];
    for (data_contract_id, transitions) in transitions_by_data_contract_id.iter() {
        futures.push(validate_document_transitions(
            state_repository,
            data_contract_id,
            owner_id,
            transitions,
            state_transition.get_execution_context(),
        ))
    }

    let state_transition_actions = join_all(futures)
        .await
        .into_iter()
        .filter_map(|execution_result| match execution_result {
            Ok(state_transition) => Some(state_transition),
            Err(validation_result) => {
                result.merge(validation_result);
                None
            }
        })
        .flatten()
        .collect::<Vec<DocumentTransitionAction>>();

    if result.is_valid() {
        let batch_transition_action = DocumentsBatchTransitionAction {
            version: DOCUMENTS_BATCH_TRANSITION_ACTION_VERSION,
            owner_id,
            transitions: state_transition_actions,
        };
        Ok(batch_transition_action)
    } else {
        Err(result)
    }
}

pub async fn validate_document_transitions(
    state_repository: &impl StateRepositoryLike,
    data_contract_id: &Identifier,
    owner_id: Identifier,
    document_transitions: &[&DocumentTransition],
    execution_context: &StateTransitionExecutionContext,
) -> Result<Vec<DocumentTransitionAction>, ValidationResult<()>> {
    let mut result = ValidationResult::default();

    // We use temporary execution context without dry run,
    // because despite the dryRun, we need to get the
    // data contract to proceed with following logic
    let tmp_execution_context = StateTransitionExecutionContext::default();

    // Data Contract must exist
    let data_contract = state_repository
        .fetch_data_contract(data_contract_id, &tmp_execution_context)
        .await?
        .map(TryInto::try_into)
        .transpose()
        .map_err(Into::into)?
        .ok_or_else(|| {
            ProtocolError::DataContractNotPresentError(DataContractNotPresentError::new(
                *data_contract_id,
            ))
        })?;

    execution_context.add_operations(tmp_execution_context.get_operations());

    let fetched_documents =
        fetch_documents(state_repository, document_transitions, execution_context).await?;

    // Calculate time window for timestamp
    let last_header_time_millis = state_repository.fetch_latest_platform_block_time().await?;

    let document_transition_actions = if !execution_context.is_dry_run() {
        let document_transition_actions = document_transitions
            .iter()
            .filter_map(|transition| {
                let validation_result = validate_transition(
                    transition.as_ref(),
                    &fetched_documents,
                    last_header_time_millis,
                    &owner_id,
                );
                match validation_result {
                    Ok(document_transition_action) => Some(document_transition_action),
                    Err(validation_errors) => {
                        result.merge(validation_errors);
                        None
                    }
                }
            })
            .collect::<Vec<DocumentTransitionAction>>();
        if !result.is_valid() {
            return Err(result);
        }
        document_transition_actions
    } else {
        vec![]
    };

    if let Err(e) = validate_documents_uniqueness_by_indices(
        state_repository,
        &owner_id,
        document_transitions
            .iter()
            .filter(|d| d.as_transition_delete().is_none())
            .cloned(),
        &data_contract,
        execution_context,
    )
    .await
    {
        result.merge(e);
        return Err(result);
    }

    let data_trigger_execution_context = DataTriggerExecutionContext {
        state_repository: state_repository.to_owned(),
        owner_id: &owner_id,
        data_contract: &data_contract,
        state_transition_execution_context: execution_context,
    };
    let data_trigger_execution_results =
        execute_data_triggers(document_transitions, &data_trigger_execution_context).await?;

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

    return if !result.is_valid() {
        Err(result)
    } else {
        Ok(document_transition_actions)
    };
}

fn validate_transition(
    transition: &DocumentTransition,
    fetched_documents: &[Document],
    last_header_block_time_millis: u64,
    owner_id: &Identifier,
) -> Result<DocumentTransitionAction, ValidationResult<()>> {
    let mut result = ValidationResult::default();
    match transition {
        DocumentTransition::Create(document_create_transition) => {
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

            if result.is_valid() {
                Ok(DocumentTransitionAction::CreateAction(
                    document_create_transition.into(),
                ))
            } else {
                Err(result)
            }
        }
        DocumentTransition::Replace(document_replace_transition) => {
            let validation_result =
                check_updated_inside_time_window(transition, last_header_block_time_millis);
            result.merge(validation_result);

            let validation_result = check_revision(transition, fetched_documents);
            result.merge(validation_result);

            let validation_result = check_if_document_can_be_found(transition, fetched_documents);
            let original_document = match validation_result {
                Ok(document) => document,
                Err(validation_error) => {
                    result.merge(validation_error);
                    return Err(result);
                }
            };

            let validation_result = check_ownership(transition, original_document, owner_id);
            result.merge(validation_result);

            if result.is_valid() {
                Ok(DocumentTransitionAction::ReplaceAction(
                    DocumentReplaceTransitionAction::from_document_replace_transition(
                        document_replace_transition,
                        original_document.created_at,
                    ),
                ))
            } else {
                Err(result)
            }
        }
        DocumentTransition::Delete(document_delete_transition) => {
            let validation_result = check_if_document_can_be_found(transition, fetched_documents);
            let original_document = match validation_result {
                Ok(document) => document,
                Err(validation_error) => {
                    result.merge(validation_error);
                    return Err(result);
                }
            };

            let validation_result = check_ownership(transition, original_document, owner_id);
            result.merge(validation_result);

            if result.is_valid() {
                Ok(DocumentTransitionAction::DeleteAction(
                    document_delete_transition.into(),
                ))
            } else {
                Err(result)
            }
        }
    }
}

fn check_ownership(
    document_transition: &DocumentTransition,
    fetched_document: &Document,
    owner_id: &Identifier,
) -> ValidationResult<()> {
    let mut result = ValidationResult::default();
    if fetched_document.owner_id != owner_id {
        result.add_error(ConsensusError::StateError(Box::new(
            StateError::DocumentOwnerIdMismatchError {
                document_id: document_transition.base().id,
                document_owner_id: owner_id.to_owned(),
                existing_document_owner_id: fetched_document.owner_id,
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
    let Some(previous_revision) =  fetched_document.revision else {
        result.add_error(ConsensusError::StateError(Box::new(
            StateError::InvalidDocumentRevisionError {
                document_id: document_transition.base().id,
                current_revision: None,
            },
        )));
        return result;
    };
    let expected_revision = previous_revision + 1;
    if revision != expected_revision {
        result.add_error(ConsensusError::StateError(Box::new(
            StateError::InvalidDocumentRevisionError {
                document_id: document_transition.base().id,
                current_revision: Some(previous_revision),
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
                document_id: document_transition.base().id,
            },
        )))
    }
    result
}

fn check_if_document_can_be_found<'a>(
    document_transition: &'a DocumentTransition,
    fetched_documents: &'a [Document],
) -> Result<&'a Document, ValidationResult<()>> {
    let maybe_fetched_document = fetched_documents
        .iter()
        .find(|d| d.id == document_transition.base().id);

    maybe_fetched_document.ok_or(
        ConsensusError::StateError(Box::new(StateError::DocumentNotFoundError {
            document_id: document_transition.base().id,
        }))
        .into(),
    )
}

fn check_if_timestamps_are_equal(document_transition: &DocumentTransition) -> ValidationResult<()> {
    let mut result = ValidationResult::default();
    let created_at = document_transition.get_created_at();
    let updated_at = document_transition.get_updated_at();

    if created_at.is_some() && updated_at.is_some() && updated_at.unwrap() != created_at.unwrap() {
        result.add_error(ConsensusError::StateError(Box::new(
            StateError::DocumentTimestampsMismatchError {
                document_id: document_transition.base().id,
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
    };

    let window_validation = validate_time_in_block_time_window(last_block_ts_millis, created_at);
    if !window_validation.is_valid() {
        result.add_error(ConsensusError::StateError(Box::new(
            StateError::DocumentTimestampWindowViolationError {
                timestamp_name: String::from("createdAt"),
                document_id: document_transition.base().id,
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
    };

    let window_validation = validate_time_in_block_time_window(last_block_ts_millis, updated_at);
    if !window_validation.is_valid() {
        result.add_error(ConsensusError::StateError(Box::new(
            StateError::DocumentTimestampWindowViolationError {
                timestamp_name: String::from("updatedAt"),
                document_id: document_transition.base().id,
                timestamp: updated_at as i64,
                time_window_start: window_validation.time_window_start as i64,
                time_window_end: window_validation.time_window_end as i64,
            },
        )));
    }
    result
}
