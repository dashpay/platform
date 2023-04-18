use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

use crate::error::Error;
use crate::execution::data_trigger::DataTriggerExecutionContext;
use crate::platform::PlatformStateRef;
use crate::validation::state_transition::document_state_validation::execute_data_triggers::execute_data_triggers;
use crate::validation::state_transition::document_state_validation::fetch_documents::fetch_documents_for_transitions_knowing_contract_and_document_type;
use dpp::consensus::basic::BasicError;
use dpp::data_contract::document_type::DocumentType;
use dpp::data_contract::{DataContract, DriveContractExt};
use dpp::document::document_transition::{
    DocumentCreateTransitionAction, DocumentDeleteTransitionAction, DocumentReplaceTransition,
    DocumentReplaceTransitionAction, DocumentTransitionAction,
};
use dpp::document::state_transition::documents_batch_transition::{
    DocumentsBatchTransitionAction, DOCUMENTS_BATCH_TRANSITION_ACTION_VERSION,
};
use dpp::document::Document;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::{
    block_time_window::validate_time_in_block_time_window::validate_time_in_block_time_window,
    consensus::ConsensusError,
    document::{
        document_transition::{DocumentTransition, DocumentTransitionExt},
        DocumentsBatchTransition,
    },
    prelude::{Identifier, TimestampMillis},
    state_transition::{
        state_transition_execution_context::StateTransitionExecutionContext,
        StateTransitionIdentitySigned,
    },
    validation::ConsensusValidationResult,
    ProtocolError, StateError,
};
use drive::grovedb::TransactionArg;

pub fn validate_document_batch_transition_state(
    platform: &PlatformStateRef,
    batch_state_transition: &DocumentsBatchTransition,
    transaction: TransactionArg,
    execution_context: &StateTransitionExecutionContext,
) -> Result<ConsensusValidationResult<DocumentsBatchTransitionAction>, Error> {
    let owner_id = *batch_state_transition.get_owner_id();
    let mut transitions_by_contracts_and_types: BTreeMap<
        &Identifier,
        BTreeMap<&String, Vec<&DocumentTransition>>,
    > = BTreeMap::new();

    // We want to validate by contract, and then for each document type within a contract
    for document_transition in batch_state_transition.transitions.iter() {
        let document_type = &document_transition.base().document_type_name;
        let data_contract_id = &document_transition.base().data_contract_id;

        match transitions_by_contracts_and_types.entry(data_contract_id) {
            Entry::Vacant(v) => {
                v.insert(BTreeMap::from([(document_type, vec![document_transition])]));
            }
            Entry::Occupied(mut transitions_by_types_in_contract) => {
                match transitions_by_types_in_contract
                    .get_mut()
                    .entry(document_type)
                {
                    Entry::Vacant(v) => {
                        v.insert(vec![document_transition]);
                    }
                    Entry::Occupied(mut o) => o.get_mut().push(document_transition),
                }
            }
        }
    }

    let validation_result = transitions_by_contracts_and_types
        .iter()
        .map(
            |(data_contract_id, document_transitions_by_document_type)| {
                validate_document_transitions_within_contract(
                    platform,
                    data_contract_id,
                    owner_id,
                    document_transitions_by_document_type,
                    execution_context,
                    transaction,
                )
            },
        )
        .collect::<Result<Vec<ConsensusValidationResult<Vec<DocumentTransitionAction>>>, Error>>(
        )?;
    let validation_result = ConsensusValidationResult::flatten(validation_result);

    if validation_result.is_valid() {
        let batch_transition_action = DocumentsBatchTransitionAction {
            version: DOCUMENTS_BATCH_TRANSITION_ACTION_VERSION,
            owner_id,
            transitions: validation_result.into_data()?,
        };
        Ok(ConsensusValidationResult::new_with_data(
            batch_transition_action,
        ))
    } else {
        Ok(ConsensusValidationResult::new_with_errors(
            validation_result.errors,
        ))
    }
}

