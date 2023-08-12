use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::error::Error;
use crate::platform_types::platform::PlatformStateRef;
use dpp::consensus::basic::document::DataContractNotPresentError;
use dpp::consensus::basic::BasicError;
use dpp::consensus::state::document::document_already_present_error::DocumentAlreadyPresentError;
use dpp::consensus::state::document::document_not_found_error::DocumentNotFoundError;
use dpp::consensus::state::document::document_owner_id_mismatch_error::DocumentOwnerIdMismatchError;
use dpp::consensus::state::document::document_timestamp_window_violation_error::DocumentTimestampWindowViolationError;
use dpp::consensus::state::document::document_timestamps_mismatch_error::DocumentTimestampsMismatchError;
use dpp::consensus::state::document::invalid_document_revision_error::InvalidDocumentRevisionError;
use dpp::consensus::state::state_error::StateError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::document::{Document, DocumentV0Getters};
use dpp::validation::SimpleConsensusValidationResult;
use dpp::{
    consensus::ConsensusError,
    prelude::{Identifier, TimestampMillis},
    validation::ConsensusValidationResult,
    ProtocolError,
};

use dpp::state_transition::documents_batch_transition::{DocumentsBatchTransition};
use dpp::state_transition::documents_batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::documents_batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use dpp::state_transition::documents_batch_transition::document_transition::{DocumentTransition, DocumentReplaceTransition, DocumentTransitionV0Methods};
use dpp::state_transition::StateTransitionLike;
use drive::state_transition_action::document::documents_batch::document_transition::document_create_transition_action::DocumentCreateTransitionAction;
use drive::state_transition_action::document::documents_batch::document_transition::document_delete_transition_action::DocumentDeleteTransitionAction;
use drive::state_transition_action::document::documents_batch::document_transition::document_replace_transition_action::DocumentReplaceTransitionAction;
use drive::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;
use drive::state_transition_action::document::documents_batch::DocumentsBatchTransitionAction;
use drive::state_transition_action::document::documents_batch::v0::DocumentsBatchTransitionActionV0;
use dpp::validation::block_time_window::validate_time_in_block_time_window::validate_time_in_block_time_window;
use dpp::version::{DefaultForPlatformVersion, PlatformVersion};
use drive::grovedb::TransactionArg;
use crate::execution::validation::state_transition::documents_batch::state::v0::fetch_documents::fetch_documents_for_transitions_knowing_contract_and_document_type;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use dpp::state_transition::documents_batch_transition::document_transition::document_replace_transition::v0::v0_methods::DocumentReplaceTransitionV0Methods;
use drive::drive::contract::DataContractFetchInfo;
use crate::execution::types::state_transition_execution_context::{StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0};

