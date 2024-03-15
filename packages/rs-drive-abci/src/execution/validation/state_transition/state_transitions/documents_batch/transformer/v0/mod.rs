use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::error::Error;
use crate::platform_types::platform::PlatformStateRef;
use dpp::consensus::basic::document::{DataContractNotPresentError, InvalidDocumentTypeError};
use dpp::consensus::basic::BasicError;

use dpp::consensus::state::document::document_not_found_error::DocumentNotFoundError;
use dpp::consensus::state::document::document_owner_id_mismatch_error::DocumentOwnerIdMismatchError;

use dpp::consensus::state::document::invalid_document_revision_error::InvalidDocumentRevisionError;
use dpp::consensus::state::state_error::StateError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;

use dpp::block::block_info::BlockInfo;
use dpp::document::{Document, DocumentV0Getters};
use dpp::validation::SimpleConsensusValidationResult;
use dpp::{consensus::ConsensusError, prelude::Identifier, validation::ConsensusValidationResult};

use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;
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

use crate::execution::validation::state_transition::documents_batch::state::v0::fetch_documents::fetch_documents_for_transitions_knowing_contract_and_document_type;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

use dpp::state_transition::documents_batch_transition::document_transition::document_replace_transition::v0::v0_methods::DocumentReplaceTransitionV0Methods;
use drive::drive::contract::DataContractFetchInfo;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;

pub(in crate::execution::validation::state_transition::state_transitions::documents_batch) trait DocumentsBatchTransitionTransformerV0
{
    fn try_into_action_v0(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        validate: bool,
        transaction: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
    ) -> Result<ConsensusValidationResult<DocumentsBatchTransitionAction>, Error>;
}

trait DocumentsBatchTransitionInternalTransformerV0 {
    fn transform_document_transitions_within_contract_v0(
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        validate: bool,
        data_contract_id: &Identifier,
        owner_id: Identifier,
        document_transitions: &BTreeMap<&String, Vec<&DocumentTransition>>,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<Vec<DocumentTransitionAction>>, Error>;
    fn transform_document_transitions_within_document_type_v0(
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        validate: bool,
        data_contract_fetch_info: Arc<DataContractFetchInfo>,
        document_type_name: &str,
        owner_id: Identifier,
        document_transitions: &[&DocumentTransition],
        _execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<Vec<DocumentTransitionAction>>, Error>;
    /// The data contract can be of multiple difference versions
    fn transform_transition_v0(
        validate: bool,
        block_info: &BlockInfo,
        data_contract_fetch_info: Arc<DataContractFetchInfo>,
        transition: &DocumentTransition,
        replaced_documents: &[Document],
        owner_id: Identifier,
    ) -> Result<ConsensusValidationResult<DocumentTransitionAction>, Error>;
    fn find_replaced_document_v0<'a>(
        document_transition: &'a DocumentTransition,
        fetched_documents: &'a [Document],
    ) -> ConsensusValidationResult<&'a Document>;
    fn check_ownership_of_old_replaced_document_v0(
        document_transition: &DocumentReplaceTransition,
        fetched_document: &Document,
        owner_id: &Identifier,
    ) -> SimpleConsensusValidationResult;
    fn check_revision_is_bumped_by_one_during_replace_v0(
        document_transition: &DocumentReplaceTransition,
        original_document: &Document,
    ) -> SimpleConsensusValidationResult;
}

impl DocumentsBatchTransitionTransformerV0 for DocumentsBatchTransition {
    fn try_into_action_v0(
        &self,
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        validate_against_state: bool,
        transaction: TransactionArg,
        execution_context: &mut StateTransitionExecutionContext,
    ) -> Result<ConsensusValidationResult<DocumentsBatchTransitionAction>, Error> {
        let owner_id = self.owner_id();
        let user_fee_increase = self.user_fee_increase();
        let platform_version = platform.state.current_platform_version()?;
        let mut transitions_by_contracts_and_types: BTreeMap<
            &Identifier,
            BTreeMap<&String, Vec<&DocumentTransition>>,
        > = BTreeMap::new();

        // We want to validate by contract, and then for each document type within a contract
        for document_transition in self.transitions().iter() {
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
                    Self::transform_document_transitions_within_contract_v0(
                        platform,
                        block_info,
                        validate_against_state,
                        data_contract_id,
                        owner_id,
                        document_transitions_by_document_type,
                        execution_context,
                        transaction,
                        platform_version,
                    )
                },
            )
            .collect::<Result<Vec<ConsensusValidationResult<Vec<DocumentTransitionAction>>>, Error>>()?;
        let validation_result = ConsensusValidationResult::flatten(validation_result);