fn validate_document_transitions_within_contract(
    platform: &PlatformStateRef,
    data_contract_id: &Identifier,
    owner_id: Identifier,
    document_transitions: &BTreeMap<&String, Vec<&DocumentTransition>>,
    execution_context: &StateTransitionExecutionContext,
    transaction: TransactionArg,
) -> Result<ConsensusValidationResult<Vec<DocumentTransitionAction>>, Error> {
    let drive = platform.drive;
    // Data Contract must exist
    let Some(contract_fetch_info) = drive
            .get_contract_with_fetch_info(data_contract_id.0 .0, None, false, transaction)?
            .1
        else {
            return Ok(ConsensusValidationResult::new_with_error(BasicError::DataContractNotPresent { data_contract_id: *data_contract_id }.into()));
        };

    let data_contract = &contract_fetch_info.contract;

    let validation_result = document_transitions
        .iter()
        .map(|(document_type_name, document_transitions)| {
            validate_document_transitions_within_document_type(
                platform,
                data_contract,
                document_type_name,
                owner_id,
                document_transitions,
                execution_context,
                transaction,
            )
        })
        .collect::<Result<Vec<ConsensusValidationResult<Vec<DocumentTransitionAction>>>, Error>>(
        )?;
    Ok(ConsensusValidationResult::flatten(validation_result))
}

fn validate_document_transitions_within_document_type(
    platform: &PlatformStateRef,
    data_contract: &DataContract,
    document_type_name: &String,
    owner_id: Identifier,
    document_transitions: &[&DocumentTransition],
    execution_context: &StateTransitionExecutionContext,
    transaction: TransactionArg,
) -> Result<ConsensusValidationResult<Vec<DocumentTransitionAction>>, Error> {
    // We use temporary execution context without dry run,
    // because despite the dryRun, we need to get the
    // data contract to proceed with following logic
    let tmp_execution_context = StateTransitionExecutionContext::default();

    execution_context.add_operations(tmp_execution_context.get_operations());

    let document_type = data_contract.document_type_for_name(document_type_name)?;

    // we fetch all documents needed for the transitions
    // for create they should not exist
    // for replace/patch they should
    // for delete they should
    // Validation will come after, but doing one request can be faster.
    let fetched_documents_validation_result =
        fetch_documents_for_transitions_knowing_contract_and_document_type(
            platform.drive,
            data_contract,
            document_type,
            document_transitions,
            transaction,
        )?;

    if !fetched_documents_validation_result.is_valid() {
        return Ok(ConsensusValidationResult::new_with_errors(
            fetched_documents_validation_result.errors,
        ));
    }

    let fetched_documents = fetched_documents_validation_result.into_data()?;

    let document_transition_actions_result = if !execution_context.is_dry_run() {
        let document_transition_actions_validation_result = document_transitions
            .iter()
            .map(|transition| {
                // we validate every transition in this document type
                validate_transition(
                    platform,
                    data_contract,
                    document_type,
                    transition,
                    &fetched_documents,
                    &owner_id,
                    transaction,
                )
            })
            .collect::<Result<Vec<ConsensusValidationResult<DocumentTransitionAction>>, Error>>()?;

        let result =
            ConsensusValidationResult::merge_many(document_transition_actions_validation_result);

        if !result.is_valid() {
            return Ok(result);
        }
        result
    } else {
        ConsensusValidationResult::default()
    };

    if !document_transition_actions_result.is_valid() {
        return Ok(document_transition_actions_result);
    }

    let document_transition_actions = document_transition_actions_result.into_data()?;

    let data_trigger_execution_context = DataTriggerExecutionContext {
        platform,
        transaction,
        owner_id: &owner_id,
        data_contract: &data_contract,
        state_transition_execution_context: execution_context,
    };
    let data_trigger_execution_results = execute_data_triggers(
        document_transition_actions.as_slice(),
        &data_trigger_execution_context,
    )?;

    for execution_result in data_trigger_execution_results.into_iter() {
        if !execution_result.is_valid() {
            return Ok(ConsensusValidationResult::new_with_errors(
                execution_result
                    .errors
                    .into_iter()
                    .map(|e| ConsensusError::StateError(Box::new(e.into())))
                    .collect(),
            ));
        }
    }
    Ok(ConsensusValidationResult::new_with_data(
        document_transition_actions,
    ))
}