pub(crate) fn validate_document_batch_transition_state(
    bypass_validation: bool,
    platform: &PlatformStateRef,
    batch_state_transition: &DocumentsBatchTransition,
    transaction: TransactionArg,
    execution_context: &mut StateTransitionExecutionContext,
) -> Result<ConsensusValidationResult<DocumentsBatchTransitionAction>, Error> {
    let owner_id = batch_state_transition.owner_id();
    let platform_version = platform.state.current_platform_version()?;
    let mut transitions_by_contracts_and_types: BTreeMap<
        &Identifier,
        BTreeMap<&String, Vec<&DocumentTransition>>,
    > = BTreeMap::new();

    // We want to validate by contract, and then for each document type within a contract
    for document_transition in batch_state_transition.transitions().iter() {
        let document_type = document_transition.base().document_type_name();
        let data_contract_id = document_transition.base().data_contract_id_ref();

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
                    bypass_validation,
                    platform,
                    data_contract_id,
                    owner_id,
                    document_transitions_by_document_type,
                    execution_context,
                    transaction,
                    platform_version,
                )
            },
        )
        .collect::<Result<Vec<ConsensusValidationResult<Vec<DocumentTransitionAction>>>, Error>>(
        )?;
    let validation_result = ConsensusValidationResult::flatten(validation_result);

    if validation_result.is_valid() {
        let batch_transition_action = DocumentsBatchTransitionActionV0 {
            owner_id,
            transitions: validation_result.into_data()?,
        }
        .into();
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
    bypass_validation: bool,
    platform: &PlatformStateRef,
    data_contract_id: &Identifier,
    owner_id: Identifier,
    document_transitions: &BTreeMap<&String, Vec<&DocumentTransition>>,
    execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<Vec<DocumentTransitionAction>>, Error> {
    let drive = platform.drive;
    // Data Contract must exist
    let Some(data_contract_fetch_info) = drive
            .get_contract_with_fetch_info_and_fee(data_contract_id.0 .0, None, false, transaction, platform_version)?
            .1
        else {
            return Ok(ConsensusValidationResult::new_with_error(BasicError::DataContractNotPresentError(DataContractNotPresentError::new(*data_contract_id)).into()));
        };

    let validation_result = document_transitions
        .iter()
        .map(|(document_type_name, document_transitions)| {
            validate_document_transitions_within_document_type(
                bypass_validation,
                platform,
                data_contract_fetch_info.clone(),
                document_type_name,
                owner_id,
                document_transitions,
                execution_context,
                transaction,
                platform_version,
            )
        })
        .collect::<Result<Vec<ConsensusValidationResult<Vec<DocumentTransitionAction>>>, Error>>(
        )?;
    Ok(ConsensusValidationResult::flatten(validation_result))
}

fn validate_document_transitions_within_document_type(
    bypass_validation: bool,
    platform: &PlatformStateRef,
    data_contract_fetch_info: Arc<DataContractFetchInfo>,
    document_type_name: &str,
    owner_id: Identifier,
    document_transitions: &[&DocumentTransition],
    execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<Vec<DocumentTransitionAction>>, Error> {
    // // We use temporary execution context without dry run,
    // // because despite the dryRun, we need to get the
    // // data contract to proceed with following logic
    // let tmp_execution_context = StateTransitionExecutionContext::default_for_platform_version(platform_version)?;
    //
    // execution_context.add_operations(tmp_execution_context.operations_slice());

    let dry_run = false; //maybe reenable

    let data_contract = &data_contract_fetch_info.contract;

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
            platform_version,
        )?;

    if !fetched_documents_validation_result.is_valid() {
        return Ok(ConsensusValidationResult::new_with_errors(
            fetched_documents_validation_result.errors,
        ));
    }

    let fetched_documents = fetched_documents_validation_result.into_data()?;

    let document_transition_actions_result = if !dry_run {
        let document_transition_actions_validation_result = document_transitions
            .iter()
            .map(|transition| {
                // we validate every transition in this document type
                validate_transition(
                    bypass_validation,
                    platform,
                    data_contract_fetch_info.clone(),
                    document_type,
                    transition,
                    &fetched_documents,
                    owner_id,
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

    Ok(ConsensusValidationResult::new_with_data(
        document_transition_actions,
    ))
}

/// The data contract can be of multiple difference versions
fn validate_transition<'a>(
    bypass_validation: bool,
    platform: &PlatformStateRef,
    data_contract_fetch_info: Arc<DataContractFetchInfo>,
    document_type: DocumentTypeRef,
    transition: &DocumentTransition,
    fetched_documents: &[Document],
    owner_id: Identifier,
    transaction: TransactionArg,
) -> Result<ConsensusValidationResult<DocumentTransitionAction>, Error> {
    let platform_version =
        PlatformVersion::get(platform.state.current_protocol_version_in_consensus())?;
    let latest_block_time_ms = platform.state.last_block_time_ms();
    let average_block_spacing_ms = platform.config.block_spacing_ms;
    match transition {
        DocumentTransition::Create(document_create_transition) => {
            let mut result = ConsensusValidationResult::<DocumentTransitionAction>::new();
            if !bypass_validation {
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
                        platform_version,
                    )?;
                    result.merge(validation_result);

                    if !result.is_valid() {
                        return Ok(result);
                    }
                    let validation_result = check_updated_inside_time_window(
                        transition,
                        latest_block_time_ms,
                        average_block_spacing_ms,
                        platform_version,
                    )?;
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
            }

            let document_create_action = DocumentCreateTransitionAction::from_document_borrowed_create_transition_with_contract_lookup(document_create_transition, |identifier| {
                Ok(data_contract_fetch_info.clone())
            })?;

            if !bypass_validation {
                let validation_result = platform
                    .drive
                    .validate_document_create_transition_action_uniqueness(
                        &data_contract_fetch_info.contract,
                        document_type,
                        &document_create_action,
                        owner_id,
                        transaction,
                        platform_version,
                    )?;
                result.merge(validation_result);
            }

            if result.is_valid() {
                Ok(DocumentTransitionAction::CreateAction(document_create_action).into())
            } else {
                Ok(result)
            }
        }
        DocumentTransition::Replace(document_replace_transition) => {
            let mut result = ConsensusValidationResult::<DocumentTransitionAction>::new();
            let document_replace_action = if !bypass_validation {
                // We do not need to perform this check on genesis
                if let Some(latest_block_time_ms) = latest_block_time_ms {
                    let validation_result = check_updated_inside_time_window(
                        transition,
                        latest_block_time_ms,
                        average_block_spacing_ms,
                        platform_version,
                    )?;
                    result.merge(validation_result);

                    if !result.is_valid() {
                        return Ok(result);
                    }
                }

                let validation_result =
                    check_if_document_can_be_found(transition, fetched_documents);
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

                let validation_result = check_ownership(transition, original_document, &owner_id);
                result.merge(validation_result);

                if !result.is_valid() {
                    return Ok(result);
                }
                let document_replace_action =
                    DocumentReplaceTransitionAction::try_from_borrowed_document_replace_transition(
                        document_replace_transition,
                        original_document.created_at(),
                        |identifier| Ok(data_contract_fetch_info.clone()),
                    )?;

                let validation_result = platform
                    .drive
                    .validate_document_replace_transition_action_uniqueness(
                        &data_contract_fetch_info.contract,
                        document_type,
                        &document_replace_action,
                        owner_id,
                        transaction,
                        platform_version,
                    )?;
                result.merge(validation_result);
                document_replace_action
            } else {
                let validation_result =
                    check_if_document_can_be_found(transition, fetched_documents);
                // There is a case where we updated a just deleted document
                // In this case we don't care about the created at
                let original_document_created_at = if validation_result.is_valid() {
                    validation_result.into_data()?.created_at()
                } else {
                    None
                };

                DocumentReplaceTransitionAction::try_from_borrowed_document_replace_transition(
                    document_replace_transition,
                    original_document_created_at,
                    |identifier| Ok(data_contract_fetch_info.clone()),
                )?
            };

            if result.is_valid() {
                Ok(DocumentTransitionAction::ReplaceAction(document_replace_action).into())
            } else {
                Ok(result)
            }
        }
        DocumentTransition::Delete(document_delete_transition) => {
            let mut result = ConsensusValidationResult::<DocumentTransitionAction>::new();

            if !bypass_validation {
                let validation_result =
                    check_if_document_can_be_found(transition, fetched_documents);

                // we only do is_valid on purpose because it would be a system error if it didn't have
                // data
                let original_document = if validation_result.is_valid() {
                    validation_result.into_data()?
                } else {
                    result.add_errors(validation_result.errors);
                    return Ok(result);
                };
                let validation_result = check_ownership(transition, original_document, &owner_id);
                if !validation_result.is_valid() {
                    result.add_errors(validation_result.errors);
                }
            }

            if result.is_valid() {
                let action = DocumentDeleteTransitionAction::from_document_borrowed_create_transition_with_contract_lookup(document_delete_transition,                      |identifier| {
                    Ok(data_contract_fetch_info.clone())
                })?;
                Ok(DocumentTransitionAction::DeleteAction(action).into())
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
    if fetched_document.owner_id() != owner_id {
        result.add_error(ConsensusError::StateError(
            StateError::DocumentOwnerIdMismatchError(DocumentOwnerIdMismatchError::new(
                document_transition.base().id(),
                owner_id.to_owned(),
                fetched_document.owner_id(),
            )),
        ));
    }
    result
}

pub fn check_revision_is_bumped_by_one(
    document_transition: &DocumentReplaceTransition,
    original_document: &Document,
) -> SimpleConsensusValidationResult {
    let mut result = SimpleConsensusValidationResult::default();

    let revision = document_transition.revision();

    // If there was no previous revision this means that the document_type is not update-able
    // However this should have been caught earlier
    let Some(previous_revision) =  original_document.revision() else {
        result.add_error(ConsensusError::StateError(
            StateError::InvalidDocumentRevisionError(
                InvalidDocumentRevisionError::new(
                    document_transition.base().id(),
                    None,
                )
            )
        ));
        return result;
    };
    // no need to check bounds here, because it would be impossible to hit the end on a u64
    let expected_revision = previous_revision + 1;
    if revision != expected_revision {
        result.add_error(ConsensusError::StateError(
            StateError::InvalidDocumentRevisionError(InvalidDocumentRevisionError::new(
                document_transition.base().id(),
                Some(previous_revision),
            )),
        ))
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
        .find(|d| d.id() == document_transition.base().id());

    if maybe_fetched_document.is_some() {
        result.add_error(ConsensusError::StateError(
            StateError::DocumentAlreadyPresentError(DocumentAlreadyPresentError::new(
                document_transition.base().id(),
            )),
        ))
    }
    result
}

pub fn check_if_document_can_be_found<'a>(
    document_transition: &'a DocumentTransition,
    fetched_documents: &'a [Document],
) -> ConsensusValidationResult<&'a Document> {
    let maybe_fetched_document = fetched_documents
        .iter()
        .find(|d| d.id() == document_transition.base().id());

    if let Some(document) = maybe_fetched_document {
        ConsensusValidationResult::new_with_data(document)
    } else {
        ConsensusValidationResult::new_with_error(ConsensusError::StateError(
            StateError::DocumentNotFoundError(DocumentNotFoundError::new(
                document_transition.base().id(),
            )),
        ))
    }
}

pub fn check_if_timestamps_are_equal(
    document_transition: &DocumentTransition,
) -> SimpleConsensusValidationResult {
    let mut result = SimpleConsensusValidationResult::default();
    let created_at = document_transition.created_at();
    let updated_at = document_transition.updated_at();

    if created_at.is_some() && updated_at.is_some() && updated_at.unwrap() != created_at.unwrap() {
        result.add_error(ConsensusError::StateError(
            StateError::DocumentTimestampsMismatchError(DocumentTimestampsMismatchError::new(
                document_transition.base().id(),
            )),
        ));
    }

    result
}

pub fn check_created_inside_time_window(
    document_transition: &DocumentTransition,
    last_block_ts_millis: TimestampMillis,
    average_block_spacing_ms: u64,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    let mut result = SimpleConsensusValidationResult::default();
    let created_at = match document_transition.created_at() {
        Some(t) => t,
        None => return Ok(result),
    };

    let window_validation = validate_time_in_block_time_window(
        last_block_ts_millis,
        created_at,
        average_block_spacing_ms,
        platform_version,
    )
    .map_err(|e| Error::Protocol(ProtocolError::NonConsensusError(e)))?;
    if !window_validation.valid {
        result.add_error(ConsensusError::StateError(
            StateError::DocumentTimestampWindowViolationError(
                DocumentTimestampWindowViolationError::new(
                    String::from("createdAt"),
                    document_transition.base().id(),
                    created_at as i64,
                    window_validation.time_window_start as i64,
                    window_validation.time_window_end as i64,
                ),
            ),
        ));
    }
    Ok(result)
}

pub fn check_updated_inside_time_window(
    document_transition: &DocumentTransition,
    last_block_ts_millis: TimestampMillis,
    average_block_spacing_ms: u64,
    platform_version: &PlatformVersion,
) -> Result<SimpleConsensusValidationResult, Error> {
    let mut result = SimpleConsensusValidationResult::default();
    let updated_at = match document_transition.updated_at() {
        Some(t) => t,
        None => return Ok(result),
    };

    let window_validation = validate_time_in_block_time_window(
        last_block_ts_millis,
        updated_at,
        average_block_spacing_ms,
        platform_version,
    )
    .map_err(|e| Error::Protocol(ProtocolError::NonConsensusError(e)))?;
    if !window_validation.valid {
        result.add_error(ConsensusError::StateError(
            StateError::DocumentTimestampWindowViolationError(
                DocumentTimestampWindowViolationError::new(
                    String::from("updatedAt"),
                    document_transition.base().id(),
                    updated_at as i64,
                    window_validation.time_window_start as i64,
                    window_validation.time_window_end as i64,
                ),
            ),
        ));
    }
    Ok(result)
}