        if validation_result.is_valid() {
            let batch_transition_action = DocumentsBatchTransitionActionV0 {
                owner_id,
                transitions: validation_result.into_data()?,
                user_fee_increase,
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
}

impl DocumentsBatchTransitionInternalTransformerV0 for DocumentsBatchTransition {
    fn transform_document_transitions_within_contract_v0(
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        validate_against_state: bool,
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
            .get_contract_with_fetch_info_and_fee(
                data_contract_id.0 .0,
                None,
                false,
                transaction,
                platform_version,
            )?
            .1
        else {
            return Ok(ConsensusValidationResult::new_with_error(
                BasicError::DataContractNotPresentError(DataContractNotPresentError::new(
                    *data_contract_id,
                ))
                .into(),
            ));
        };

        let validation_result = document_transitions
        .iter()
        .map(|(document_type_name, document_transitions)| {
            Self::transform_document_transitions_within_document_type_v0(
                platform,
                block_info,
                validate_against_state,
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

    fn transform_document_transitions_within_document_type_v0(
        platform: &PlatformStateRef,
        block_info: &BlockInfo,
        validate_against_state: bool,
        data_contract_fetch_info: Arc<DataContractFetchInfo>,
        document_type_name: &str,
        owner_id: Identifier,
        document_transitions: &[&DocumentTransition],
        _execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<Vec<DocumentTransitionAction>>, Error> {
        // We use temporary execution context without dry run,
        // because despite the dryRun, we need to get the
        // data contract to proceed with following logic
        // let tmp_execution_context = StateTransitionExecutionContext::default_for_platform_version(platform_version)?;
        //
        // execution_context.add_operations(tmp_execution_context.operations_slice());

        let dry_run = false; //maybe reenable

        let data_contract = &data_contract_fetch_info.contract;

        let Some(document_type) = data_contract.document_type_optional_for_name(document_type_name)
        else {
            return Ok(ConsensusValidationResult::new_with_error(
                InvalidDocumentTypeError::new(document_type_name.to_owned(), data_contract.id())
                    .into(),
            ));
        };

        let replace_transitions = document_transitions
            .iter()
            .filter(|transition| matches!(transition, DocumentTransition::Replace(_)))
            .copied()
            .collect::<Vec<_>>();

        // We fetch documents only for replace transitions
        // since we need them to create transition actions
        // Below we also perform state validation for replace transitions only
        // other transitions are validated in their validate_state functions
        // TODO: Think more about this architecture
        let fetched_documents_validation_result =
            fetch_documents_for_transitions_knowing_contract_and_document_type(
                platform.drive,
                data_contract,
                document_type,
                replace_transitions.as_slice(),
                transaction,
                platform_version,
            )?;

        if !fetched_documents_validation_result.is_valid() {
            return Ok(ConsensusValidationResult::new_with_errors(
                fetched_documents_validation_result.errors,
            ));
        }

        let replaced_documents = fetched_documents_validation_result.into_data()?;

        let document_transition_actions_result = if !dry_run {
            let document_transition_actions_validation_result = document_transitions
                .iter()
                .map(|transition| {
                    // we validate every transition in this document type
                    Self::transform_transition_v0(
                        validate_against_state,
                        block_info,
                        data_contract_fetch_info.clone(),
                        transition,
                        &replaced_documents,
                        owner_id,
                    )
                })
                .collect::<Result<Vec<ConsensusValidationResult<DocumentTransitionAction>>, Error>>(
                )?;

            let result = ConsensusValidationResult::merge_many(
                document_transition_actions_validation_result,
            );

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
    fn transform_transition_v0<'a>(
        validate_against_state: bool,
        block_info: &BlockInfo,
        data_contract_fetch_info: Arc<DataContractFetchInfo>,
        transition: &DocumentTransition,
        replaced_documents: &[Document],
        owner_id: Identifier,
    ) -> Result<ConsensusValidationResult<DocumentTransitionAction>, Error> {
        match transition {
            DocumentTransition::Create(document_create_transition) => {
                let result = ConsensusValidationResult::<DocumentTransitionAction>::new();

                let document_create_action = DocumentCreateTransitionAction::from_document_borrowed_create_transition_with_contract_lookup(document_create_transition, block_info, |_identifier| {
                Ok(data_contract_fetch_info.clone())
            })?;

                if result.is_valid() {
                    Ok(DocumentTransitionAction::CreateAction(document_create_action).into())
                } else {
                    Ok(result)
                }
            }
            DocumentTransition::Replace(document_replace_transition) => {
                let mut result = ConsensusValidationResult::<DocumentTransitionAction>::new();

                let validation_result =
                    Self::find_replaced_document_v0(transition, replaced_documents);

                if !validation_result.is_valid_with_data() {
                    result.merge(validation_result);
                    return Ok(result);
                }

                let original_document = validation_result.into_data()?;

                // There is a case where we updated a just deleted document
                // In this case we don't care about the created at
                let original_document_created_at = original_document.created_at();

                let validation_result = Self::check_ownership_of_old_replaced_document_v0(
                    document_replace_transition,
                    original_document,
                    &owner_id,
                );

                if !validation_result.is_valid() {
                    result.merge(validation_result);
                    return Ok(result);
                }

                if validate_against_state {
                    //there are situations where we don't want to validate this against the state
                    // for example when we already applied the state transition action
                    // and we are just validating it happened
                    let validation_result = Self::check_revision_is_bumped_by_one_during_replace_v0(
                        document_replace_transition,
                        original_document,
                    );

                    if !validation_result.is_valid() {
                        result.merge(validation_result);
                        return Ok(result);
                    }
                }

                let document_replace_action =
                    DocumentReplaceTransitionAction::try_from_borrowed_document_replace_transition(
                        document_replace_transition,
                        original_document_created_at,
                        block_info.time_ms,
                        |_identifier| Ok(data_contract_fetch_info.clone()),
                    )?;

                if result.is_valid() {
                    Ok(DocumentTransitionAction::ReplaceAction(document_replace_action).into())
                } else {
                    Ok(result)
                }
            }
            DocumentTransition::Delete(document_delete_transition) => {
                let action = DocumentDeleteTransitionAction::from_document_borrowed_create_transition_with_contract_lookup(document_delete_transition,                      |_identifier| {
                Ok(data_contract_fetch_info.clone())
            })?;
                Ok(DocumentTransitionAction::DeleteAction(action).into())
            }
        }
    }

    fn find_replaced_document_v0<'a>(
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

    fn check_ownership_of_old_replaced_document_v0(
        document_transition: &DocumentReplaceTransition,
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
    fn check_revision_is_bumped_by_one_during_replace_v0(
        document_transition: &DocumentReplaceTransition,
        original_document: &Document,
    ) -> SimpleConsensusValidationResult {
        let mut result = SimpleConsensusValidationResult::default();

        let revision = document_transition.revision();

        // If there was no previous revision this means that the document_type is not update-able
        // However this should have been caught earlier
        let Some(previous_revision) = original_document.revision() else {
            result.add_error(ConsensusError::StateError(
                StateError::InvalidDocumentRevisionError(InvalidDocumentRevisionError::new(
                    document_transition.base().id(),
                    None,
                    revision,
                )),
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
                    revision,
                )),
            ))
        }
        result
    }
}