fn validate_transition(
    platform: &PlatformStateRef,
    contract: &DataContract,
    document_type: &DocumentType,
    transition: &DocumentTransition,
    fetched_documents: &[Document],
    owner_id: &Identifier,
    transaction: TransactionArg,
) -> Result<ConsensusValidationResult<DocumentTransitionAction>, Error> {
    let latest_block_time_ms = platform.state.last_block_time_ms();
    let average_block_spacing_ms = platform.config.block_spacing_ms;
    match transition {
        DocumentTransition::Create(document_create_transition) => {
            let mut result = ConsensusValidationResult::<DocumentTransitionAction>::new_with_data(
                DocumentTransitionAction::CreateAction(DocumentCreateTransitionAction::default()),
            );
            let validation_result = check_if_timestamps_are_equal(transition);
            result.merge(validation_result);

            if !result.is_valid() {
                return Ok(result);
            }

            // We do not need to perform these checks on genesis
            if let Some(latest_block_time_ms) = latest_block_time_ms {
                let validation_result = check_created_inside_time_window(
                    transition,
                    latest_block_time_ms,
                    average_block_spacing_ms,
                );
                result.merge(validation_result);

                if !result.is_valid() {
                    return Ok(result);
                }
                let validation_result = check_updated_inside_time_window(
                    transition,
                    latest_block_time_ms,
                    average_block_spacing_ms,
                );
                result.merge(validation_result);

                if !result.is_valid() {
                    return Ok(result);
                }
            }

            let validation_result =
                check_if_document_is_not_already_present(transition, fetched_documents);
            result.merge(validation_result);

            if !result.is_valid() {
                return Ok(result);
            }

            let document_create_action: DocumentCreateTransitionAction =
                document_create_transition.into();

            let validation_result = platform
                .drive
                .validate_document_create_transition_action_uniqueness(
                    contract,
                    document_type,
                    &document_create_action,
                    owner_id,
                    transaction,
                )?;
            result.merge(validation_result);

            if result.is_valid() {
                Ok(DocumentTransitionAction::CreateAction(document_create_action).into())
            } else {
                Ok(result)
            }
        }
        DocumentTransition::Replace(document_replace_transition) => {
            let mut result = ConsensusValidationResult::<DocumentTransitionAction>::new_with_data(
                DocumentTransitionAction::ReplaceAction(DocumentReplaceTransitionAction::default()),
            );
            // We do not need to perform this check on genesis
            if let Some(latest_block_time_ms) = latest_block_time_ms {
                let validation_result = check_updated_inside_time_window(
                    transition,
                    latest_block_time_ms,
                    average_block_spacing_ms,
                );
                result.merge(validation_result);

                if !result.is_valid() {
                    return Ok(result);
                }
            }

            let validation_result = check_if_document_can_be_found(transition, fetched_documents);
            // we only do is_valid on purpose because it would be a system error if it didn't have
            // data
            let original_document = if validation_result.is_valid() {
                validation_result.into_data()?
            } else {
                result.add_errors(validation_result.errors);
                return Ok(result);
            };

            // we check the revision first because it is a more common issue
            let validation_result =
                check_revision_is_bumped_by_one(document_replace_transition, original_document);
            result.merge(validation_result);

            if !result.is_valid() {
                return Ok(result);
            }

            let validation_result = check_ownership(transition, original_document, owner_id);
            result.merge(validation_result);

            if !result.is_valid() {
                return Ok(result);
            }

            let document_replace_action: DocumentReplaceTransitionAction =
                DocumentReplaceTransitionAction::from_document_replace_transition(
                    document_replace_transition,
                    original_document.created_at,
                );

            let validation_result = platform
                .drive
                .validate_document_replace_transition_action_uniqueness(
                    contract,
                    document_type,
                    &document_replace_action,
                    owner_id,
                    transaction,
                )?;
            result.merge(validation_result);

            if result.is_valid() {
                Ok(DocumentTransitionAction::ReplaceAction(document_replace_action).into())
            } else {
                Ok(result)
            }
        }
        DocumentTransition::Delete(document_delete_transition) => {
            let mut result = ConsensusValidationResult::<DocumentTransitionAction>::new_with_data(
                DocumentTransitionAction::DeleteAction(DocumentDeleteTransitionAction::default()),
            );
            let validation_result = check_if_document_can_be_found(transition, fetched_documents);

            // we only do is_valid on purpose because it would be a system error if it didn't have
            // data
            let original_document = if validation_result.is_valid() {
                validation_result.into_data()?
            } else {
                result.add_errors(validation_result.errors);
                return Ok(result);
            };

            let validation_result = check_ownership(transition, original_document, owner_id);
            if !validation_result.is_valid() {
                result.add_errors(validation_result.errors);
            }

            if result.is_valid() {
                Ok(
                    DocumentTransitionAction::DeleteAction(document_delete_transition.into())
                        .into(),
                )
            } else {
                Ok(result)
            }
        }
    }
}

pub fn check_ownership(
    document_transition: &DocumentTransition,
    fetched_document: &Document,
    owner_id: &Identifier,
) -> SimpleConsensusValidationResult {
    let mut result = SimpleConsensusValidationResult::default();
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

pub fn check_revision_is_bumped_by_one(
    document_transition: &DocumentReplaceTransition,
    original_document: &Document,
) -> SimpleConsensusValidationResult {
    let mut result = SimpleConsensusValidationResult::default();

    let revision = document_transition.revision;

    // If there was no previous revision this means that the document_type is not update-able
    // However this should have been caught earlier
    let Some(previous_revision) =  original_document.revision else {
        result.add_error(ConsensusError::StateError(Box::new(
            StateError::InvalidDocumentRevisionError {
                document_id: document_transition.base.id,
                current_revision: None,
            },
        )));
        return result;
    };
    // no need to check bounds here, because it would be impossible to hit the end on a u64
    let expected_revision = previous_revision + 1;
    if revision != expected_revision {
        result.add_error(ConsensusError::StateError(Box::new(
            StateError::InvalidDocumentRevisionError {
                document_id: document_transition.base.id,
                current_revision: Some(previous_revision),
            },
        )))
    }
    result
}

/// We don't want the document id to already be present in the state
pub fn check_if_document_is_not_already_present(
    document_transition: &DocumentTransition,
    fetched_documents: &[Document],
) -> SimpleConsensusValidationResult {
    let mut result = SimpleConsensusValidationResult::default();
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

pub fn check_if_document_can_be_found<'a>(
    document_transition: &'a DocumentTransition,
    fetched_documents: &'a [Document],
) -> ConsensusValidationResult<&'a Document> {
    let maybe_fetched_document = fetched_documents
        .iter()
        .find(|d| d.id == document_transition.base().id);

    if let Some(document) = maybe_fetched_document {
        ConsensusValidationResult::new_with_data(document)
    } else {
        ConsensusValidationResult::new_with_error(ConsensusError::StateError(Box::new(
            StateError::DocumentNotFoundError {
                document_id: document_transition.base().id,
            },
        )))
    }
}

pub fn check_if_timestamps_are_equal(
    document_transition: &DocumentTransition,
) -> SimpleConsensusValidationResult {
    let mut result = SimpleConsensusValidationResult::default();
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

pub fn check_created_inside_time_window(
    document_transition: &DocumentTransition,
    last_block_ts_millis: TimestampMillis,
    average_block_spacing_ms: u64,
) -> SimpleConsensusValidationResult {
    let mut result = SimpleConsensusValidationResult::default();
    let created_at = match document_transition.get_created_at() {
        Some(t) => t,
        None => return result,
    };

    let window_validation = validate_time_in_block_time_window(
        last_block_ts_millis,
        created_at,
        average_block_spacing_ms,
    );
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

pub fn check_updated_inside_time_window(
    document_transition: &DocumentTransition,
    last_block_ts_millis: TimestampMillis,
    average_block_spacing_ms: u64,
) -> SimpleConsensusValidationResult {
    let mut result = SimpleConsensusValidationResult::default();
    let updated_at = match document_transition.get_updated_at() {
        Some(t) => t,
        None => return result,
    };

    let window_validation = validate_time_in_block_time_window(
        last_block_ts_millis,
        updated_at,
        average_block_spacing_ms,
    );
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
